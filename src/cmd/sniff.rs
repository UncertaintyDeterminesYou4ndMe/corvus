use anyhow::{Context, Result};
use crate::config::EnvConfig;

pub fn run(port: u16, upstream: Option<&str>, verbose: u8, launch: &[String]) -> Result<()> {
    let env = EnvConfig::load();
    let upstream_url = upstream
        .map(String::from)
        .or(env.base_url.clone())
        .context("No upstream URL. Set ANTHROPIC_BASE_URL or pass --upstream <url>")?;

    let rt = tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?;

    if launch.is_empty() {
        // Classic mode: start proxy and print the export hint, run forever.
        rt.block_on(async {
            crate::proxy::server::run_proxy(port, &upstream_url, verbose).await
        })
    } else {
        // Launcher mode: bind first (confirms port is free), then spawn the child.
        let listener = rt.block_on(crate::proxy::server::bind_listener(port))?;

        // Start the proxy accept-loop in a background thread.
        let upstream_bg = upstream_url.clone();
        std::thread::spawn(move || {
            let rt2 = tokio::runtime::Runtime::new().expect("tokio runtime");
            rt2.block_on(async {
                crate::proxy::server::run_proxy_with_listener(listener, &upstream_bg, verbose)
                    .await
                    .ok();
            });
        });

        // Print banner (the background thread hasn't printed it yet — print here instead).
        eprintln!();
        eprintln!(
            "\x1b[1mCorvus Sniff\x1b[0m — Listening on :{} → {}",
            port, upstream_url
        );
        eprintln!("\x1b[2m═══════════════════════════════════════════════════════════\x1b[0m");
        eprintln!();
        eprintln!(
            "  Launching: \x1b[36m{}\x1b[0m",
            launch.join(" ")
        );
        eprintln!(
            "  with \x1b[36mANTHROPIC_BASE_URL=http://localhost:{}\x1b[0m",
            port
        );
        eprintln!();

        // Launch the child process with the proxy env var injected.
        let exit_status = std::process::Command::new(&launch[0])
            .args(&launch[1..])
            .env("ANTHROPIC_BASE_URL", format!("http://localhost:{}", port))
            .status()
            .with_context(|| format!("Failed to launch '{}'", launch[0]))?;

        std::process::exit(exit_status.code().unwrap_or(1));
    }
}
