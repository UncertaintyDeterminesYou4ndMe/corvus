use anyhow::Result;
use crate::config::{self, ClaudeConfig};
use crate::display::{self, OutputFormat};

pub fn run(daily: bool, by_model: bool, verbose: u8, format: &OutputFormat) -> Result<()> {
    let _config = ClaudeConfig::load()?;
    let stats = config::stats_cache::load_stats()?;
    let usage = config::stats_cache::load_usage_info()?;
    display::print_stats_report(&stats, &usage, daily, by_model, verbose, format)?;
    Ok(())
}
