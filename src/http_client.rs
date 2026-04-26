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
