/// yokushi - AI agent PreToolUse hook validator.
///
/// Reads hook JSON from stdin, matches against YAML-configured suppression rules,
/// and outputs a block decision in the format appropriate for the specified agent.
///
/// Usage:
///   yokushi [--config <file>] [--agent <name>]
///
/// Exit codes:
///   0 - Allow (or JSON-format block for claude-code)
///   2 - Block (stderr message for default agents)
mod config;
mod hook;
mod matcher;
mod output;

use std::io::{self, Read};
use std::path::PathBuf;

use anyhow::Result;
use clap::Parser;

use output::OutputMode;

/// AI agent hook tool that suppresses specific tool/command usage based on YAML rules.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Path to YAML config file.
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Agent type for output format.
    #[arg(short, long, default_value = "default", value_name = "NAME")]
    agent: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Load configuration (allow-all if no config found)
    let config = config::discover(cli.config.as_ref())?;

    // Read hook JSON from stdin
    let mut stdin_buf = String::new();
    io::stdin().read_to_string(&mut stdin_buf)?;

    // Parse hook input; on parse failure allow through (don't block legitimate tool use)
    let input: hook::HookInput = match serde_json::from_str(&stdin_buf) {
        Ok(v) => v,
        Err(_) => output::allow(),
    };

    // Determine output mode for this agent
    let mode = match &config {
        Some(cfg) => cfg.output_mode_for(&cli.agent),
        None => OutputMode::Exit,
    };

    // If no config, allow all
    let cfg = match config {
        Some(c) => c,
        None => output::allow(),
    };

    // Try to find a matching suppression rule
    let event_name = input.hook_event_name.as_deref().unwrap_or("PreToolUse");

    if let Some(m) = matcher::find_match(&cfg.rules, &input) {
        let reason = format!(
            "blocked by rule '{}' (matched: {:?}): {}",
            m.rule.pattern, m.matched_value, m.rule.message
        );
        match mode {
            OutputMode::Json => output::block_json(event_name, &reason),
            OutputMode::Exit => output::block_exit(&reason),
        }
    } else {
        output::allow()
    }
}
