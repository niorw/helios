/// 前置脚本模块
/// 支持声明式脚本，在请求发送前自动执行
/// 语法:
///   @header Key=Value  -- 注入 header
///   @set varname=value -- 设置变量
///   @delay 100         -- 延迟毫秒
use crate::models::{KeyValue, Request};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScriptAction {
    /// 注入 header: @header X-Timestamp={{$timestamp}}
    SetHeader { key: String, value: String },
    /// 设置变量: @set token=abc123
    SetVar { name: String, value: String },
    /// 延迟: @delay 1000
    DelayMs(u64),
}

/// 解析前置脚本文本为动作列表
pub fn parse_pre_script(script: &str) -> Vec<ScriptAction> {
    script
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| {
            if let Some(rest) = line.strip_prefix("@header ") {
                let parts: Vec<&str> = rest.splitn(2, '=').collect();
                if parts.len() == 2 {
                    Some(ScriptAction::SetHeader {
                        key: parts[0].trim().to_string(),
                        value: parts[1].trim().to_string(),
                    })
                } else {
                    None
                }
            } else if let Some(rest) = line.strip_prefix("@set ") {
                let parts: Vec<&str> = rest.splitn(2, '=').collect();
                if parts.len() == 2 {
                    Some(ScriptAction::SetVar {
                        name: parts[0].trim().to_string(),
                        value: parts[1].trim().to_string(),
                    })
                } else {
                    None
                }
            } else if let Some(rest) = line.strip_prefix("@delay ") {
                rest.trim().parse::<u64>().ok().map(ScriptAction::DelayMs)
            } else {
                None
            }
        })
        .collect()
}

/// 对请求执行前置脚本动作
pub fn apply_pre_script(
    req: &mut Request,
    actions: &[ScriptAction],
) -> HashMap<String, String> {
    let mut vars = HashMap::new();

    for action in actions {
        match action {
            ScriptAction::SetHeader { key, value } => {
                let resolved_value = crate::utils::resolve_builtin_variables(value);
                req.headers.push(KeyValue {
                    key: key.clone(),
                    value: resolved_value,
                    enabled: true,
                });
            }
            ScriptAction::SetVar { name, value } => {
                vars.insert(name.clone(), value.clone());
            }
            ScriptAction::DelayMs(_) => {
                // 延迟在实际发送时处理，这里只记录
            }
        }
    }

    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_set_header() {
        let script = "@header X-Token=abc123";
        let actions = parse_pre_script(script);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            ScriptAction::SetHeader {
                key: "X-Token".into(),
                value: "abc123".into(),
            }
        );
    }

    #[test]
    fn test_parse_set_var() {
        let script = "@set auth_token=xyz789";
        let actions = parse_pre_script(script);
        assert_eq!(actions.len(), 1);
        assert_eq!(
            actions[0],
            ScriptAction::SetVar {
                name: "auth_token".into(),
                value: "xyz789".into(),
            }
        );
    }

    #[test]
    fn test_parse_delay() {
        let script = "@delay 500";
        let actions = parse_pre_script(script);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0], ScriptAction::DelayMs(500));
    }

    #[test]
    fn test_parse_multiple_actions() {
        let script = "@header X-Ts=123\n@set key=val\n@delay 100";
        let actions = parse_pre_script(script);
        assert_eq!(actions.len(), 3);
    }

    #[test]
    fn test_parse_comments_and_empty_lines() {
        let script = "# this is a comment\n\n@header A=B\n# another comment\n";
        let actions = parse_pre_script(script);
        assert_eq!(actions.len(), 1);
    }

    #[test]
    fn test_apply_set_header() {
        let mut req = Request::default();
        let actions = vec![ScriptAction::SetHeader {
            key: "X-Custom".into(),
            value: "test-value".into(),
        }];
        apply_pre_script(&mut req, &actions);
        assert!(req.headers.iter().any(|h| h.key == "X-Custom" && h.value == "test-value"));
    }

    #[test]
    fn test_apply_set_var() {
        let mut req = Request::default();
        let actions = vec![ScriptAction::SetVar {
            name: "token".into(),
            value: "abc".into(),
        }];
        let vars = apply_pre_script(&mut req, &actions);
        assert_eq!(vars.get("token").unwrap(), "abc");
    }
}
