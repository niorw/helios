use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    #[default]
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
}

impl std::fmt::Display for HttpMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct KeyValue {
    pub key: String,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum BodyType {
    #[default]
    None,
    Json,
    Form,
    Text,
    Xml,
    Graphql,
}

impl std::fmt::Display for BodyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BodyType::None => write!(f, "none"),
            BodyType::Json => write!(f, "json"),
            BodyType::Form => write!(f, "form"),
            BodyType::Text => write!(f, "text"),
            BodyType::Xml => write!(f, "xml"),
            BodyType::Graphql => write!(f, "graphql"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub enum Auth {
    #[default]
    None,
    Bearer {
        token: String,
    },
    Basic {
        username: String,
        password: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Request {
    pub id: String,
    pub name: String,
    pub method: HttpMethod,
    pub url: String,
    pub headers: Vec<KeyValue>,
    pub params: Vec<KeyValue>,
    pub body: String,
    pub body_type: BodyType,
    pub auth: Auth,
    #[serde(default)]
    pub graphql_query: Option<String>,
    #[serde(default)]
    pub graphql_variables: Option<String>,
}

impl Request {
    pub fn new(name: &str, method: HttpMethod, url: &str) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: name.to_string(),
            method,
            url: url.to_string(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Response {
    pub status: u16,
    pub status_text: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub duration_ms: u64,
    pub timestamp: DateTime<Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub requests: Vec<Request>,
    pub created_at: DateTime<Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Environment {
    pub id: String,
    pub name: String,
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryItem {
    pub id: String,
    pub request: Request,
    pub response: Response,
    pub timestamp: DateTime<Local>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AppData {
    pub collections: Vec<Collection>,
    pub environments: Vec<Environment>,
    pub history: Vec<HistoryItem>,
    pub active_env_id: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_method_display() {
        assert_eq!(format!("{}", HttpMethod::GET), "GET");
        assert_eq!(format!("{}", HttpMethod::POST), "POST");
        assert_eq!(format!("{}", HttpMethod::PUT), "PUT");
        assert_eq!(format!("{}", HttpMethod::DELETE), "DELETE");
    }

    #[test]
    fn test_body_type_display() {
        assert_eq!(format!("{}", BodyType::None), "none");
        assert_eq!(format!("{}", BodyType::Json), "json");
        assert_eq!(format!("{}", BodyType::Form), "form");
        assert_eq!(format!("{}", BodyType::Text), "text");
        assert_eq!(format!("{}", BodyType::Xml), "xml");
    }

    #[test]
    fn test_request_new() {
        let req = Request::new("Test Request", HttpMethod::POST, "https://api.example.com/users");
        assert_eq!(req.name, "Test Request");
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.url, "https://api.example.com/users");
        assert!(!req.id.is_empty());
        assert!(req.headers.is_empty());
        assert!(req.params.is_empty());
    }

    #[test]
    fn test_request_default() {
        let req: Request = Default::default();
        assert_eq!(req.method, HttpMethod::GET);
        assert_eq!(req.body_type, BodyType::None);
        assert_eq!(req.auth, Auth::None);
    }

    #[test]
    fn test_key_value_serialization() {
        let kv = KeyValue {
            key: "Content-Type".to_string(),
            value: "application/json".to_string(),
            enabled: true,
        };
        let json = serde_json::to_string(&kv).unwrap();
        assert!(json.contains("Content-Type"));
        assert!(json.contains("application/json"));
    }

    #[test]
    fn test_collection_default() {
        let col: Collection = Default::default();
        assert!(col.requests.is_empty());
        assert!(col.name.is_empty());
    }

    #[test]
    fn test_environment_default() {
        let env: Environment = Default::default();
        assert!(env.variables.is_empty());
        assert!(env.name.is_empty());
    }

    #[test]
    fn test_app_data_default() {
        let data: AppData = Default::default();
        assert!(data.collections.is_empty());
        assert!(data.environments.is_empty());
        assert!(data.history.is_empty());
        assert!(data.active_env_id.is_none());
    }

    #[test]
    fn test_auth_bearer() {
        let auth = Auth::Bearer {
            token: "secret_token".to_string(),
        };
        match auth {
            Auth::Bearer { token } => assert_eq!(token, "secret_token"),
            _ => panic!("Expected Bearer auth"),
        }
    }

    #[test]
    fn test_auth_basic() {
        let auth = Auth::Basic {
            username: "admin".to_string(),
            password: "password".to_string(),
        };
        match auth {
            Auth::Basic { username, password } => {
                assert_eq!(username, "admin");
                assert_eq!(password, "password");
            }
            _ => panic!("Expected Basic auth"),
        }
    }

    #[test]
    fn test_body_type_graphql_display() {
        assert_eq!(BodyType::Graphql.to_string(), "graphql");
    }

    #[test]
    fn test_request_graphql_fields() {
        let mut req = Request::default();
        req.graphql_query = Some("query { users { id name } }".to_string());
        req.graphql_variables = Some(r#"{"limit":10}"#.to_string());
        req.body_type = BodyType::Graphql;
        assert!(req.graphql_query.is_some());
        assert_eq!(req.body_type, BodyType::Graphql);
    }

    #[test]
    fn test_request_graphql_serialization() {
        let mut req = Request::default();
        req.graphql_query = Some("{ hello }".to_string());
        req.body_type = BodyType::Graphql;
        let json = serde_json::to_string(&req).unwrap();
        let loaded: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.graphql_query, Some("{ hello }".to_string()));
        assert_eq!(loaded.body_type, BodyType::Graphql);
    }

    #[test]
    fn test_response_default() {
        let resp: Response = Default::default();
        assert_eq!(resp.status, 0);
        assert_eq!(resp.body, "");
        assert_eq!(resp.duration_ms, 0);
    }
}
