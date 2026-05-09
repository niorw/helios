use std::collections::HashMap;

pub fn replace_variables(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (key, value) in vars {
        let pattern = format!("{}{}{}", "{", "{", key);
        let pattern = format!("{}{}", pattern, "}}");
        result = result.replace(&pattern, value);

        let pattern2 = format!("{{{{ {key} }}}}");
        result = result.replace(&pattern2, value);
    }
    result
}

pub fn format_json(json_str: &str) -> String {
    match serde_json::from_str::<serde_json::Value>(json_str) {
        Ok(v) => serde_json::to_string_pretty(&v).unwrap_or_else(|_| json_str.to_string()),
        Err(_) => json_str.to_string(),
    }
}

pub fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() > max_len {
        format!("{}...", &s[..s.char_indices().nth(max_len).map(|(i, _)| i).unwrap_or(s.len())])
    } else {
        s.to_string()
    }
}

pub fn copy_to_clipboard(text: &str) -> anyhow::Result<()> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let mut child = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("Failed to spawn pbcopy: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(text.as_bytes())
            .map_err(|e| anyhow::anyhow!("Failed to write to pbcopy: {}", e))?;
    }

    let _ = child.wait();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replace_variables_simple() {
        let mut vars = HashMap::new();
        vars.insert("base_url".to_string(), "https://api.example.com".to_string());

        let result = replace_variables("{{base_url}}/users", &vars);
        assert_eq!(result, "https://api.example.com/users");
    }

    #[test]
    fn test_replace_variables_multiple() {
        let mut vars = HashMap::new();
        vars.insert("host".to_string(), "api.example.com".to_string());
        vars.insert("version".to_string(), "v1".to_string());

        let result = replace_variables("https://{{host}}/{{version}}/users", &vars);
        assert_eq!(result, "https://api.example.com/v1/users");
    }

    #[test]
    fn test_replace_variables_no_match() {
        let vars = HashMap::new();
        let result = replace_variables("{{base_url}}/users", &vars);
        assert_eq!(result, "{{base_url}}/users");
    }

    #[test]
    fn test_replace_variables_with_spaces() {
        let mut vars = HashMap::new();
        vars.insert("api_key".to_string(), "secret123".to_string());

        let result = replace_variables("Authorization: {{ api_key }}", &vars);
        assert_eq!(result, "Authorization: secret123");
    }

    #[test]
    fn test_format_json_valid() {
        let input = r#"{"name":"test","value":123}"#;
        let result = format_json(input);
        assert!(result.contains("{\n"));
        assert!(result.contains("\"name\":"));
        assert!(result.contains("\"test\""));
    }

    #[test]
    fn test_format_json_invalid() {
        let input = "not valid json";
        let result = format_json(input);
        assert_eq!(result, "not valid json");
    }

    #[test]
    fn test_truncate_short_string() {
        let result = truncate("hello", 10);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        let result = truncate("this is a very long string", 10);
        assert_eq!(result, "this is a ...");
    }

    #[test]
    fn test_truncate_exact_length() {
        let result = truncate("exactlyten", 10);
        assert_eq!(result, "exactlyten");
    }
}
