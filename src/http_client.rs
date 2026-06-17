use crate::models::{Auth, BodyType, HttpMethod, KeyValue, Request, Response};
use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

pub async fn send_request(req: &Request, cookie_jar: Option<Arc<reqwest::cookie::Jar>>) -> Result<Response> {
    let mut builder = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30));

    if let Some(jar) = cookie_jar {
        builder = builder.cookie_provider(jar);
    }

    let client = builder.build()?;

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
        BodyType::Graphql => {
            builder = builder.header("Content-Type", "application/json");
            if !req.body.is_empty() {
                builder = builder.body(req.body.clone());
            }
        }
        BodyType::FormData => {
            // multipart/form-data 由 reqwest multipart 处理
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
    fn test_parse_headers_empty_value() {
        let raw = vec!["X-Empty-Value:".to_string()];
        let headers = parse_headers(&raw);
        assert_eq!(headers.len(), 1);
        assert_eq!(headers[0].key, "X-Empty-Value");
        assert_eq!(headers[0].value, "");
    }

    #[tokio::test]
    async fn test_send_request_with_cookie_jar() {
        let jar = Arc::new(reqwest::cookie::Jar::default());
        let req = Request::new("Test", HttpMethod::GET, "http://127.0.0.1:1");
        let result = send_request(&req, Some(jar)).await;
        // Connection refused is expected - we're testing the API accepts the jar
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_send_request_without_cookie_jar() {
        let req = Request::new("Test", HttpMethod::GET, "http://127.0.0.1:1");
        let result = send_request(&req, None).await;
        // Connection refused is expected - we're testing None still works
        assert!(result.is_err());
    }
}
