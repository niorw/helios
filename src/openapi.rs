use crate::models::{Auth, BodyType, Collection, HttpMethod, KeyValue, Request};
use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct OpenApiSpec {
    info: OpenApiInfo,
    #[serde(default)]
    paths: std::collections::HashMap<String, OpenApiPathItem>,
    #[serde(default)]
    servers: Vec<OpenApiServer>,
}

#[derive(Debug, Deserialize)]
struct OpenApiInfo {
    title: String,
}

#[derive(Debug, Deserialize)]
struct OpenApiServer {
    url: String,
}

#[derive(Debug, Deserialize, Default)]
struct OpenApiPathItem {
    #[serde(default)]
    get: Option<OpenApiOperation>,
    #[serde(default)]
    post: Option<OpenApiOperation>,
    #[serde(default)]
    put: Option<OpenApiOperation>,
    #[serde(default)]
    delete: Option<OpenApiOperation>,
    #[serde(default)]
    patch: Option<OpenApiOperation>,
    #[serde(default)]
    head: Option<OpenApiOperation>,
    #[serde(default)]
    options: Option<OpenApiOperation>,
}

#[derive(Debug, Deserialize, Default)]
struct OpenApiOperation {
    #[serde(default)]
    summary: Option<String>,
    #[serde(default, rename = "operationId")]
    operation_id: Option<String>,
    #[serde(default)]
    parameters: Vec<OpenApiParameter>,
    #[serde(default, rename = "requestBody")]
    request_body: Option<OpenApiRequestBody>,
}

#[derive(Debug, Deserialize)]
struct OpenApiParameter {
    name: String,
    #[serde(rename = "in")]
    location: String,
    required: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct OpenApiRequestBody {
    #[serde(default)]
    content: std::collections::HashMap<String, OpenApiMediaType>,
}

#[derive(Debug, Deserialize)]
struct OpenApiMediaType {
    schema: Option<serde_json::Value>,
}

pub fn parse_openapi(json_str: &str) -> Result<Collection> {
    let spec: OpenApiSpec = serde_json::from_str(json_str)
        .context("Failed to parse OpenAPI 3.0 specification")?;

    let base_url = spec
        .servers
        .first()
        .map(|s| s.url.trim_end_matches('/').to_string())
        .unwrap_or_default();

    let mut requests: Vec<Request> = Vec::new();

    let methods: [(fn(&OpenApiPathItem) -> &Option<OpenApiOperation>, HttpMethod); 7] = [
        (|p: &OpenApiPathItem| &p.get, HttpMethod::GET),
        (|p: &OpenApiPathItem| &p.post, HttpMethod::POST),
        (|p: &OpenApiPathItem| &p.put, HttpMethod::PUT),
        (|p: &OpenApiPathItem| &p.delete, HttpMethod::DELETE),
        (|p: &OpenApiPathItem| &p.patch, HttpMethod::PATCH),
        (|p: &OpenApiPathItem| &p.head, HttpMethod::HEAD),
        (|p: &OpenApiPathItem| &p.options, HttpMethod::OPTIONS),
    ];

    for (path, path_item) in &spec.paths {
        for (accessor, method) in &methods {
            if let Some(op) = accessor(path_item) {
                let url = format!("{}{}", base_url, path);

                let name = op
                    .operation_id
                    .clone()
                    .or_else(|| op.summary.clone())
                    .unwrap_or_else(|| format!("{} {}", method, path));

                let headers: Vec<KeyValue> = op
                    .parameters
                    .iter()
                    .filter(|p| p.location == "header")
                    .map(|p| KeyValue {
                        key: p.name.clone(),
                        value: String::new(),
                        enabled: true,
                    })
                    .collect();

                let params: Vec<KeyValue> = op
                    .parameters
                    .iter()
                    .filter(|p| p.location == "query")
                    .map(|p| KeyValue {
                        key: p.name.clone(),
                        value: String::new(),
                        enabled: true,
                    })
                    .collect();

                let (body_type, body) = if let Some(rb) = &op.request_body {
                    if rb.content.contains_key("application/json") {
                        let sample = rb
                            .content
                            .get("application/json")
                            .and_then(|m| m.schema.as_ref())
                            .map(|s| serde_json::to_string_pretty(s).unwrap_or_default())
                            .unwrap_or_default();
                        (BodyType::Json, sample)
                    } else if rb.content.contains_key("application/x-www-form-urlencoded") {
                        (BodyType::Form, String::new())
                    } else {
                        (BodyType::Text, String::new())
                    }
                } else {
                    (BodyType::None, String::new())
                };

                requests.push(Request {
                    id: uuid::Uuid::new_v4().to_string(),
                    name,
                    method: method.clone(),
                    url,
                    headers,
                    params,
                    body,
                    body_type,
                    auth: Auth::None,
                    ..Default::default()
                });
            }
        }
    }

    Ok(Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: spec.info.title,
        requests,
        created_at: chrono::Local::now(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_get_request() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": { "title": "Test API", "version": "1.0.0" },
            "paths": {
                "/users": {
                    "get": {
                        "summary": "List users",
                        "operationId": "listUsers"
                    }
                }
            }
        }"#;

        let col = parse_openapi(json).unwrap();
        assert_eq!(col.name, "Test API");
        assert_eq!(col.requests.len(), 1);
        let req = &col.requests[0];
        assert_eq!(req.method, HttpMethod::GET);
        assert_eq!(req.url, "/users");
        assert_eq!(req.name, "listUsers");
    }

    #[test]
    fn test_parse_multiple_methods_and_paths() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": { "title": "Multi API", "version": "1.0.0" },
            "paths": {
                "/users": {
                    "get": { "summary": "List" },
                    "post": { "summary": "Create" }
                },
                "/users/{id}": {
                    "get": { "summary": "Get user" },
                    "delete": { "summary": "Delete user" }
                }
            }
        }"#;

        let col = parse_openapi(json).unwrap();
        assert_eq!(col.name, "Multi API");
        assert_eq!(col.requests.len(), 4);

        let methods: Vec<&HttpMethod> = col.requests.iter().map(|r| &r.method).collect();
        assert!(methods.contains(&&HttpMethod::GET));
        assert!(methods.contains(&&HttpMethod::POST));
        assert!(methods.contains(&&HttpMethod::DELETE));
    }

    #[test]
    fn test_parse_request_body_and_headers() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": { "title": "Body API", "version": "1.0.0" },
            "paths": {
                "/items": {
                    "post": {
                        "summary": "Create item",
                        "parameters": [
                            { "name": "X-Custom", "in": "header" },
                            { "name": "page", "in": "query" }
                        ],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": { "type": "object", "properties": { "name": { "type": "string" } } }
                                }
                            }
                        }
                    }
                }
            }
        }"#;

        let col = parse_openapi(json).unwrap();
        let req = &col.requests[0];
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.body_type, BodyType::Json);
        assert!(!req.body.is_empty());
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.headers[0].key, "X-Custom");
        assert_eq!(req.params.len(), 1);
        assert_eq!(req.params[0].key, "page");
    }

    #[test]
    fn test_parse_with_server_base_url() {
        let json = r#"{
            "openapi": "3.0.0",
            "info": { "title": "Server API", "version": "1.0.0" },
            "servers": [ { "url": "https://api.example.com/v1" } ],
            "paths": {
                "/health": {
                    "get": { "summary": "Health check" }
                }
            }
        }"#;

        let col = parse_openapi(json).unwrap();
        assert_eq!(col.name, "Server API");
        assert_eq!(col.requests.len(), 1);
        assert_eq!(col.requests[0].url, "https://api.example.com/v1/health");
    }
}
