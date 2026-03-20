use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use bytes::Bytes;
use http_body_util::{BodyExt, Full, combinators::BoxBody};
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use tokio::net::TcpListener;

use super::analyzer;
use crate::config::EnvConfig;
use crate::diagnosis::provider::{self, ProviderType};

struct ProxyState {
    upstream: String,
    provider_type: ProviderType,
    verbose: u8,
}

pub async fn run_proxy(port: u16, upstream: &str, verbose: u8) -> Result<()> {
    let env = EnvConfig::load();
    let provider_type = provider::detect(&env);

    let state = Arc::new(ProxyState {
        upstream: upstream.trim_end_matches('/').to_string(),
        provider_type,
        verbose,
    });

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = TcpListener::bind(addr).await
        .with_context(|| format!("Failed to bind to port {}", port))?;

    eprintln!();
    eprintln!("\x1b[1mCorvus Sniff\x1b[0m — Listening on :{} → {}", port, upstream);
    eprintln!("\x1b[2m═══════════════════════════════════════════════════════════\x1b[0m");
    eprintln!();
    eprintln!("  Set this to use the proxy:");
    eprintln!("  \x1b[36mexport ANTHROPIC_BASE_URL=http://localhost:{}\x1b[0m", port);
    eprintln!();

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
) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
    let start = Instant::now();
    let method = req.method().clone();
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
    let _api_key = req.headers().get("authorization")
        .or_else(|| req.headers().get("x-api-key"))
        .and_then(|v| v.to_str().ok())
        .map(String::from);
    let _content_type = req.headers().get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    // Collect all headers for forwarding
    let orig_headers: Vec<(String, String)> = req.headers().iter()
        .filter(|(name, _)| {
            let n = name.as_str();
            // Don't forward hop-by-hop or host headers
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
        method: method.to_string(),
        path: path.clone(),
        model,
        message_count: msg_count,
        tool_count,
        anthropic_version,
        beta_flags,
        is_streaming,
        warnings: Vec::new(),
    };

    // Check for issues
    analysis.warnings = analyzer::check_request(&analysis, &state.provider_type);

    // Print request log
    let timestamp = chrono::Local::now().format("%H:%M:%S").to_string();
    print!("{}", analyzer::format_request_log(&analysis, &timestamp));

    // Forward request to upstream
    let upstream_url = format!("{}{}", state.upstream, path);

    // Build the forwarding request using ureq (blocking in spawn_blocking to not block tokio)
    let body_vec = body_bytes.to_vec();
    let method_str = method.to_string();
    let headers_clone = orig_headers.clone();
    let verbose = state.verbose;

    let upstream_result = tokio::task::spawn_blocking(move || {
        forward_request(&upstream_url, &method_str, &headers_clone, &body_vec, verbose)
    }).await;

    let elapsed = start.elapsed().as_millis();

    match upstream_result {
        Ok(Ok((status, resp_headers, resp_body))) => {
            // Try to extract output tokens from response
            let output_tokens = extract_output_tokens(&resp_body);
            let error_message = if status >= 400 {
                extract_error_message(&resp_body)
            } else {
                None
            };

            let resp_analysis = analyzer::ResponseAnalysis {
                status,
                duration_ms: elapsed,
                output_tokens,
                error_message,
            };
            print!("{}", analyzer::format_response_log(&resp_analysis));

            // Build response
            let mut builder = Response::builder().status(status);
            for (name, value) in &resp_headers {
                builder = builder.header(name.as_str(), value.as_str());
            }
            let body = Full::new(Bytes::from(resp_body))
                .map_err(|never| match never {})
                .boxed();
            Ok(builder.body(body).unwrap())
        }
        Ok(Err(e)) => {
            eprintln!("  \x1b[31m→ Upstream error ({}ms): {}\x1b[0m", elapsed, e);
            Ok(error_response(502, &format!("Upstream error: {}", e)))
        }
        Err(e) => {
            eprintln!("  \x1b[31m→ Internal error: {}\x1b[0m", e);
            Ok(error_response(500, "Internal proxy error"))
        }
    }
}

/// Forward request using ureq (blocking). Returns (status, headers, body).
#[allow(clippy::type_complexity)]
fn forward_request(
    url: &str,
    method: &str,
    headers: &[(String, String)],
    body: &[u8],
    _verbose: u8,
) -> Result<(u16, Vec<(String, String)>, Vec<u8>)> {
    let mut req = match method {
        "POST" => ureq::post(url),
        "GET" => ureq::get(url),
        "PUT" => ureq::put(url),
        "DELETE" => ureq::delete(url),
        "PATCH" => ureq::patch(url),
        _ => ureq::request(method, url),
    };

    for (name, value) in headers {
        req = req.set(name, value);
    }

    let result = if method == "GET" || method == "DELETE" || body.is_empty() {
        req.call()
    } else {
        req.send_bytes(body)
    };

    match result {
        Ok(resp) => {
            let status = resp.status();
            let resp_headers: Vec<(String, String)> = resp.headers_names().iter()
                .filter_map(|name| {
                    resp.header(name).map(|val| (name.clone(), val.to_string()))
                })
                .collect();

            let mut body_buf = Vec::new();
            use std::io::Read;
            resp.into_reader().read_to_end(&mut body_buf)
                .context("Failed to read upstream response body")?;

            Ok((status, resp_headers, body_buf))
        }
        Err(ureq::Error::Status(status, resp)) => {
            let resp_headers: Vec<(String, String)> = resp.headers_names().iter()
                .filter_map(|name| {
                    resp.header(name).map(|val| (name.clone(), val.to_string()))
                })
                .collect();

            let mut body_buf = Vec::new();
            use std::io::Read;
            resp.into_reader().read_to_end(&mut body_buf)
                .context("Failed to read upstream error body")?;

            Ok((status, resp_headers, body_buf))
        }
        Err(e) => {
            anyhow::bail!("Upstream request failed: {}", e)
        }
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

fn error_response(status: u16, msg: &str) -> Response<BoxBody<Bytes, hyper::Error>> {
    let body = serde_json::json!({
        "error": {
            "type": "proxy_error",
            "message": msg
        }
    });
    let body_bytes = serde_json::to_vec(&body).unwrap_or_default();
    let body = Full::new(Bytes::from(body_bytes))
        .map_err(|never| match never {})
        .boxed();
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .body(body)
        .unwrap()
}
