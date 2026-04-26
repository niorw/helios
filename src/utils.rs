use std::collections::HashMap;

pub fn replace_variables(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (key, value) in vars {
        let pattern = format!("{}{}{}", "{{", "{", key);
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
