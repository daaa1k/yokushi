/// Hook protocol types for PreToolUse stdin/stdout communication.
///
/// Input JSON structure (from Claude Code / OpenCode):
/// ```json
/// {
///   "session_id": "...",
///   "hook_event_name": "PreToolUse",
///   "tool_name": "Bash",
///   "tool_input": { "command": "git push origin main" }
/// }
/// ```
use serde::Deserialize;

/// Parsed hook input from stdin.
#[derive(Debug, Deserialize, Default)]
pub struct HookInput {
    pub hook_event_name: Option<String>,
    pub tool_name: Option<String>,
    /// Raw tool_input object; field access is done dynamically per rule.
    pub tool_input: Option<serde_json::Value>,
}

impl HookInput {
    /// Returns the tool name, defaulting to "Bash".
    pub fn effective_tool(&self) -> &str {
        self.tool_name.as_deref().unwrap_or("Bash")
    }

    /// Extracts the string value of `field` from tool_input.
    /// Returns `None` if tool_input is absent or the field is not a string.
    pub fn get_field(&self, field: &str) -> Option<&str> {
        self.tool_input.as_ref()?.get(field)?.as_str()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_bash_input() {
        let json = r#"{"tool_name":"Bash","hook_event_name":"PreToolUse","tool_input":{"command":"git push"}}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.effective_tool(), "Bash");
        assert_eq!(input.get_field("command"), Some("git push"));
    }

    #[test]
    fn test_parse_write_input() {
        let json = r#"{"tool_name":"Write","tool_input":{"file_path":"/project/.env","content":"secret"}}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.effective_tool(), "Write");
        assert_eq!(input.get_field("file_path"), Some("/project/.env"));
        assert_eq!(input.get_field("command"), None);
    }

    #[test]
    fn test_missing_tool_name_defaults_to_bash() {
        let json = r#"{"tool_input":{"command":"ls"}}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.effective_tool(), "Bash");
    }

    #[test]
    fn test_empty_input() {
        let json = r#"{}"#;
        let input: HookInput = serde_json::from_str(json).unwrap();
        assert_eq!(input.effective_tool(), "Bash");
        assert_eq!(input.get_field("command"), None);
    }
}
