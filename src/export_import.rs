use crate::models::{Auth, BodyType, Collection, HttpMethod, KeyValue, Request};
use anyhow::{Context, Result};

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PostmanCollection {
    info: PostmanInfo,
    item: Vec<PostmanItem>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PostmanInfo {
    name: String,
    #[serde(rename = "_postman_id")]
    id: String,
    #[serde(rename = "schema")]
    _schema: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PostmanItem {
    name: String,
    request: PostmanRequest,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PostmanRequest {
    method: String,
    #[serde(default)]
    header: Vec<PostmanHeader>,
    url: PostmanUrl,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    body: Option<PostmanBody>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PostmanHeader {
    key: String,
    value: String,
    #[serde(default)]
    disabled: bool,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PostmanUrl {
    raw: String,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PostmanBody {
    mode: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    raw: Option<String>,
}

pub fn export_collection_json(col: &Collection) -> Result<String> {
    let json = serde_json::to_string_pretty(col)?;
    Ok(json)
}

pub fn export_collection_postman(col: &Collection) -> Result<String> {
    let items: Vec<PostmanItem> = col
        .requests
        .iter()
        .map(|req| {
            let headers: Vec<PostmanHeader> = req
                .headers
                .iter()
                .map(|h| PostmanHeader {
                    key: h.key.clone(),
                    value: h.value.clone(),
                    disabled: !h.enabled,
                })
                .collect();

            PostmanItem {
                name: req.name.clone(),
                request: PostmanRequest {
                    method: req.method.to_string(),
                    header: headers,
                    url: PostmanUrl {
                        raw: req.url.clone(),
                    },
                    body: if req.body.is_empty() {
                        None
                    } else {
                        Some(PostmanBody {
                            mode: req.body_type.to_string(),
                            raw: Some(req.body.clone()),
                        })
                    },
                },
            }
        })
        .collect();

    let postman = PostmanCollection {
        info: PostmanInfo {
            name: col.name.clone(),
            id: col.id.clone(),
            _schema: "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
                .to_string(),
        },
        item: items,
    };

    Ok(serde_json::to_string_pretty(&postman)?)
}

pub fn import_json(data: &str) -> Result<Collection> {
    let col: Collection =
        serde_json::from_str(data).context("Failed to parse as native JSON format")?;
    Ok(col)
}

pub fn import_postman(data: &str) -> Result<Collection> {
    let postman: PostmanCollection =
        serde_json::from_str(data).context("Failed to parse as Postman collection")?;

    let requests: Vec<Request> = postman
        .item
        .into_iter()
        .map(|item| {
            let method = parse_method(&item.request.method);
            let headers: Vec<KeyValue> = item
                .request
                .header
                .into_iter()
                .map(|h| KeyValue {
                    key: h.key,
                    value: h.value,
                    enabled: !h.disabled,
                })
                .collect();

            let (body_type, body) = if let Some(b) = item.request.body {
                let bt = match b.mode.as_str() {
                    "json" | "raw" => BodyType::Json,
                    "formdata" | "urlencoded" => BodyType::Form,
                    _ => BodyType::Text,
                };
                (bt, b.raw.unwrap_or_default())
            } else {
                (BodyType::None, String::new())
            };

            Request {
                id: uuid::Uuid::new_v4().to_string(),
                name: item.name,
                method,
                url: item.request.url.raw,
                headers,
                params: vec![],
                body,
                body_type,
                auth: Auth::None,
                graphql_query: None,
                graphql_variables: None,
                form_data: vec![],
                notes: String::new(),
            }
        })
        .collect();

    Ok(Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: postman.info.name,
        folders: vec![],
        requests,
        created_at: chrono::Local::now(),
    })
}

fn parse_method(s: &str) -> HttpMethod {
    match s.to_uppercase().as_str() {
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "DELETE" => HttpMethod::DELETE,
        "PATCH" => HttpMethod::PATCH,
        "HEAD" => HttpMethod::HEAD,
        "OPTIONS" => HttpMethod::OPTIONS,
        _ => HttpMethod::GET,
    }
}

pub fn guess_format(data: &str) -> &'static str {
    if data.trim().starts_with("{")
        && data.contains("info")
        && data.contains("schema")
        && data.contains("getpostman.com")
    {
        "postman"
    } else {
        "json"
    }
}
