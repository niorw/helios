/// 响应断言模块
/// 支持对 HTTP 响应进行自动化断言检查
use crate::models::Response;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AssertOp {
    Equals,
    NotEquals,
    Contains,
    GreaterThan,
    LessThan,
    Exists,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Assertion {
    /// 断言路径: "status", "body.key", "header.Content-Type"
    pub path: String,
    pub operator: AssertOp,
    pub expected: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AssertResult {
    Pass,
    Fail(String),
}

/// 对响应执行单条断言
pub fn assert_response(resp: &Response, assertion: &Assertion) -> AssertResult {
    let actual = extract_value(resp, &assertion.path);

    match &assertion.operator {
        AssertOp::Equals => match actual {
            Some(val) if val == assertion.expected => AssertResult::Pass,
            Some(val) => AssertResult::Fail(format!(
                "expected '{}', got '{}'",
                assertion.expected, val
            )),
            None => AssertResult::Fail(format!("path '{}' not found", assertion.path)),
        },
        AssertOp::NotEquals => match actual {
            Some(val) if val != assertion.expected => AssertResult::Pass,
            Some(val) => AssertResult::Fail(format!(
                "expected not '{}', but got '{}'",
                assertion.expected, val
            )),
            None => AssertResult::Pass, // 不存在也算不等于
        },
        AssertOp::Contains => match actual {
            Some(val) if val.contains(&assertion.expected) => AssertResult::Pass,
            Some(val) => AssertResult::Fail(format!(
                "'{}' does not contain '{}'",
                val, assertion.expected
            )),
            None => AssertResult::Fail(format!("path '{}' not found", assertion.path)),
        },
        AssertOp::GreaterThan => match actual.and_then(|v| v.parse::<f64>().ok()) {
            Some(val) if val > assertion.expected.parse::<f64>().unwrap_or(0.0) => {
                AssertResult::Pass
            }
            Some(val) => AssertResult::Fail(format!(
                "{} is not greater than {}",
                val, assertion.expected
            )),
            None => AssertResult::Fail(format!("path '{}' not found or not numeric", assertion.path)),
        },
        AssertOp::LessThan => match actual.and_then(|v| v.parse::<f64>().ok()) {
            Some(val) if val < assertion.expected.parse::<f64>().unwrap_or(0.0) => {
                AssertResult::Pass
            }
            Some(val) => AssertResult::Fail(format!(
                "{} is not less than {}",
                val, assertion.expected
            )),
            None => AssertResult::Fail(format!("path '{}' not found or not numeric", assertion.path)),
        },
        AssertOp::Exists => {
            if actual.is_some() {
                AssertResult::Pass
            } else {
                AssertResult::Fail(format!("path '{}' does not exist", assertion.path))
            }
        }
    }
}

/// 从响应中提取值
/// 支持路径: "status", "body.key", "body.nested.key", "header.Header-Name"
fn extract_value(resp: &Response, path: &str) -> Option<String> {
    let parts: Vec<&str> = path.split('.').collect();

    match parts.first().copied() {
        Some("status") => Some(resp.status.to_string()),
        Some("header") if parts.len() >= 2 => {
            let name = &path[7..]; // skip "header."
            resp.headers.get(name).cloned()
        }
        Some("body") if parts.len() >= 2 => {
            // 简易 JSONPath: body.key1.key2
            let json: serde_json::Value = serde_json::from_str(&resp.body).ok()?;
            let mut current = &json;
            for key in &parts[1..] {
                current = current.get(key)?;
            }
            Some(json_value_to_string(current))
        }
        _ => None,
    }
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

    fn make_response(status: u16, body: &str) -> Response {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Response {
            status,
            status_text: "OK".to_string(),
            headers,
            body: body.to_string(),
            duration_ms: 100,
            timestamp: chrono::Local::now(),
        }
    }

    #[test]
    fn test_assert_status_equals() {
        let resp = make_response(200, "{}");
        let a = Assertion {
            path: "status".to_string(),
            operator: AssertOp::Equals,
            expected: "200".to_string(),
        };
        assert_eq!(assert_response(&resp, &a), AssertResult::Pass);
    }

    #[test]
    fn test_assert_body_contains() {
        let resp = make_response(200, r#"{"message":"success"}"#);
        let a = Assertion {
            path: "body.message".to_string(),
            operator: AssertOp::Contains,
            expected: "succ".to_string(),
        };
        assert_eq!(assert_response(&resp, &a), AssertResult::Pass);
    }

    #[test]
    fn test_assert_header_exists() {
        let resp = make_response(200, "{}");
        let a = Assertion {
            path: "header.Content-Type".to_string(),
            operator: AssertOp::Exists,
            expected: "".to_string(),
        };
        assert_eq!(assert_response(&resp, &a), AssertResult::Pass);
    }

    #[test]
    fn test_assert_numeric_compare() {
        let resp = make_response(200, r#"{"count":42}"#);
        let a = Assertion {
            path: "body.count".to_string(),
            operator: AssertOp::GreaterThan,
            expected: "10".to_string(),
        };
        assert_eq!(assert_response(&resp, &a), AssertResult::Pass);
    }

    #[test]
    fn test_assert_fail_message() {
        let resp = make_response(404, "{}");
        let a = Assertion {
            path: "status".to_string(),
            operator: AssertOp::Equals,
            expected: "200".to_string(),
        };
        match assert_response(&resp, &a) {
            AssertResult::Fail(msg) => assert!(msg.contains("200")),
            _ => panic!("expected Fail"),
        }
    }

    #[test]
    fn test_assert_not_equals() {
        let resp = make_response(200, "{}");
        let a = Assertion {
            path: "status".to_string(),
            operator: AssertOp::NotEquals,
            expected: "404".to_string(),
        };
        assert_eq!(assert_response(&resp, &a), AssertResult::Pass);
    }

    #[test]
    fn test_assert_nested_body() {
        let resp = make_response(200, r#"{"data":{"user":{"name":"alice"}}}"#);
        let a = Assertion {
            path: "body.data.user.name".to_string(),
            operator: AssertOp::Equals,
            expected: "alice".to_string(),
        };
        assert_eq!(assert_response(&resp, &a), AssertResult::Pass);
    }
}
