use crate::models::{Auth, BodyType, HttpMethod, KeyValue, Request, Response};
use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::time::Instant;

pub async fn send_request(req: &Request) -> Result<Response> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()?;

    let method = match req.method {
        HttpMethod::GET => reqwest::Method::GET,
        HttpMethod::POST => reqwest::Method::POST,
        HttpMethod::PUT => reqwest::Method::PUT,
        HttpMethod::DELETE => reqwest::Method::DELETE,
        HttpMethod::PATCH => reqwest::Method::PATCH,
        HttpMethod::HEAD => reqwest::Method::HEAD,
        HttpMethod::OPTIONS => reqwest::Method::OPTIONS,
    };

    let mut url = req.url.clone();
    let active_params: Vec<_> = req
        .params
        .iter()
        .filter(|p| p.enabled && !p.key.is_empty())
        .map(|p| (p.key.clone(), p.value.clone()))
        .collect();

    if !active_params.is_empty() {
        let query = serde_urlencoded::to_string(&active_params)?;
        url = if url.contains('?') {
            format!("{}&{}", url, query)
        } else {
            format!("{}?{}", url, query)
        };
    }

    let mut headers = HeaderMap::new();
    for h in &req.headers {
        if h.enabled && !h.key.is_empty() {
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(h.key.as_bytes()),
                HeaderValue::from_str(&h.value),
            ) {
                headers.insert(name, value);
            }
        }
    }

    let start = Instant::now();
    let mut builder = client.request(method, &url).headers(headers);

    match req.body_type {
        BodyType::Json => {
            builder = builder.header("Content-Type", "application/json");
            if !req.body.is_empty() {
                builder = builder.body(req.body.clone());
            }
        }
        BodyType::Text => {
            builder = builder.header("Content-Type", "text/plain");
            if !req.body.is_empty() {
                builder = builder.body(req.body.clone());
            }
        }
        BodyType::Form => {
            builder = builder.header("Content-Type", "application/x-www-form-urlencoded");
            if !req.body.is_empty() {
                builder = builder.body(req.body.clone());
            }
        }
        BodyType::Xml => {
            builder = builder.header("Content-Type", "application/xml");
            if !req.body.is_empty() {
                builder = builder.body(req.body.clone());
            }
        }
        BodyType::None => {}
    }

    match &req.auth {
        Auth::Bearer { token } => {
            builder = builder.bearer_auth(token);
        }
        Auth::Basic { username, password } => {
            builder = builder.basic_auth(username, Some(password));
        }
        Auth::None => {}
    }

    let raw = builder.send().await?;
    let status = raw.status();
    let status_text = status.canonical_reason().unwrap_or("Unknown").to_string();
    let status_u16 = status.as_u16();

    let resp_headers: HashMap<String, String> = raw
        .headers()
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    let body = raw.text().await.unwrap_or_default();

    Ok(Response {
        status: status_u16,
        status_text,
        headers: resp_headers,
        body,
        duration_ms: start.elapsed().as_millis() as u64,
        timestamp: chrono::Local::now(),
    })
}

pub fn parse_headers(raw: &[String]) -> Vec<KeyValue> {
    raw.iter()
        .filter_map(|s| {
            let mut parts = s.splitn(2, ':');
            let key = parts.next()?.trim().to_string();
            let value = parts.next()?.trim().to_string();
            Some(KeyValue {
                key,
                value,
                enabled: true,
            })
        })
        .collect()
}

/// 对请求执行变量替换（环境变量 + 内置变量）
/// 在发送请求前调用，替换 URL、Headers、Body 中的 {{var}} 占位符
pub fn resolve_request_variables(
    req: &Request,
    env_vars: &std::collections::HashMap<String, String>,
) -> Request {
    let mut resolved = req.clone();
    resolved.url = crate::utils::replace_variables(&resolved.url, env_vars);
    resolved.url = crate::utils::resolve_builtin_variables(&resolved.url);
    for h in &mut resolved.headers {
        h.value = crate::utils::replace_variables(&h.value, env_vars);
        h.value = crate::utils::resolve_builtin_variables(&h.value);
    }
    resolved.body = crate::utils::replace_variables(&resolved.body, env_vars);
    resolved.body = crate::utils::resolve_builtin_variables(&resolved.body);
    resolved
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_headers_simple() {
        let raw = vec!["Content-Type: application/json".to_string()];
        let headers = parse_headers(&raw);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].key, "Content-Type");
        assert_eq!(headers[0].value, "application/json");
        assert!(headers[0].enabled);
    }

    #[test]
    fn test_parse_headers_multiple() {
        let raw = vec![
            "Content-Type: application/json".to_string(),
            "Authorization: Bearer token123".to_string(),
            "X-Custom-Header: value".to_string(),
        ];
        let headers = parse_headers(&raw);
        assert_eq!(headers.len(), 3);
        assert_eq!(headers[1].key, "Authorization");
        assert_eq!(headers[1].value, "Bearer token123");
    }

    #[test]
    fn test_parse_headers_empty() {
        let raw: Vec<String> = vec![];
        let headers = parse_headers(&raw);
        assert!(headers.is_empty());
    }

    #[test]
    fn test_parse_headers_with_spaces() {
        let raw = vec!["  Content-Type  :  application/json  ".to_string()];
        let headers = parse_headers(&raw);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].key, "Content-Type");
        assert_eq!(headers[0].value, "application/json");
    }

    #[test]
    fn test_parse_headers_value_with_colon() {
        let raw = vec!["Authorization: Basic: dXNlcjpwYXNz".to_string()];
        let headers = parse_headers(&raw);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].key, "Authorization");
        assert_eq!(headers[0].value, "Basic: dXNlcjpwYXNz");
    }

    #[test]
    fn test_parse_headers_missing_colon() {
        let raw = vec!["InvalidHeader".to_string()];
        let headers = parse_headers(&raw);
        assert!(headers.is_empty());
    }

    #[test]
    fn test_resolve_url_env_variable() {
        let mut req = Request::default();
        req.url = "{{base_url}}/users".to_string();
        let mut vars = std::collections::HashMap::new();
        vars.insert("base_url".to_string(), "https://api.example.com".to_string());
        let resolved = resolve_request_variables(&req, &vars);
        assert_eq!(resolved.url, "https://api.example.com/users");
    }

    #[test]
    fn test_resolve_header_variable() {
        let mut req = Request::default();
        req.headers = vec![KeyValue { key: "Auth".into(), value: "Bearer {{token}}".into(), enabled: true }];
        let mut vars = std::collections::HashMap::new();
        vars.insert("token".to_string(), "abc123".to_string());
        let resolved = resolve_request_variables(&req, &vars);
        assert_eq!(resolved.headers[0].value, "Bearer abc123");
    }

    #[test]
    fn test_resolve_body_variable() {
        let mut req = Request::default();
        req.body = r#"{"user":"{{name}}"}"#.to_string();
        let mut vars = std::collections::HashMap::new();
        vars.insert("name".to_string(), "admin".to_string());
        let resolved = resolve_request_variables(&req, &vars);
        assert_eq!(resolved.body, r#"{"user":"admin"}"#);
    }

    #[test]
    fn test_resolve_builtin_uuid() {
        let mut req = Request::default();
        req.url = "https://api.example.com/{{$uuid}}".to_string();
        let vars = std::collections::HashMap::new();
        let resolved = resolve_request_variables(&req, &vars);
        assert!(!resolved.url.contains("{{$uuid}}"));
        assert!(resolved.url.starts_with("https://api.example.com/"));
    }

    #[test]
    fn test_parse_headers_empty_value() {
        let raw = vec!["X-Empty-Value:".to_string()];
        let headers = parse_headers(&raw);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].key, "X-Empty-Value");
        assert_eq!(headers[0].value, "");
    }
}
