use anyhow::{Context, Result};
use crate::config::EnvConfig;

pub fn run(port: u16, upstream: Option<&str>, verbose: u8) -> Result<()> {
    let env = EnvConfig::load();
    let upstream_url = upstream
        .map(String::from)
        .or(env.base_url.clone())
        .context("No upstream URL. Set ANTHROPIC_BASE_URL or use --upstream")?;

    let rt = tokio::runtime::Runtime::new()
        .context("Failed to create tokio runtime")?;

    rt.block_on(async {
        crate::proxy::server::run_proxy(port, &upstream_url, verbose).await
    })
}
