use anyhow::Result;
use crate::config::ClaudeConfig;
use crate::diagnosis;
use crate::display::{self, OutputFormat};

pub fn run(skip_network: bool, verbose: u8, format: &OutputFormat) -> Result<()> {
    let config = ClaudeConfig::load()?;
    let report = diagnosis::run_all_checks(&config, skip_network, verbose)?;
    display::print_health_report(&report, format)?;
    Ok(())
}
