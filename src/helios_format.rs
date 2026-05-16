//! .helios.yml 文件格式解析与序列化
//!
//! Phase 0 - F01: 将 API 请求定义为 YAML 文件格式，
//! 对齐 Bruno OpenCollection 规范但做超集扩展。
//! 一个 .helios.yml 文件 = 一个请求，一个目录 = 一个集合。

use crate::models::{Auth, BodyType, HttpMethod, KeyValue, Request};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// .helios.yml 文件中的表单项
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct FormDataItem {
    pub key: String,
    pub value: String,
    #[serde(default)]
    pub is_file: bool,
    #[serde(default)]
    pub file_path: Option<String>,
}

// ─── .helios.yml 文件格式数据结构 ──────────────────────────────────

/// .helios.yml 文件根结构
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HeliosYml {
    #[serde(default)]
    pub info: HeliosInfo,
    #[serde(default)]
    pub http: HeliosHttp,
    #[serde(default)]
    pub runtime: Option<HeliosRuntime>,
    #[serde(default)]
    pub settings: Option<HeliosSettings>,
    #[serde(default)]
    pub docs: Option<String>,
}

/// 请求元信息
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HeliosInfo {
    pub name: String,
    #[serde(default = "default_request_type")]
    pub r#type: String,
    #[serde(default)]
    pub seq: u32,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_request_type() -> String {
    "http".to_string()
}

/// HTTP 请求定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HeliosHttp {
    #[serde(default = "default_method")]
    pub method: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub params: Vec<KeyValue>,
    #[serde(default)]
    pub headers: Vec<KeyValue>,
    #[serde(default)]
    pub body: Option<HeliosBody>,
    #[serde(default)]
    pub auth: Option<HeliosAuth>,
}

fn default_method() -> String {
    "GET".to_string()
}

/// 请求体定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HeliosBody {
    #[serde(default = "default_body_type")]
    pub r#type: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub graphql_query: Option<String>,
    #[serde(default)]
    pub graphql_variables: Option<String>,
    #[serde(default)]
    pub form_data: Vec<FormDataItem>,
}

fn default_body_type() -> String {
    "none".to_string()
}

/// 认证定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HeliosAuth {
    pub r#type: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub token: Option<String>,
}

/// 运行时操作定义
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HeliosRuntime {
    #[serde(default)]
    pub pre_request: Vec<HeliosAction>,
    #[serde(default)]
    pub post_response: Vec<HeliosAction>,
}

/// 单个运行时操作
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HeliosAction {
    pub action: String,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub var_name: Option<String>,
    #[serde(default)]
    pub json_path: Option<String>,
    #[serde(default)]
    pub assert: Option<String>,
    #[serde(default)]
    pub operator: Option<String>,
    #[serde(default)]
    pub expected: Option<String>,
}

/// 请求设置
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct HeliosSettings {
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_true")]
    pub follow_redirects: bool,
    #[serde(default = "default_max_redirects")]
    pub max_redirects: u32,
    #[serde(default = "default_true")]
    pub encode_url: bool,
}

fn default_timeout() -> u64 {
    30000
}

fn default_true() -> bool {
    true
}

fn default_max_redirects() -> u32 {
    5
}

// ─── 集合元数据 collection.yml ──────────────────────────────────

/// 集合根目录的 collection.yml 结构
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct CollectionYml {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
}

// ─── 转换函数：HeliosYml ↔ Request ──────────────────────────────────

/// 将 HeliosYml 解析为内部 Request 模型
pub fn helios_yml_to_request(yml: &HeliosYml, id: &str) -> Result<Request> {
    let method = parse_method_from_str(&yml.http.method)?;
    let body_type = parse_body_type_from_str(
        yml.http
            .body
            .as_ref()
            .map(|b| b.r#type.as_str())
            .unwrap_or("none"),
    );
    let auth = parse_auth_from_helios(&yml.http.auth);

    let body_content = yml
        .http
        .body
        .as_ref()
        .map(|b| b.content.clone())
        .unwrap_or_default();
    let graphql_query = yml.http.body.as_ref().and_then(|b| b.graphql_query.clone());
    let graphql_variables = yml
        .http
        .body
        .as_ref()
        .and_then(|b| b.graphql_variables.clone());
    let form_data = yml
        .http
        .body
        .as_ref()
        .map(|b| b.form_data.clone())
        .unwrap_or_default();

    Ok(Request {
        id: id.to_string(),
        name: yml.info.name.clone(),
        method,
        url: yml.http.url.clone(),
        headers: yml.http.headers.clone(),
        params: yml.http.params.clone(),
        body: body_content,
        body_type,
        auth,
        graphql_query,
        graphql_variables,
        form_data,
        notes: String::new(),
    })
}

/// 将内部 Request 模型转换为 HeliosYml
pub fn request_to_helios_yml(req: &Request) -> Result<HeliosYml> {
    let body = if req.body_type == BodyType::None && req.body.is_empty() && req.form_data.is_empty()
    {
        None
    } else {
        Some(HeliosBody {
            r#type: body_type_to_yaml_str(&req.body_type),
            content: req.body.clone(),
            graphql_query: req.graphql_query.clone(),
            graphql_variables: req.graphql_variables.clone(),
            form_data: req.form_data.clone(),
        })
    };

    let auth = if matches!(req.auth, Auth::None) {
        None
    } else {
        Some(auth_to_helios(&req.auth))
    };

    Ok(HeliosYml {
        info: HeliosInfo {
            name: req.name.clone(),
            r#type: "http".to_string(),
            seq: 0,
            tags: vec![],
        },
        http: HeliosHttp {
            method: method_to_yaml_str(&req.method),
            url: req.url.clone(),
            params: req.params.clone(),
            headers: req.headers.clone(),
            body,
            auth,
        },
        runtime: None,
        settings: None,
        docs: if req.notes.is_empty() {
            None
        } else {
            Some(req.notes.clone())
        },
    })
}

// ─── 内部辅助函数 ──────────────────────────────────────────────────

fn parse_method_from_str(s: &str) -> Result<HttpMethod> {
    match s.to_uppercase().as_str() {
        "GET" => Ok(HttpMethod::GET),
        "POST" => Ok(HttpMethod::POST),
        "PUT" => Ok(HttpMethod::PUT),
        "DELETE" => Ok(HttpMethod::DELETE),
        "PATCH" => Ok(HttpMethod::PATCH),
        "HEAD" => Ok(HttpMethod::HEAD),
        "OPTIONS" => Ok(HttpMethod::OPTIONS),
        _ => Err(anyhow::anyhow!("未知的 HTTP 方法: {}", s)),
    }
}

fn parse_body_type_from_str(s: &str) -> BodyType {
    match s {
        "json" => BodyType::Json,
        "form-urlencoded" | "form" => BodyType::Form,
        "text" => BodyType::Text,
        "xml" => BodyType::Xml,
        "graphql" => BodyType::Graphql,
        "multipart-form" | "form-data" => BodyType::FormData,
        _ => BodyType::None,
    }
}

fn parse_auth_from_helios(auth: &Option<HeliosAuth>) -> Auth {
    match auth {
        Some(a) => match a.r#type.as_str() {
            "bearer" => Auth::Bearer {
                token: a.token.clone().unwrap_or_default(),
            },
            "basic" => Auth::Basic {
                username: a.username.clone().unwrap_or_default(),
                password: a.password.clone().unwrap_or_default(),
            },
            _ => Auth::None,
        },
        None => Auth::None,
    }
}

fn method_to_yaml_str(method: &HttpMethod) -> String {
    match method {
        HttpMethod::GET => "GET".to_string(),
        HttpMethod::POST => "POST".to_string(),
        HttpMethod::PUT => "PUT".to_string(),
        HttpMethod::DELETE => "DELETE".to_string(),
        HttpMethod::PATCH => "PATCH".to_string(),
        HttpMethod::HEAD => "HEAD".to_string(),
        HttpMethod::OPTIONS => "OPTIONS".to_string(),
    }
}

fn body_type_to_yaml_str(bt: &BodyType) -> String {
    match bt {
        BodyType::None => "none".to_string(),
        BodyType::Json => "json".to_string(),
        BodyType::Form => "form-urlencoded".to_string(),
        BodyType::Text => "text".to_string(),
        BodyType::Xml => "xml".to_string(),
        BodyType::Graphql => "graphql".to_string(),
        BodyType::FormData => "multipart-form".to_string(),
    }
}

fn auth_to_helios(auth: &Auth) -> HeliosAuth {
    match auth {
        Auth::Bearer { token } => HeliosAuth {
            r#type: "bearer".to_string(),
            token: Some(token.clone()),
            username: None,
            password: None,
        },
        Auth::Basic { username, password } => HeliosAuth {
            r#type: "basic".to_string(),
            token: None,
            username: Some(username.clone()),
            password: Some(password.clone()),
        },
        Auth::None => HeliosAuth {
            r#type: "none".to_string(),
            token: None,
            username: None,
            password: None,
        },
    }
}

/// 从 YAML 字符串解析 HeliosYml
pub fn parse_helios_yml(content: &str) -> Result<HeliosYml> {
    let yml: HeliosYml = serde_yaml::from_str(content)?;
    Ok(yml)
}

/// 将 HeliosYml 序列化为 YAML 字符串
pub fn serialize_helios_yml(yml: &HeliosYml) -> Result<String> {
    let content = serde_yaml::to_string(yml)?;
    Ok(content)
}

/// 从文件路径加载 HeliosYml
pub fn load_helios_yml(path: &Path) -> Result<HeliosYml> {
    let content = std::fs::read_to_string(path)?;
    parse_helios_yml(&content)
}

/// 将 HeliosYml 保存到文件路径
pub fn save_helios_yml(yml: &HeliosYml, path: &Path) -> Result<()> {
    let content = serialize_helios_yml(yml)?;
    std::fs::write(path, content)?;
    Ok(())
}

/// 从目录路径加载整个集合
pub fn load_collection_from_dir(dir: &Path) -> Result<crate::models::Collection> {
    // 读取 collection.yml 获取集合名称
    let col_yml_path = dir.join("collection.yml");
    let col_name = if col_yml_path.exists() {
        let content = std::fs::read_to_string(&col_yml_path)?;
        let col_yml: CollectionYml = serde_yaml::from_str(&content)?;
        col_yml.name
    } else {
        dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    };

    // 扫描目录下所有 .helios.yml 文件
    let mut requests = Vec::new();
    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name() {
            let name = name.to_string_lossy();
            if name.ends_with(".helios.yml") {
                if let Ok(yml) = load_helios_yml(&path) {
                    let file_stem = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default()
                        .replace(".helios", "");
                    let req = helios_yml_to_request(&yml, &file_stem)?;
                    requests.push(req);
                }
            }
        }
    }

    // 按 info.seq 排序（通过重新读取，但这里简化为按文件名排序）
    // 从 helios_yml 中读取 seq，按 seq 排序
    let mut indexed: Vec<(u32, Request)> = Vec::new();
    for req in requests {
        // 重新加载 yml 获取 seq
        let path = dir.join(format!("{}.helios.yml", req.id));
        let seq = if path.exists() {
            load_helios_yml(&path).map(|y| y.info.seq).unwrap_or(0)
        } else {
            0
        };
        indexed.push((seq, req));
    }
    indexed.sort_by_key(|(seq, _)| *seq);

    Ok(crate::models::Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: col_name,
        requests: indexed.into_iter().map(|(_, req)| req).collect(),
        created_at: chrono::Local::now(),
    })
}

// ─── 测试 ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_helios_yml_parse_basic_request() {
        // 基本 GET 请求解析
        let yaml_content = r#"
info:
  name: 获取用户列表
  type: http
  seq: 1
  tags:
    - 用户
    - 列表
http:
  method: GET
  url: https://api.example.com/users
  params:
    - key: page
      value: "1"
      enabled: true
  headers:
    - key: Accept
      value: application/json
      enabled: true
"#;
        let result = parse_helios_yml(yaml_content).expect("解析基本请求应成功");
        assert_eq!(result.info.name, "获取用户列表");
        assert_eq!(result.info.r#type, "http");
        assert_eq!(result.http.method, "GET");
        assert_eq!(result.http.url, "https://api.example.com/users");
        assert_eq!(result.http.params.len(), 1);
        assert_eq!(result.http.params[0].key, "page");
        assert_eq!(result.http.headers.len(), 1);
    }

    #[test]
    fn test_helios_yml_parse_with_auth() {
        // 带认证的请求解析
        let yaml_content = r#"
info:
  name: 创建用户
  type: http
  seq: 2
http:
  method: POST
  url: https://api.example.com/users
  body:
    type: json
    content: '{"name": "test"}'
  auth:
    type: bearer
    token: my-secret-token
"#;
        let result = parse_helios_yml(yaml_content).expect("解析带认证请求应成功");
        assert_eq!(result.http.method, "POST");
        let auth = result.http.auth.expect("应有认证配置");
        assert_eq!(auth.r#type, "bearer");
        assert_eq!(auth.token, Some("my-secret-token".to_string()));
        let body = result.http.body.expect("应有请求体");
        assert_eq!(body.r#type, "json");
        assert_eq!(body.content, r#"{"name": "test"}"#);
    }

    #[test]
    fn test_helios_yml_parse_with_tests() {
        // 带运行时操作和测试的请求解析
        let yaml_content = r#"
info:
  name: 登录测试
  type: http
  seq: 1
http:
  method: POST
  url: https://api.example.com/login
  body:
    type: json
    content: '{"username":"admin","password":"pass"}'
runtime:
  post_response:
    - action: extract
      var_name: token
      json_path: "$.data.token"
    - action: assert
      assert: status
      operator: equals
      expected: "200"
settings:
  timeout: 10000
  follow_redirects: false
"#;
        let result = parse_helios_yml(yaml_content).expect("解析带运行时请求应成功");
        let runtime = result.runtime.expect("应有运行时配置");
        assert_eq!(runtime.post_response.len(), 2);
        assert_eq!(runtime.post_response[0].action, "extract");
        assert_eq!(runtime.post_response[0].var_name, Some("token".to_string()));
        assert_eq!(runtime.post_response[1].action, "assert");
        let settings = result.settings.expect("应有设置");
        assert_eq!(settings.timeout, 10000);
        assert!(!settings.follow_redirects);
    }

    #[test]
    fn test_helios_yml_roundtrip() {
        // 序列化再反解析应保持一致
        let original = HeliosYml {
            info: HeliosInfo {
                name: "测试请求".to_string(),
                r#type: "http".to_string(),
                seq: 3,
                tags: vec!["smoke".to_string()],
            },
            http: HeliosHttp {
                method: "PUT".to_string(),
                url: "https://api.example.com/users/1".to_string(),
                params: vec![KeyValue {
                    key: "debug".to_string(),
                    value: "true".to_string(),
                    enabled: true,
                }],
                headers: vec![KeyValue {
                    key: "Content-Type".to_string(),
                    value: "application/json".to_string(),
                    enabled: true,
                }],
                body: Some(HeliosBody {
                    r#type: "json".to_string(),
                    content: r#"{"name":"updated"}"#.to_string(),
                    graphql_query: None,
                    graphql_variables: None,
                    form_data: vec![],
                }),
                auth: None,
            },
            runtime: None,
            settings: Some(HeliosSettings {
                timeout: 5000,
                follow_redirects: true,
                max_redirects: 3,
                encode_url: false,
            }),
            docs: Some("更新用户信息".to_string()),
        };

        let yaml_str = serialize_helios_yml(&original).expect("序列化应成功");
        let parsed = parse_helios_yml(&yaml_str).expect("反解析应成功");
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_helios_yml_to_request_conversion() {
        // HeliosYml 转 Request 模型
        let yml = HeliosYml {
            info: HeliosInfo {
                name: "获取用户".to_string(),
                r#type: "http".to_string(),
                seq: 1,
                tags: vec![],
            },
            http: HeliosHttp {
                method: "GET".to_string(),
                url: "https://api.example.com/users".to_string(),
                params: vec![KeyValue {
                    key: "id".to_string(),
                    value: "1".to_string(),
                    enabled: true,
                }],
                headers: vec![],
                body: None,
                auth: Some(HeliosAuth {
                    r#type: "bearer".to_string(),
                    token: Some("tok123".to_string()),
                    username: None,
                    password: None,
                }),
            },
            runtime: None,
            settings: None,
            docs: None,
        };

        let req = helios_yml_to_request(&yml, "test-id-001").expect("转换应成功");
        assert_eq!(req.id, "test-id-001");
        assert_eq!(req.name, "获取用户");
        assert_eq!(req.method, HttpMethod::GET);
        assert_eq!(req.url, "https://api.example.com/users");
        assert_eq!(req.params.len(), 1);
        assert_eq!(
            req.auth,
            Auth::Bearer {
                token: "tok123".to_string(),
            }
        );
    }

    #[test]
    fn test_request_to_helios_yml_conversion() {
        // Request 模型转 HeliosYml
        let mut req = Request::new(
            "创建订单",
            HttpMethod::POST,
            "https://api.example.com/orders",
        );
        req.body_type = BodyType::Json;
        req.body = r#"{"item":"book"}"#.to_string();
        req.auth = Auth::Basic {
            username: "admin".to_string(),
            password: "pass123".to_string(),
        };

        let yml = request_to_helios_yml(&req).expect("转换应成功");
        assert_eq!(yml.info.name, "创建订单");
        assert_eq!(yml.http.method, "POST");
        assert_eq!(yml.http.url, "https://api.example.com/orders");
        let body = yml.http.body.expect("应有请求体");
        assert_eq!(body.r#type, "json");
        assert_eq!(body.content, r#"{"item":"book"}"#);
        let auth = yml.http.auth.expect("应有认证");
        assert_eq!(auth.r#type, "basic");
        assert_eq!(auth.username, Some("admin".to_string()));
    }

    #[test]
    fn test_load_collection_from_directory() {
        // 从目录加载集合
        use std::fs;
        let tmp_dir = std::env::temp_dir().join("helios_test_collection");
        let _ = fs::remove_dir_all(&tmp_dir);
        fs::create_dir_all(&tmp_dir).unwrap();

        // 创建 collection.yml
        let col_yml = r#"
name: 测试集合
description: 用于测试的集合
"#;
        fs::write(tmp_dir.join("collection.yml"), col_yml).unwrap();

        // 创建请求文件
        let req_yml = r#"
info:
  name: 获取列表
  type: http
  seq: 1
http:
  method: GET
  url: https://api.example.com/items
"#;
        fs::write(tmp_dir.join("list-items.helios.yml"), req_yml).unwrap();

        let collection = load_collection_from_dir(&tmp_dir).expect("从目录加载集合应成功");
        assert_eq!(collection.name, "测试集合");
        assert_eq!(collection.requests.len(), 1);
        assert_eq!(collection.requests[0].name, "获取列表");

        // 清理
        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_yaml_syntax_validation() {
        // 无效 YAML 应返回错误
        let bad_yaml = "info: [broken\n  yaml: content";
        let result = parse_helios_yml(bad_yaml);
        assert!(result.is_err(), "无效 YAML 应解析失败");

        // 缺少 info 字段应使用默认值
        let minimal_yaml = r#"
http:
  method: GET
  url: https://example.com
"#;
        let result = parse_helios_yml(minimal_yaml).expect("最简 YAML 应解析成功");
        // info 的 name 应为默认空字符串
        assert_eq!(result.info.name, "");
        assert_eq!(result.http.method, "GET");
    }

    #[test]
    fn test_helios_body_type_mapping() {
        // 所有 BodyType 应正确映射
        let types = vec![
            (BodyType::None, "none", false),
            (BodyType::Json, "json", true),
            (BodyType::Form, "form-urlencoded", true),
            (BodyType::Text, "text", true),
            (BodyType::Xml, "xml", true),
            (BodyType::Graphql, "graphql", true),
            (BodyType::FormData, "multipart-form", true),
        ];
        for (bt, yaml_type, expect_body) in types {
            let mut req = Request::default();
            req.body_type = bt.clone();
            req.name = format!("test_{}", yaml_type);
            // 非 None 类型需要设置 body 内容才能触发序列化
            if expect_body {
                req.body = "test-content".to_string();
            }

            let yml = request_to_helios_yml(&req).expect("转换应成功");
            if expect_body {
                let body = yml.http.body.as_ref().expect("应有 body");
                assert_eq!(
                    body.r#type, yaml_type,
                    "BodyType {:?} 应映射为 {}",
                    bt, yaml_type
                );
            }
            // BodyType::None + 空 body => 不序列化 body 字段（设计决策：省略空body）
        }
    }
}
