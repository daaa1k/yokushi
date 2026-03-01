/// Configuration types for yokushi suppression rules.
///
/// Loaded from YAML; config discovery order:
/// 1. --config CLI flag
/// 2. YOKUSHI_CONFIG env var
/// 3. ./yokushi.yaml
/// 4. ~/.config/yokushi/config.yaml
use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::output::OutputMode;

/// A single suppression rule.
#[derive(Debug, Deserialize)]
pub struct Rule {
    /// Tool name to match (default: "Bash").
    pub tool: Option<String>,
    /// Field in tool_input to match against (default: auto-detected per tool).
    pub field: Option<String>,
    /// Regex pattern to match the field value.
    pub pattern: String,
    /// Human-readable reason shown when the rule blocks.
    pub message: String,
}

impl Rule {
    /// Returns the effective tool name (defaults to "Bash").
    pub fn effective_tool(&self) -> &str {
        self.tool.as_deref().unwrap_or("Bash")
    }

    /// Returns the effective field name, auto-detected from the tool when not specified.
    pub fn effective_field(&self) -> &str {
        if let Some(f) = &self.field {
            return f.as_str();
        }
        default_field(self.effective_tool())
    }
}

/// Per-agent output configuration.
#[derive(Debug, Deserialize)]
pub struct AgentConfig {
    pub output: OutputMode,
}

/// Root configuration structure.
#[derive(Debug, Deserialize)]
pub struct Config {
    /// Agent-specific output behavior keyed by agent name.
    #[serde(default)]
    pub agents: HashMap<String, AgentConfig>,
    /// Ordered list of suppression rules.
    #[serde(default)]
    pub rules: Vec<Rule>,
}

impl Config {
    /// Returns the output mode for the given agent name.
    /// Falls back to the "default" agent config, then to `OutputMode::Exit`.
    pub fn output_mode_for(&self, agent: &str) -> OutputMode {
        if let Some(cfg) = self.agents.get(agent) {
            return cfg.output;
        }
        if let Some(cfg) = self.agents.get("default") {
            return cfg.output;
        }
        OutputMode::Exit
    }
}

/// Returns the default tool_input field name for a given tool.
pub fn default_field(tool: &str) -> &str {
    match tool {
        "Bash" => "command",
        "Write" | "Edit" | "Read" | "Glob" | "Grep" => "file_path",
        "WebFetch" => "url",
        "WebSearch" => "query",
        "Task" => "prompt",
        _ => "command",
    }
}

/// Loads config from the given path.
pub fn load_from_path(path: &PathBuf) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    serde_yml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))
}

/// Discovers and loads the config file using the standard search order.
/// Returns `None` if no config file is found (allow-all mode).
pub fn discover(explicit_path: Option<&PathBuf>) -> Result<Option<Config>> {
    // 1. Explicit --config flag
    if let Some(path) = explicit_path {
        return load_from_path(path).map(Some);
    }

    // 2. YOKUSHI_CONFIG environment variable
    if let Ok(env_path) = std::env::var("YOKUSHI_CONFIG") {
        return load_from_path(&PathBuf::from(env_path)).map(Some);
    }

    // 3. ./yokushi.yaml
    let local = PathBuf::from("yokushi.yaml");
    if local.exists() {
        return load_from_path(&local).map(Some);
    }

    // 4. ~/.config/yokushi/config.yaml
    if let Some(home) = dirs_next() {
        let global = home.join(".config/yokushi/config.yaml");
        if global.exists() {
            return load_from_path(&global).map(Some);
        }
    }

    Ok(None)
}

/// Returns the home directory path.
fn dirs_next() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_field() {
        assert_eq!(default_field("Bash"), "command");
        assert_eq!(default_field("Write"), "file_path");
        assert_eq!(default_field("Edit"), "file_path");
        assert_eq!(default_field("Read"), "file_path");
        assert_eq!(default_field("WebFetch"), "url");
        assert_eq!(default_field("WebSearch"), "query");
        assert_eq!(default_field("Task"), "prompt");
        assert_eq!(default_field("Unknown"), "command");
    }

    #[test]
    fn test_rule_defaults() {
        let rule = Rule {
            tool: None,
            field: None,
            pattern: "git push".into(),
            message: "blocked".into(),
        };
        assert_eq!(rule.effective_tool(), "Bash");
        assert_eq!(rule.effective_field(), "command");
    }

    #[test]
    fn test_rule_with_tool() {
        let rule = Rule {
            tool: Some("Write".into()),
            field: None,
            pattern: "\\.env$".into(),
            message: "blocked".into(),
        };
        assert_eq!(rule.effective_tool(), "Write");
        assert_eq!(rule.effective_field(), "file_path");
    }

    #[test]
    fn test_parse_config_yaml() {
        let yaml = r#"
version: "1"
agents:
  claude-code:
    output: json
  default:
    output: exit
rules:
  - pattern: "git push"
    message: "Direct git push is prohibited."
  - tool: "Write"
    pattern: "\\.env$"
    message: "Writing to .env is prohibited."
"#;
        let config: Config = serde_yml::from_str(yaml).unwrap();
        assert_eq!(config.rules.len(), 2);
        assert_eq!(config.rules[0].effective_tool(), "Bash");
        assert_eq!(config.rules[1].effective_tool(), "Write");
        assert!(matches!(
            config.output_mode_for("claude-code"),
            OutputMode::Json
        ));
        assert!(matches!(
            config.output_mode_for("default"),
            OutputMode::Exit
        ));
        assert!(matches!(
            config.output_mode_for("unknown"),
            OutputMode::Exit
        ));
    }
}
