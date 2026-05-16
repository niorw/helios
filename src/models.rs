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

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct Folder {
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub seq: u32,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub docs: String,
    #[serde(default)]
    pub folders: Vec<Folder>,
    #[serde(default)]
    pub requests: Vec<Request>,
    #[serde(default)]
    pub created_at: DateTime<Local>,
}

impl std::fmt::Display for Folder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl Folder {
    /// Recursively collect all requests from this folder and its sub-folders
    pub fn all_requests(&self) -> Vec<&Request> {
        let mut result: Vec<&Request> = self.requests.iter().collect();
        for sub in &self.folders {
            result.extend(sub.all_requests());
        }
        result
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Collection {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub folders: Vec<Folder>,
    pub requests: Vec<Request>,
    pub created_at: DateTime<Local>,
}

impl Collection {
    /// Recursively collect all requests from root and all folders
    pub fn all_requests(&self) -> Vec<&Request> {
        let mut result: Vec<&Request> = self.requests.iter().collect();
        for folder in &self.folders {
            result.extend(folder.all_requests());
        }
        result
    }
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
    fn test_folder_nested_hierarchy() {
        // Build: root > sub1 > sub2 > sub3 (4 levels of nesting)
        let sub3 = Folder {
            id: "sub3".to_string(),
            name: "Level 3".to_string(),
            seq: 1,
            variables: HashMap::new(),
            docs: String::new(),
            folders: vec![],
            requests: vec![Request::new(
                "Deep Request",
                HttpMethod::GET,
                "https://deep.example.com",
            )],
            created_at: chrono::Local::now(),
        };
        let sub2 = Folder {
            id: "sub2".to_string(),
            name: "Level 2".to_string(),
            seq: 1,
            variables: HashMap::new(),
            docs: String::new(),
            folders: vec![sub3],
            requests: vec![],
            created_at: chrono::Local::now(),
        };
        let sub1 = Folder {
            id: "sub1".to_string(),
            name: "Level 1".to_string(),
            seq: 1,
            variables: HashMap::new(),
            docs: String::new(),
            folders: vec![sub2],
            requests: vec![],
            created_at: chrono::Local::now(),
        };
        let root = Folder {
            id: "root".to_string(),
            name: "Root Folder".to_string(),
            seq: 0,
            variables: HashMap::new(),
            docs: String::new(),
            folders: vec![sub1],
            requests: vec![Request::new(
                "Root Request",
                HttpMethod::GET,
                "https://root.example.com",
            )],
            created_at: chrono::Local::now(),
        };

        // Verify structure: root.folders[0].folders[0].folders[0]
        assert_eq!(root.name, "Root Folder");
        assert_eq!(root.folders[0].name, "Level 1");
        assert_eq!(root.folders[0].folders[0].name, "Level 2");
        assert_eq!(root.folders[0].folders[0].folders[0].name, "Level 3");
        assert_eq!(root.folders[0].folders[0].folders[0].folders.len(), 0);

        // Verify Display trait
        assert_eq!(format!("{}", root), "Root Folder");
        assert_eq!(
            format!("{}", root.folders[0].folders[0].folders[0]),
            "Level 3"
        );
    }

    #[test]
    fn test_collection_with_folders() {
        let folder1 = Folder {
            id: "f1".to_string(),
            name: "Auth APIs".to_string(),
            seq: 0,
            variables: HashMap::new(),
            docs: String::new(),
            folders: vec![],
            requests: vec![Request::new(
                "Login",
                HttpMethod::POST,
                "https://api.example.com/login",
            )],
            created_at: chrono::Local::now(),
        };
        let folder2 = Folder {
            id: "f2".to_string(),
            name: "User APIs".to_string(),
            seq: 1,
            variables: HashMap::new(),
            docs: String::new(),
            folders: vec![],
            requests: vec![Request::new(
                "Get User",
                HttpMethod::GET,
                "https://api.example.com/users/1",
            )],
            created_at: chrono::Local::now(),
        };

        let col = Collection {
            id: "col1".to_string(),
            name: "My Collection".to_string(),
            folders: vec![folder1, folder2],
            requests: vec![Request::new(
                "Health Check",
                HttpMethod::GET,
                "https://api.example.com/health",
            )],
            created_at: chrono::Local::now(),
        };

        // Root-level requests
        assert_eq!(col.requests.len(), 1);
        assert_eq!(col.requests[0].name, "Health Check");

        // Folders
        assert_eq!(col.folders.len(), 2);
        assert_eq!(col.folders[0].name, "Auth APIs");
        assert_eq!(col.folders[0].requests.len(), 1);
        assert_eq!(col.folders[1].name, "User APIs");
        assert_eq!(col.folders[1].requests.len(), 1);

        // All requests via helper
        let all = col.all_requests();
        assert_eq!(all.len(), 3);
    }

    #[test]
    fn test_folder_variables() {
        let mut vars = HashMap::new();
        vars.insert(
            "base_url".to_string(),
            "https://api.example.com".to_string(),
        );
        vars.insert("token".to_string(), "abc123".to_string());

        let folder = Folder {
            id: "f1".to_string(),
            name: "Production".to_string(),
            seq: 0,
            variables: vars,
            docs: String::new(),
            folders: vec![],
            requests: vec![],
            created_at: chrono::Local::now(),
        };

        assert_eq!(folder.variables.len(), 2);
        assert_eq!(
            folder.variables.get("base_url").unwrap(),
            "https://api.example.com"
        );
        assert_eq!(folder.variables.get("token").unwrap(), "abc123");

        // Verify serialization round-trip with variables
        let json = serde_json::to_string(&folder).unwrap();
        let parsed: Folder = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.variables, folder.variables);
    }

    #[test]
    fn test_folder_tree_walk() {
        // Build tree:
        // Collection
        //   ├── root_request1
        //   ├── root_request2
        //   ├── Folder A
        //   │   ├── req_a1
        //   │   └── SubFolder B
        //   │       └── req_b1
        //   └── Folder C
        //       └── req_c1

        let sub_b = Folder {
            id: "sub_b".to_string(),
            name: "SubFolder B".to_string(),
            seq: 0,
            variables: HashMap::new(),
            docs: String::new(),
            folders: vec![],
            requests: vec![Request::new(
                "req_b1",
                HttpMethod::POST,
                "https://b.example.com",
            )],
            created_at: chrono::Local::now(),
        };

        let folder_a = Folder {
            id: "folder_a".to_string(),
            name: "Folder A".to_string(),
            seq: 0,
            variables: HashMap::new(),
            docs: String::new(),
            folders: vec![sub_b],
            requests: vec![Request::new(
                "req_a1",
                HttpMethod::GET,
                "https://a.example.com",
            )],
            created_at: chrono::Local::now(),
        };

        let folder_c = Folder {
            id: "folder_c".to_string(),
            name: "Folder C".to_string(),
            seq: 1,
            variables: HashMap::new(),
            docs: String::new(),
            folders: vec![],
            requests: vec![Request::new(
                "req_c1",
                HttpMethod::DELETE,
                "https://c.example.com",
            )],
            created_at: chrono::Local::now(),
        };

        let col = Collection {
            id: "col_walk".to_string(),
            name: "Walk Test".to_string(),
            folders: vec![folder_a, folder_c],
            requests: vec![
                Request::new(
                    "root_request1",
                    HttpMethod::GET,
                    "https://root1.example.com",
                ),
                Request::new(
                    "root_request2",
                    HttpMethod::PUT,
                    "https://root2.example.com",
                ),
            ],
            created_at: chrono::Local::now(),
        };

        // Walk all requests via all_requests()
        let all = col.all_requests();
        assert_eq!(all.len(), 5);

        let names: Vec<&str> = all.iter().map(|r| r.name.as_str()).collect();
        assert!(names.contains(&"root_request1"));
        assert!(names.contains(&"root_request2"));
        assert!(names.contains(&"req_a1"));
        assert!(names.contains(&"req_b1"));
        assert!(names.contains(&"req_c1"));

        // Folder-level walk
        let folder_a_all = col.folders[0].all_requests();
        assert_eq!(folder_a_all.len(), 2);
        let names_a: Vec<&str> = folder_a_all.iter().map(|r| r.name.as_str()).collect();
        assert!(names_a.contains(&"req_a1"));
        assert!(names_a.contains(&"req_b1"));
    }

    #[test]
    fn test_collection_default() {
        let col: Collection = Default::default();
        assert!(col.requests.is_empty());
        assert!(col.folders.is_empty());
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

    #[test]
    fn test_collection_backward_compat_deserialize() {
        // Old JSON without folders/seq/variables fields should still deserialize
        let old_json = r#"{"id":"c1","name":"Old Collection","requests":[],"created_at":"2024-01-01T00:00:00+08:00"}"#;
        let col: Collection = serde_json::from_str(old_json).unwrap();
        assert_eq!(col.id, "c1");
        assert_eq!(col.name, "Old Collection");
        assert!(col.folders.is_empty());
        assert!(col.requests.is_empty());
    }

    #[test]
    fn test_folder_backward_compat_deserialize() {
        // Old JSON without seq/variables/docs fields should still deserialize
        let old_json = r#"{"id":"f1","name":"Old Folder","folders":[],"requests":[],"created_at":"2024-01-01T00:00:00+08:00"}"#;
        let folder: Folder = serde_json::from_str(old_json).unwrap();
        assert_eq!(folder.id, "f1");
        assert_eq!(folder.name, "Old Folder");
        assert_eq!(folder.seq, 0);
        assert!(folder.variables.is_empty());
        assert!(folder.folders.is_empty());
    }
}
