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
    FormData,
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
            BodyType::FormData => write!(f, "form-data"),
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
    #[serde(default)]
    pub form_data: Vec<crate::helios_format::FormDataItem>,
    #[serde(default)]
    pub notes: String,
    /// 请求标签，用于分类和过滤（如 smoke, auth, regression）
    #[serde(default)]
    pub tags: Vec<String>,
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

// ─── 标签颜色生成 ──────────────────────────────────────────────────

/// 预定义标签调色板（RGB）
const TAG_PALETTE: [(u8, u8, u8); 12] = [
    (255, 121, 198), // 粉
    (0, 245, 212),   // 青
    (255, 184, 108), // 橙
    (139, 233, 253), // 浅蓝
    (241, 250, 140), // 黄
    (189, 147, 249), // 紫
    (80, 250, 123),  // 绿
    (255, 85, 85),   // 红
    (98, 214, 240),  // 天蓝
    (255, 213, 79),  // 金
    (206, 147, 216), // 淡紫
    (129, 236, 236), // 薄荷
];

/// 基于标签名 hash 生成一致的 RGB 颜色
pub fn tag_color(tag: &str) -> ratatui::style::Color {
    let hash = tag
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let idx = (hash as usize) % TAG_PALETTE.len();
    let (r, g, b) = TAG_PALETTE[idx];
    ratatui::style::Color::Rgb(r, g, b)
}

/// 收集所有集合中的唯一标签列表
pub fn collect_all_tags(collections: &[Collection]) -> Vec<String> {
    let mut tags: Vec<String> = collections
        .iter()
        .flat_map(|c| c.requests.iter())
        .flat_map(|r| r.tags.iter())
        .cloned()
        .collect();
    tags.sort();
    tags.dedup();
    tags
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
        let req = Request::new(
            "Test Request",
            HttpMethod::POST,
            "https://api.example.com/users",
        );
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
    fn test_response_default() {
        let resp: Response = Default::default();
        assert_eq!(resp.status, 0);
        assert_eq!(resp.body, "");
        assert_eq!(resp.duration_ms, 0);
    }

    // ─── 标签测试 ──────────────────────────────────────────────

    #[test]
    fn test_tag_add_to_request() {
        let mut req = Request::default();
        req.tags.push("smoke".to_string());
        assert_eq!(req.tags, vec!["smoke"]);
    }

    #[test]
    fn test_tag_multiple_tags() {
        let mut req = Request::default();
        req.tags = vec![
            "smoke".to_string(),
            "auth".to_string(),
            "regression".to_string(),
        ];
        assert_eq!(req.tags.len(), 3);
        assert!(req.tags.contains(&"auth".to_string()));
    }

    #[test]
    fn test_tag_color_generation() {
        // 同一标签总是生成同一颜色
        let c1 = tag_color("smoke");
        let c2 = tag_color("smoke");
        assert_eq!(c1, c2);
        // 不同标签生成不同颜色（大概率）
        let c3 = tag_color("auth");
        assert_ne!(c1, c3);
    }

    #[test]
    fn test_tag_in_helios_yml() {
        let mut req = Request::default();
        req.tags = vec!["smoke".to_string(), "auth".to_string()];
        let yml = serde_yaml::to_string(&req).unwrap();
        assert!(yml.contains("smoke"));
        assert!(yml.contains("auth"));
    }

    #[test]
    fn test_collect_all_tags() {
        let mut col1 = Collection::default();
        let mut req1 = Request::default();
        req1.tags = vec!["smoke".to_string(), "auth".to_string()];
        col1.requests.push(req1);

        let mut col2 = Collection::default();
        let mut req2 = Request::default();
        req2.tags = vec!["auth".to_string(), "regression".to_string()];
        col2.requests.push(req2);

        let tags = collect_all_tags(&[col1, col2]);
        assert_eq!(tags, vec!["auth", "regression", "smoke"]);
    }
}
