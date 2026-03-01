/// Agent-specific output strategies for block/allow decisions.
///
/// - `Json`: Outputs hookSpecificOutput JSON to stdout with exit 0 (Claude Code format).
/// - `Exit`: Prints reason to stderr and exits with code 2 (works with most agents).
use serde::Deserialize;

/// Output mode for a given agent.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    /// Claude Code official format: JSON to stdout + exit 0.
    Json,
    /// Generic format: message to stderr + exit 2.
    Exit,
}

/// Emits a block decision in Claude Code JSON format and exits with 0.
///
/// Stdout:
/// ```json
/// {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny","permissionDecisionReason":"..."}}
/// ```
pub fn block_json(event_name: &str, reason: &str) -> ! {
    let output = serde_json::json!({
        "hookSpecificOutput": {
            "hookEventName": event_name,
            "permissionDecision": "deny",
            "permissionDecisionReason": reason
        }
    });
    println!("{}", output);
    std::process::exit(0);
}

/// Emits a block decision to stderr and exits with 2.
pub fn block_exit(reason: &str) -> ! {
    eprintln!("yokushi: {}", reason);
    std::process::exit(2);
}

/// Exits silently with 0 (allow).
pub fn allow() -> ! {
    std::process::exit(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_mode_deserialize_json() {
        let mode: OutputMode = serde_yml::from_str("json").unwrap();
        assert_eq!(mode, OutputMode::Json);
    }

    #[test]
    fn test_output_mode_deserialize_exit() {
        let mode: OutputMode = serde_yml::from_str("exit").unwrap();
        assert_eq!(mode, OutputMode::Exit);
    }
}
