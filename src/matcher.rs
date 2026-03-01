/// Rule matching logic against hook inputs.
///
/// Patterns are compiled as regex. If a pattern is invalid regex,
/// it falls back to literal substring matching.
use regex::Regex;

use crate::config::Rule;
use crate::hook::HookInput;

/// Result of matching a hook input against the rule set.
pub struct MatchResult<'a> {
    pub rule: &'a Rule,
    pub matched_value: String,
}

/// Matches the hook input against the ordered rule list.
/// Returns the first matching rule, or `None` if all rules pass.
pub fn find_match<'a>(rules: &'a [Rule], input: &HookInput) -> Option<MatchResult<'a>> {
    let tool = input.effective_tool();

    for rule in rules {
        if !tool_matches(rule.effective_tool(), tool) {
            continue;
        }

        let field = rule.effective_field();
        let value = match input.get_field(field) {
            Some(v) => v,
            None => continue,
        };

        if pattern_matches(&rule.pattern, value) {
            return Some(MatchResult {
                rule,
                matched_value: value.to_owned(),
            });
        }
    }

    None
}

/// Checks whether the rule's tool name matches the input tool name (case-insensitive).
fn tool_matches(rule_tool: &str, input_tool: &str) -> bool {
    rule_tool.eq_ignore_ascii_case(input_tool)
}

/// Tests a pattern against a value.
/// Attempts regex first; falls back to literal substring match on invalid regex.
fn pattern_matches(pattern: &str, value: &str) -> bool {
    match Regex::new(pattern) {
        Ok(re) => re.is_match(value),
        Err(_) => value.contains(pattern),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Rule;
    use crate::hook::HookInput;

    fn bash_input(command: &str) -> HookInput {
        let json = serde_json::json!({
            "tool_name": "Bash",
            "tool_input": { "command": command }
        });
        serde_json::from_value(json).unwrap()
    }

    fn write_input(path: &str) -> HookInput {
        let json = serde_json::json!({
            "tool_name": "Write",
            "tool_input": { "file_path": path }
        });
        serde_json::from_value(json).unwrap()
    }

    fn rule(tool: Option<&str>, pattern: &str) -> Rule {
        Rule {
            tool: tool.map(str::to_owned),
            field: None,
            pattern: pattern.to_owned(),
            message: "blocked".to_owned(),
        }
    }

    #[test]
    fn test_bash_regex_match() {
        let rules = vec![rule(None, r"git push")];
        let input = bash_input("git push origin main");
        assert!(find_match(&rules, &input).is_some());
    }

    #[test]
    fn test_bash_no_match() {
        let rules = vec![rule(None, r"git push")];
        let input = bash_input("cargo build");
        assert!(find_match(&rules, &input).is_none());
    }

    #[test]
    fn test_write_tool_match() {
        let rules = vec![rule(Some("Write"), r"\.env$")];
        let input = write_input("/project/.env");
        assert!(find_match(&rules, &input).is_some());
    }

    #[test]
    fn test_write_tool_no_match() {
        let rules = vec![rule(Some("Write"), r"\.env$")];
        let input = write_input("/project/main.rs");
        assert!(find_match(&rules, &input).is_none());
    }

    #[test]
    fn test_tool_mismatch_skipped() {
        // Rule targets Write, input is Bash — should not match
        let rules = vec![rule(Some("Write"), r".*")];
        let input = bash_input("anything");
        assert!(find_match(&rules, &input).is_none());
    }

    #[test]
    fn test_first_match_wins() {
        let rules = vec![
            rule(None, r"git push"),
            rule(None, r".*"), // would match everything
        ];
        let input = bash_input("git push origin main");
        let result = find_match(&rules, &input).unwrap();
        assert_eq!(result.rule.pattern, "git push");
    }

    #[test]
    fn test_invalid_regex_falls_back_to_literal() {
        // "[unclosed" is invalid regex; should fall back to literal match
        let rules = vec![rule(None, "[unclosed")];
        let input = bash_input("echo [unclosed");
        assert!(find_match(&rules, &input).is_some());
    }

    #[test]
    fn test_invalid_regex_literal_no_match() {
        let rules = vec![rule(None, "[unclosed")];
        let input = bash_input("echo hello");
        assert!(find_match(&rules, &input).is_none());
    }

    #[test]
    fn test_word_boundary_regex() {
        let rules = vec![rule(None, r"\bawk\b")];
        assert!(find_match(&rules, &bash_input("awk '{print}'")).is_some());
        assert!(find_match(&rules, &bash_input("echo awkward")).is_none());
    }

    #[test]
    fn test_empty_rules() {
        let rules: Vec<Rule> = vec![];
        let input = bash_input("rm -rf /");
        assert!(find_match(&rules, &input).is_none());
    }
}
