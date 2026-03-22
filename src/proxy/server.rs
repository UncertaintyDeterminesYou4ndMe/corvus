use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use bytes::Bytes;
use futures_util::StreamExt;
use http_body_util::{BodyExt, Full, StreamBody, combinators::BoxBody};
use hyper::body::{Frame, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use tokio::net::TcpListener;

use super::analyzer;
use crate::config::EnvConfig;
use crate::diagnosis::provider::{self, ProviderType};

type RespBody = BoxBody<Bytes, Infallible>;

struct ProxyState {
    upstream: String,
    provider_type: ProviderType,
    verbose: u8,
    client: reqwest::Client,
}

pub async fn run_proxy(port: u16, upstream: &str, verbose: u8) -> Result<()> {
    let listener = bind_listener(port).await?;
    // Print the manual-use banner (launcher mode prints its own banner).
    eprintln!();
    eprintln!("\x1b[1mCorvus Sniff\x1b[0m — Listening on :{} → {}", port, upstream);
    eprintln!("\x1b[2m═══════════════════════════════════════════════════════════\x1b[0m");
    eprintln!();
    eprintln!("  Set this to use the proxy:");
    eprintln!("  \x1b[36mexport ANTHROPIC_BASE_URL=http://localhost:{}\x1b[0m", port);
    eprintln!();
    run_proxy_with_listener(listener, upstream, verbose).await
}

/// Bind the listener first (so callers can confirm the port is ready before spawning subprocesses).
pub async fn bind_listener(port: u16) -> Result<TcpListener> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    TcpListener::bind(addr).await
        .with_context(|| format!("Failed to bind to port {port} — is it already in use?"))
}

/// Accept-loop only — no banner printed. Used by launcher mode (banner printed by caller).
pub async fn run_proxy_with_listener(listener: TcpListener, upstream: &str, verbose: u8) -> Result<()> {
    let env = EnvConfig::load();
    let provider_type = provider::detect(&env);

    let state = Arc::new(ProxyState {
        upstream: upstream.trim_end_matches('/').to_string(),
        provider_type,
        verbose,
        client: reqwest::Client::builder()
            .user_agent("corvus-sniff/0.1")
            .build()
            .context("Failed to build HTTP client")?,
    });

    loop {
        let (stream, _remote) = listener.accept().await?;
        let state = Arc::clone(&state);

        tokio::spawn(async move {
            let io = hyper_util::rt::TokioIo::new(stream);
            let svc = service_fn(move |req| {
                let state = Arc::clone(&state);
                async move { handle_request(req, &state).await }
            });
            if let Err(e) = http1::Builder::new()
                .serve_connection(io, svc)
                .with_upgrades()
                .await
            {
                if !e.is_incomplete_message() {
                    eprintln!("  \x1b[31mConnection error: {}\x1b[0m", e);
                }
            }
        });
    }
}

async fn handle_request(
    req: Request<Incoming>,
    state: &ProxyState,
) -> Result<Response<RespBody>, hyper::Error> {
    let start = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path_and_query()
        .map(|pq| pq.to_string())
        .unwrap_or_else(|| "/".to_string());

    // Extract headers before consuming body
    let anthropic_version = req.headers().get("anthropic-version")
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let beta_header = req.headers().get("anthropic-beta")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Collect all headers for forwarding (filter hop-by-hop)
    let orig_headers: Vec<(String, String)> = req.headers().iter()
        .filter(|(name, _)| {
            let n = name.as_str();
            n != "host" && n != "connection" && n != "transfer-encoding"
        })
        .filter_map(|(name, val)| {
            val.to_str().ok().map(|v| (name.to_string(), v.to_string()))
        })
        .collect();

    // Read the request body
    let body_bytes = match req.into_body().collect().await {
        Ok(collected) => collected.to_bytes(),
        Err(e) => {
            eprintln!("  \x1b[31mFailed to read request body: {}\x1b[0m", e);
            return Ok(error_response(502, "Failed to read request body"));
        }
    };

    // Analyze the request
    let (model, msg_count, tool_count, is_streaming) =
        analyzer::analyze_request_body(&body_bytes);
    let beta_flags = beta_header.as_deref()
        .map(analyzer::parse_beta_flags)
        .unwrap_or_default();

    let mut analysis = analyzer::RequestAnalysis {
        method: method.clone(),
        path: path.clone(),
        model,
        message_count: msg_count,
        tool_count,
        anthropic_version,
        beta_flags,
        is_streaming,
        warnings: Vec::new(),
    };
    analysis.warnings = analyzer::check_request(&analysis, &state.provider_type);

    // Print request log
    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
    print!("{}", analyzer::format_request_log(&analysis, &timestamp));
    if state.verbose >= 1 {
        print!("{}", analyzer::format_body_dump("request body", &body_bytes));
    }

    // Build upstream URL and reqwest request
    let upstream_url = format!("{}{}", state.upstream, path);
    let req_method = method.parse::<reqwest::Method>()
        .unwrap_or(reqwest::Method::POST);

    let mut req_builder = state.client.request(req_method, &upstream_url);
    for (name, value) in &orig_headers {
        req_builder = req_builder.header(name.as_str(), value.as_str());
    }
    if !body_bytes.is_empty() {
        req_builder = req_builder.body(body_bytes.to_vec());
    }

    // Send to upstream
    let upstream_resp = match req_builder.send().await {
        Ok(r) => r,
        Err(e) => {
            let elapsed = start.elapsed().as_millis();
            eprintln!("  \x1b[31m→ Upstream error ({}ms): {}\x1b[0m", elapsed, e);
            return Ok(error_response(502, &format!("Upstream error: {}", e)));
        }
    };

    let status = upstream_resp.status().as_u16();

    // Build response headers
    let mut builder = Response::builder().status(status);
    for (name, val) in upstream_resp.headers() {
        if let Ok(v) = val.to_str() {
            builder = builder.header(name.as_str(), v);
        }
    }

    if is_streaming && status < 400 {
        // ── Streaming path: pipe bytes to client as they arrive ──────────
        let elapsed = start.elapsed().as_millis();
        print!("{}", analyzer::format_response_log(&analyzer::ResponseAnalysis {
            status,
            duration_ms: elapsed,
            output_tokens: None,
            error_message: None,
            is_streaming: true,
        }));

        let verbose = state.verbose;
        let byte_stream = upstream_resp.bytes_stream().map(move |chunk| {
            let bytes = chunk.unwrap_or_default();
            // With -vv, print each SSE line as it arrives
            if verbose >= 2 && !bytes.is_empty() {
                if let Ok(text) = std::str::from_utf8(&bytes) {
                    for line in text.lines() {
                        if !line.is_empty() {
                            eprintln!("  \x1b[2m│ {}\x1b[0m", line);
                        }
                    }
                }
            }
            Ok::<_, Infallible>(Frame::data(bytes))
        });

        let body = BodyExt::boxed(StreamBody::new(byte_stream));
        Ok(builder.body(body).unwrap())
    } else {
        // ── Buffered path: collect entire response, then analyze ──────────
        let resp_bytes = match upstream_resp.bytes().await {
            Ok(b) => b,
            Err(e) => {
                eprintln!("  \x1b[31m→ Failed to read response body: {}\x1b[0m", e);
                return Ok(error_response(502, "Failed to read upstream response"));
            }
        };

        let elapsed = start.elapsed().as_millis();
        let output_tokens = extract_output_tokens(&resp_bytes);
        let error_message = if status >= 400 { extract_error_message(&resp_bytes) } else { None };

        print!("{}", analyzer::format_response_log(&analyzer::ResponseAnalysis {
            status,
            duration_ms: elapsed,
            output_tokens,
            error_message,
            is_streaming: false,
        }));

        if state.verbose >= 2 {
            print!("{}", analyzer::format_body_dump("response body", &resp_bytes));
        }

        let body = Full::new(resp_bytes)
            .map_err(|_: Infallible| unreachable!())
            .boxed();
        Ok(builder.body(body).unwrap())
    }
}

fn extract_output_tokens(body: &[u8]) -> Option<u64> {
    let val: serde_json::Value = serde_json::from_slice(body).ok()?;
    val.get("usage")
        .and_then(|u| u.get("output_tokens"))
        .and_then(|t| t.as_u64())
}

fn extract_error_message(body: &[u8]) -> Option<String> {
    let val: serde_json::Value = serde_json::from_slice(body).ok()?;
    val.get("error")
        .and_then(|e| e.get("message"))
        .and_then(|m| m.as_str())
        .map(String::from)
        .or_else(|| {
            val.get("message").and_then(|m| m.as_str()).map(String::from)
        })
}

fn error_response(status: u16, msg: &str) -> Response<RespBody> {
    let body = serde_json::json!({
        "error": { "type": "proxy_error", "message": msg }
    });
    let body_bytes = Bytes::from(serde_json::to_vec(&body).unwrap_or_default());
    let body = Full::new(body_bytes)
        .map_err(|_: Infallible| unreachable!())
        .boxed();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(body)
        .unwrap()
}
