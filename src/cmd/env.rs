use anyhow::Result;
use crate::config::ClaudeConfig;
use crate::display::{self, OutputFormat};

pub fn run(show_secrets: bool, verbose: u8, format: &OutputFormat) -> Result<()> {
    let config = ClaudeConfig::load()?;
    display::print_env_summary(&config, show_secrets, verbose, format)?;
    Ok(())
}
