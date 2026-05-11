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

pub fn status_code_description(code: u16) -> &'static str {
    match code {
        // 1xx Informational
        100 => "Continue",
        101 => "Switching Protocols",
        // 2xx Success
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        // 3xx Redirection
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        // 4xx Client Errors
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        // 5xx Server Errors
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        // Unknown
        _ => "Unknown",
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

    // --- status_code_description tests ---

    #[test]
    fn test_status_100_continue() {
        assert_eq!(status_code_description(100), "Continue");
    }

    #[test]
    fn test_status_101_switching_protocols() {
        assert_eq!(status_code_description(101), "Switching Protocols");
    }

    #[test]
    fn test_status_200_ok() {
        assert_eq!(status_code_description(200), "OK");
    }

    #[test]
    fn test_status_201_created() {
        assert_eq!(status_code_description(201), "Created");
    }

    #[test]
    fn test_status_204_no_content() {
        assert_eq!(status_code_description(204), "No Content");
    }

    #[test]
    fn test_status_301_moved_permanently() {
        assert_eq!(status_code_description(301), "Moved Permanently");
    }

    #[test]
    fn test_status_302_found() {
        assert_eq!(status_code_description(302), "Found");
    }

    #[test]
    fn test_status_304_not_modified() {
        assert_eq!(status_code_description(304), "Not Modified");
    }

    #[test]
    fn test_status_400_bad_request() {
        assert_eq!(status_code_description(400), "Bad Request");
    }

    #[test]
    fn test_status_401_unauthorized() {
        assert_eq!(status_code_description(401), "Unauthorized");
    }

    #[test]
    fn test_status_403_forbidden() {
        assert_eq!(status_code_description(403), "Forbidden");
    }

    #[test]
    fn test_status_404_not_found() {
        assert_eq!(status_code_description(404), "Not Found");
    }

    #[test]
    fn test_status_405_method_not_allowed() {
        assert_eq!(status_code_description(405), "Method Not Allowed");
    }

    #[test]
    fn test_status_408_request_timeout() {
        assert_eq!(status_code_description(408), "Request Timeout");
    }

    #[test]
    fn test_status_409_conflict() {
        assert_eq!(status_code_description(409), "Conflict");
    }

    #[test]
    fn test_status_410_gone() {
        assert_eq!(status_code_description(410), "Gone");
    }

    #[test]
    fn test_status_422_unprocessable_entity() {
        assert_eq!(status_code_description(422), "Unprocessable Entity");
    }

    #[test]
    fn test_status_429_too_many_requests() {
        assert_eq!(status_code_description(429), "Too Many Requests");
    }

    #[test]
    fn test_status_500_internal_server_error() {
        assert_eq!(status_code_description(500), "Internal Server Error");
    }

    #[test]
    fn test_status_502_bad_gateway() {
        assert_eq!(status_code_description(502), "Bad Gateway");
    }

    #[test]
    fn test_status_503_service_unavailable() {
        assert_eq!(status_code_description(503), "Service Unavailable");
    }

    #[test]
    fn test_status_504_gateway_timeout() {
        assert_eq!(status_code_description(504), "Gateway Timeout");
    }

    #[test]
    fn test_status_unknown_code() {
        assert_eq!(status_code_description(418), "Unknown");
    }

    #[test]
    fn test_status_unknown_high_code() {
        assert_eq!(status_code_description(999), "Unknown");
    }
}
