/// 响应提取变量模块
/// 从 HTTP 响应中提取值并存入环境变量
use crate::models::Response;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Extraction {
    /// 要存入的变量名
    pub var_name: String,
    /// JSON 路径，如 "data.token", "user.id"
    pub json_path: String,
}

/// 从响应中提取变量值
/// 支持简单的 JSON 点分路径: "data.token" -> response.body.data.token
pub fn extract_variables(
    resp: &Response,
    extractions: &[Extraction],
) -> HashMap<String, String> {
    let mut result = HashMap::new();

    for ext in extractions {
        if let Some(value) = extract_by_path(resp, &ext.json_path) {
            result.insert(ext.var_name.clone(), value);
        }
    }

    result
}

fn extract_by_path(resp: &Response, path: &str) -> Option<String> {
    let json: serde_json::Value = serde_json::from_str(&resp.body).ok()?;

    let parts: Vec<&str> = path.split('.').collect();
    let mut current = &json;

    for key in &parts {
        current = current.get(key)?;
    }

    Some(json_value_to_string(current))
}

fn json_value_to_string(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "null".to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn make_response(body: &str) -> Response {
        Response {
            status: 200,
            status_text: "OK".to_string(),
            headers: HashMap::new(),
            body: body.to_string(),
            duration_ms: 100,
            timestamp: chrono::Local::now(),
        }
    }

    #[test]
    fn test_extract_simple_path() {
        let resp = make_response(r#"{"token":"abc123","user":"alice"}"#);
        let exts = vec![Extraction {
            var_name: "auth_token".to_string(),
            json_path: "token".to_string(),
        }];
        let vars = extract_variables(&resp, &exts);
        assert_eq!(vars.get("auth_token").unwrap(), "abc123");
    }

    #[test]
    fn test_extract_nested_path() {
        let resp = make_response(r#"{"data":{"user":{"id":42,"name":"bob"}}}"#);
        let exts = vec![Extraction {
            var_name: "user_id".to_string(),
            json_path: "data.user.id".to_string(),
        }];
        let vars = extract_variables(&resp, &exts);
        assert_eq!(vars.get("user_id").unwrap(), "42");
    }

    #[test]
    fn test_extract_missing_path_returns_none() {
        let resp = make_response(r#"{"data":{}}"#);
        let exts = vec![Extraction {
            var_name: "missing".to_string(),
            json_path: "data.nonexistent".to_string(),
        }];
        let vars = extract_variables(&resp, &exts);
        assert!(vars.get("missing").is_none());
    }

    #[test]
    fn test_extract_multiple() {
        let resp = make_response(r#"{"token":"tk_123","user":{"id":7}}"#);
        let exts = vec![
            Extraction {
                var_name: "token".to_string(),
                json_path: "token".to_string(),
            },
            Extraction {
                var_name: "uid".to_string(),
                json_path: "user.id".to_string(),
            },
        ];
        let vars = extract_variables(&resp, &exts);
        assert_eq!(vars.get("token").unwrap(), "tk_123");
        assert_eq!(vars.get("uid").unwrap(), "7");
    }

    #[test]
    fn test_extract_invalid_json() {
        let resp = make_response("not json at all");
        let exts = vec![Extraction {
            var_name: "x".to_string(),
            json_path: "key".to_string(),
        }];
        let vars = extract_variables(&resp, &exts);
        assert!(vars.is_empty());
    }

    #[test]
    fn test_extract_array_index() {
        let resp = make_response(r#"{"items":["a","b","c"]}"#);
        let exts = vec![Extraction {
            var_name: "first".to_string(),
            json_path: "items.0".to_string(),
        }];
        // 注意: serde_json 不支持数字索引访问数组，需要更复杂的实现
        // 这里测试预期行为
        let vars = extract_variables(&resp, &exts);
        // 当前实现不支持数组索引，所以应该返回空
        assert!(vars.get("first").is_none());
    }
}
