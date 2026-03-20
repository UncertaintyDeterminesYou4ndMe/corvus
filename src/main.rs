#![allow(clippy::uninlined_format_args)]
#![allow(clippy::useless_format)]
mod cmd;
mod config;
mod diagnosis;
mod display;
mod errors;
mod known_models;

#[cfg(feature = "sniff")]
mod proxy;

use anyhow::Result;
use clap::{CommandFactory, Parser, Subcommand};
use display::OutputFormat;

#[derive(Parser)]
#[command(
    name = "corvus",
    version,
    about = "Corvus - Claude Code diagnostic CLI",
    long_about = "Diagnose Claude Code configurations, validate model names against provider \
                  capabilities, analyze API requests, and track usage statistics.\n\n\
                  Named after the crow genus — the smartest birds, known for solving problems."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Verbosity level (-v, -vv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Output format
    #[arg(long, default_value = "text", global = true)]
    format: OutputFormat,
}

#[derive(Subcommand)]
enum Commands {
    /// Diagnose Claude Code configuration and detect common issues
    Check {
        /// Skip URL reachability test
        #[arg(long)]
        skip_network: bool,
    },

    /// Show all Claude Code related environment variables and config
    Env {
        /// Show sensitive values unmasked
        #[arg(long)]
        show_secrets: bool,
    },

    /// Query provider's /v1/models endpoint and cross-reference with configuration
    Models {
        /// Override the base URL to query
        #[arg(long)]
        url: Option<String>,

        /// Override the API key
        #[arg(long)]
        key: Option<String>,
    },

    /// Show Claude Code usage statistics and cost estimates
    Stats {
        /// Show daily breakdown
        #[arg(short, long)]
        daily: bool,

        /// Show model-level token breakdown
        #[arg(short, long)]
        by_model: bool,
    },

    /// Generate shell completions (bash, zsh, fish, powershell)
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: ShellType,
    },

    /// Start local proxy to intercept and analyze Claude Code API requests
    #[cfg(feature = "sniff")]
    Sniff {
        /// Local port to listen on
        #[arg(short, long, default_value = "8080")]
        port: u16,

        /// Upstream URL to forward requests to (default: reads ANTHROPIC_BASE_URL)
        #[arg(long)]
        upstream: Option<String>,
    },
}

#[derive(Clone, clap::ValueEnum)]
enum ShellType {
    Bash,
    Zsh,
    Fish,
    Powershell,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Check { skip_network } => {
            cmd::check::run(skip_network, cli.verbose, &cli.format)?;
        }
        Commands::Env { show_secrets } => {
            cmd::env::run(show_secrets, cli.verbose, &cli.format)?;
        }
        Commands::Models { url, key } => {
            cmd::models::run(url.as_deref(), key.as_deref(), cli.verbose, &cli.format)?;
        }
        Commands::Stats { daily, by_model } => {
            cmd::stats::run(daily, by_model, cli.verbose, &cli.format)?;
        }
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            let shell = match shell {
                ShellType::Bash => clap_complete::Shell::Bash,
                ShellType::Zsh => clap_complete::Shell::Zsh,
                ShellType::Fish => clap_complete::Shell::Fish,
                ShellType::Powershell => clap_complete::Shell::PowerShell,
            };
            clap_complete::generate(shell, &mut cmd, "corvus", &mut std::io::stdout());
        }
        #[cfg(feature = "sniff")]
        Commands::Sniff { port, upstream } => {
            cmd::sniff::run(port, upstream.as_deref(), cli.verbose)?;
        }
    }

    Ok(())
}
