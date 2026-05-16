use crate::models::{BodyType, Collection, KeyValue, Request};
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct HarFile {
    log: HarLog,
}

#[derive(Debug, Deserialize)]
struct HarLog {
    entries: Vec<HarEntry>,
}

#[derive(Debug, Deserialize)]
struct HarEntry {
    request: HarRequest,
}

#[derive(Debug, Deserialize)]
struct HarRequest {
    method: String,
    url: String,
    #[serde(default)]
    headers: Vec<HarHeader>,
    #[serde(rename = "queryString", default)]
    query_string: Vec<HarQueryString>,
    #[serde(rename = "postData", default)]
    post_data: Option<HarPostData>,
}

#[derive(Debug, Deserialize)]
struct HarHeader {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct HarQueryString {
    name: String,
    value: String,
}

#[derive(Debug, Deserialize)]
struct HarPostData {
    #[serde(rename = "mimeType", default)]
    mime_type: Option<String>,
    #[serde(default)]
    text: Option<String>,
}

fn parse_method(s: &str) -> crate::models::HttpMethod {
    match s.to_uppercase().as_str() {
        "POST" => crate::models::HttpMethod::POST,
        "PUT" => crate::models::HttpMethod::PUT,
        "DELETE" => crate::models::HttpMethod::DELETE,
        "PATCH" => crate::models::HttpMethod::PATCH,
        "HEAD" => crate::models::HttpMethod::HEAD,
        "OPTIONS" => crate::models::HttpMethod::OPTIONS,
        _ => crate::models::HttpMethod::GET,
    }
}

fn mime_to_body_type(mime: &str) -> BodyType {
    if mime.contains("json") {
        BodyType::Json
    } else if mime.contains("x-www-form-urlencoded") {
        BodyType::Form
    } else if mime.contains("xml") {
        BodyType::Xml
    } else if mime.contains("text") {
        BodyType::Text
    } else {
        BodyType::None
    }
}

pub fn parse_har(json_str: &str) -> Result<Collection> {
    let har: HarFile =
        serde_json::from_str(json_str).context("Failed to parse HAR file")?;

    let requests: Vec<Request> = har
        .log
        .entries
        .into_iter()
        .map(|entry| {
            let method = parse_method(&entry.request.method);

            let headers: Vec<KeyValue> = entry
                .request
                .headers
                .into_iter()
                .map(|h| KeyValue {
                    key: h.name,
                    value: h.value,
                    enabled: true,
                })
                .collect();

            let params: Vec<KeyValue> = entry
                .request
                .query_string
                .into_iter()
                .map(|qs| KeyValue {
                    key: qs.name,
                    value: qs.value,
                    enabled: true,
                })
                .collect();

            let (body_type, body) = if let Some(pd) = entry.request.post_data {
                let bt = pd
                    .mime_type
                    .as_deref()
                    .map(mime_to_body_type)
                    .unwrap_or(BodyType::None);
                (bt, pd.text.unwrap_or_default())
            } else {
                (BodyType::None, String::new())
            };

            // Use URL as name, stripping query string for readability
            let name = entry
                .request
                .url
                .split('?')
                .next()
                .unwrap_or(&entry.request.url)
                .to_string();

            Request {
                id: uuid::Uuid::new_v4().to_string(),
                name,
                method,
                url: entry.request.url,
                headers,
                params,
                body,
                body_type,
                auth: crate::models::Auth::None,
                ..Default::default()
            }
        })
        .collect();

    Ok(Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: "HAR Import".to_string(),
        requests,
        created_at: chrono::Local::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_har_simple_get() {
        let har_json = r#"{
            "log": {
                "entries": [
                    {
                        "request": {
                            "method": "GET",
                            "url": "https://api.example.com/users?page=1",
                            "headers": [
                                {"name": "Accept", "value": "application/json"}
                            ],
                            "queryString": [
                                {"name": "page", "value": "1"}
                            ]
                        }
                    }
                ]
            }
        }"#;

        let col = parse_har(har_json).unwrap();
        assert_eq!(col.name, "HAR Import");
        assert_eq!(col.requests.len(), 1);

        let req = &col.requests[0];
        assert_eq!(req.method, crate::models::HttpMethod::GET);
        assert_eq!(req.url, "https://api.example.com/users?page=1");
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.headers[0].key, "Accept");
        assert_eq!(req.params.len(), 1);
        assert_eq!(req.params[0].key, "page");
        assert_eq!(req.params[0].value, "1");
    }

    #[test]
    fn test_parse_har_post_with_body() {
        let har_json = r#"{
            "log": {
                "entries": [
                    {
                        "request": {
                            "method": "POST",
                            "url": "https://api.example.com/users",
                            "headers": [
                                {"name": "Content-Type", "value": "application/json"}
                            ],
                            "queryString": [],
                            "postData": {
                                "mimeType": "application/json",
                                "text": "{\"name\":\"Alice\"}"
                            }
                        }
                    }
                ]
            }
        }"#;

        let col = parse_har(har_json).unwrap();
        let req = &col.requests[0];
        assert_eq!(req.method, crate::models::HttpMethod::POST);
        assert_eq!(req.body_type, BodyType::Json);
        assert_eq!(req.body, r#"{"name":"Alice"}"#);
    }

    #[test]
    fn test_parse_har_multiple_entries() {
        let har_json = r#"{
            "log": {
                "entries": [
                    {
                        "request": {
                            "method": "GET",
                            "url": "https://api.example.com/users"
                        }
                    },
                    {
                        "request": {
                            "method": "DELETE",
                            "url": "https://api.example.com/users/1"
                        }
                    },
                    {
                        "request": {
                            "method": "PUT",
                            "url": "https://api.example.com/users/1",
                            "postData": {
                                "mimeType": "text/plain",
                                "text": "updated"
                            }
                        }
                    }
                ]
            }
        }"#;

        let col = parse_har(har_json).unwrap();
        assert_eq!(col.requests.len(), 3);
        assert_eq!(col.requests[0].method, crate::models::HttpMethod::GET);
        assert_eq!(col.requests[1].method, crate::models::HttpMethod::DELETE);
        assert_eq!(col.requests[2].method, crate::models::HttpMethod::PUT);
        assert_eq!(col.requests[2].body_type, BodyType::Text);
        assert_eq!(col.requests[2].body, "updated");
    }
}
