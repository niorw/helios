/// 请求依赖链模块
/// 支持请求间变量传递，依赖请求的响应自动注入到后续请求
use crate::models::{KeyValue, Request, Response};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ScenarioStep {
    /// 请求在集合中的索引
    pub request_index: usize,
    /// 步骤间延迟毫秒
    pub delay_ms: u64,
    /// 失败时跳过后续步骤
    pub skip_on_fail: bool,
    /// 依赖的步骤索引，依赖步骤的响应变量会注入到当前请求
    pub depends_on: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct StepResult {
    pub step_index: usize,
    pub request_index: usize,
    pub response: Option<Response>,
    pub skipped: bool,
    pub error: Option<String>,
    pub extracted_vars: HashMap<String, String>,
}

/// 从响应中提取常用变量
/// 自动提取: status, body 中的顶层字段
pub fn extract_response_vars(resp: &Response, prefix: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    // status
    vars.insert(format!("{}.status", prefix), resp.status.to_string());

    // 尝试解析 JSON body 提取顶层字段
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(&resp.body) {
        if let Some(obj) = json.as_object() {
            for (key, value) in obj {
                let var_name = format!("{}.{}", prefix, key);
                let var_value = match value {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    _ => value.to_string(),
                };
                vars.insert(var_name, var_value);
            }
        }
    }

    vars
}

/// 将提取的变量应用到请求中
pub fn apply_vars_to_request(
    req: &Request,
    vars: &HashMap<String, String>,
) -> Request {
    let mut resolved = req.clone();

    resolved.url = apply_vars_to_string(&resolved.url, vars);
    for h in &mut resolved.headers {
        h.value = apply_vars_to_string(&h.value, vars);
    }
    resolved.body = apply_vars_to_string(&resolved.body, vars);

    resolved
}

/// 替换字符串中的 {{prefix.field}} 变量
fn apply_vars_to_string(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (key, value) in vars {
        let pattern = format!("{{{{{}}}}}", key);
        result = result.replace(&pattern, value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;

    fn make_response(status: u16, body: &str) -> Response {
        Response {
            status,
            status_text: "OK".into(),
            headers: HashMap::new(),
            body: body.into(),
            duration_ms: 100,
            timestamp: chrono::Local::now(),
        }
    }

    #[test]
    fn test_extract_status_var() {
        let resp = make_response(200, "{}");
        let vars = extract_response_vars(&resp, "resp");
        assert_eq!(vars.get("resp.status").unwrap(), "200");
    }

    #[test]
    fn test_extract_json_fields() {
        let resp = make_response(200, r#"{"token":"abc123","user_id":42}"#);
        let vars = extract_response_vars(&resp, "login");
        assert_eq!(vars.get("login.token").unwrap(), "abc123");
        assert_eq!(vars.get("login.user_id").unwrap(), "42");
    }

    #[test]
    fn test_extract_nested_json_returns_string() {
        let resp = make_response(200, r#"{"data":{"key":"value"}}"#);
        let vars = extract_response_vars(&resp, "resp");
        // 嵌套对象应该被序列化为字符串
        assert!(vars.contains_key("resp.data"));
    }

    #[test]
    fn test_apply_vars_to_url() {
        let mut req = Request::default();
        req.url = "https://api.com/users/{{user.id}}".into();
        let mut vars = HashMap::new();
        vars.insert("user.id".into(), "42".into());
        let resolved = apply_vars_to_request(&req, &vars);
        assert_eq!(resolved.url, "https://api.com/users/42");
    }

    #[test]
    fn test_apply_vars_to_header() {
        let mut req = Request::default();
        req.headers = vec![KeyValue {
            key: "Authorization".into(),
            value: "Bearer {{login.token}}".into(),
            enabled: true,
        }];
        let mut vars = HashMap::new();
        vars.insert("login.token".into(), "xyz789".into());
        let resolved = apply_vars_to_request(&req, &vars);
        assert_eq!(resolved.headers[0].value, "Bearer xyz789");
    }

    #[test]
    fn test_apply_vars_to_body() {
        let mut req = Request::default();
        req.body = r#"{"user_id":{{login.user_id}}}"#.into();
        let mut vars = HashMap::new();
        vars.insert("login.user_id".into(), "42".into());
        let resolved = apply_vars_to_request(&req, &vars);
        assert_eq!(resolved.body, r#"{"user_id":42}"#);
    }

    #[test]
    fn test_scenario_step_with_depends_on() {
        let step = ScenarioStep {
            request_index: 1,
            delay_ms: 0,
            skip_on_fail: false,
            depends_on: Some(0),
        };
        assert_eq!(step.depends_on, Some(0));
    }

    #[test]
    fn test_full_dependency_chain() {
        // Step 0: login request, returns {"token":"abc"}
        let login_resp = make_response(200, r#"{"token":"abc"}"#);
        let vars = extract_response_vars(&login_resp, "login");

        // Step 1: use token from login
        let mut protected_req = Request::default();
        protected_req.url = "https://api.com/me".into();
        protected_req.headers = vec![KeyValue {
            key: "Authorization".into(),
            value: "Bearer {{login.token}}".into(),
            enabled: true,
        }];

        let resolved = apply_vars_to_request(&protected_req, &vars);
        assert_eq!(resolved.headers[0].value, "Bearer abc");
    }
}
