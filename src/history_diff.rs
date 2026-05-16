/// 历史对比模块
/// 对比两个历史请求的差异
use crate::models::Request;

#[derive(Debug, Clone, PartialEq)]
pub enum FieldDiff {
    Method { old: String, new: String },
    Url { old: String, new: String },
    Header { key: String, old_value: Option<String>, new_value: Option<String> },
    Body { old: String, new: String },
    Param { key: String, old_value: Option<String>, new_value: Option<String> },
}

/// 对比两个请求，返回差异列表
pub fn diff_requests(old: &Request, new: &Request) -> Vec<FieldDiff> {
    let mut diffs = Vec::new();

    // 方法
    if old.method != new.method {
        diffs.push(FieldDiff::Method {
            old: format!("{}", old.method),
            new: format!("{}", new.method),
        });
    }

    // URL
    if old.url != new.url {
        diffs.push(FieldDiff::Url {
            old: old.url.clone(),
            new: new.url.clone(),
        });
    }

    // Headers
    let old_headers: std::collections::HashMap<_, _> =
        old.headers.iter().map(|h| (&h.key, &h.value)).collect();
    let new_headers: std::collections::HashMap<_, _> =
        new.headers.iter().map(|h| (&h.key, &h.value)).collect();

    for (key, old_val) in &old_headers {
        if let Some(new_val) = new_headers.get(key) {
            if old_val != new_val {
                diffs.push(FieldDiff::Header {
                    key: key.to_string(),
                    old_value: Some(old_val.to_string()),
                    new_value: Some(new_val.to_string()),
                });
            }
        } else {
            diffs.push(FieldDiff::Header {
                key: key.to_string(),
                old_value: Some(old_val.to_string()),
                new_value: None,
            });
        }
    }
    for (key, new_val) in &new_headers {
        if !old_headers.contains_key(key) {
            diffs.push(FieldDiff::Header {
                key: key.to_string(),
                old_value: None,
                new_value: Some(new_val.to_string()),
            });
        }
    }

    // Body
    if old.body != new.body {
        diffs.push(FieldDiff::Body {
            old: old.body.clone(),
            new: new.body.clone(),
        });
    }

    // Params
    let old_params: std::collections::HashMap<_, _> =
        old.params.iter().map(|p| (&p.key, &p.value)).collect();
    let new_params: std::collections::HashMap<_, _> =
        new.params.iter().map(|p| (&p.key, &p.value)).collect();

    for (key, old_val) in &old_params {
        if let Some(new_val) = new_params.get(key) {
            if old_val != new_val {
                diffs.push(FieldDiff::Param {
                    key: key.to_string(),
                    old_value: Some(old_val.to_string()),
                    new_value: Some(new_val.to_string()),
                });
            }
        } else {
            diffs.push(FieldDiff::Param {
                key: key.to_string(),
                old_value: Some(old_val.to_string()),
                new_value: None,
            });
        }
    }
    for (key, new_val) in &new_params {
        if !old_params.contains_key(key) {
            diffs.push(FieldDiff::Param {
                key: key.to_string(),
                old_value: None,
                new_value: Some(new_val.to_string()),
            });
        }
    }

    diffs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::*;

    fn make_request(method: HttpMethod, url: &str, body: &str) -> Request {
        let mut req = Request::default();
        req.method = method;
        req.url = url.to_string();
        req.body = body.to_string();
        req
    }

    #[test]
    fn test_diff_identical_requests() {
        let r1 = make_request(HttpMethod::GET, "https://api.com", "");
        let r2 = make_request(HttpMethod::GET, "https://api.com", "");
        assert!(diff_requests(&r1, &r2).is_empty());
    }

    #[test]
    fn test_diff_different_method() {
        let r1 = make_request(HttpMethod::GET, "https://api.com", "");
        let r2 = make_request(HttpMethod::POST, "https://api.com", "");
        let diffs = diff_requests(&r1, &r2);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(diffs[0], FieldDiff::Method { .. }));
    }

    #[test]
    fn test_diff_different_url() {
        let r1 = make_request(HttpMethod::GET, "https://api.com/v1", "");
        let r2 = make_request(HttpMethod::GET, "https://api.com/v2", "");
        let diffs = diff_requests(&r1, &r2);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(diffs[0], FieldDiff::Url { .. }));
    }

    #[test]
    fn test_diff_different_body() {
        let r1 = make_request(HttpMethod::POST, "https://api.com", r#"{"a":1}"#);
        let r2 = make_request(HttpMethod::POST, "https://api.com", r#"{"a":2}"#);
        let diffs = diff_requests(&r1, &r2);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(diffs[0], FieldDiff::Body { .. }));
    }

    #[test]
    fn test_diff_header_changed() {
        let mut r1 = make_request(HttpMethod::GET, "https://api.com", "");
        r1.headers = vec![KeyValue { key: "Accept".into(), value: "json".into(), enabled: true }];
        let mut r2 = make_request(HttpMethod::GET, "https://api.com", "");
        r2.headers = vec![KeyValue { key: "Accept".into(), value: "html".into(), enabled: true }];
        let diffs = diff_requests(&r1, &r2);
        assert_eq!(diffs.len(), 1);
        assert!(matches!(diffs[0], FieldDiff::Header { .. }));
    }

    #[test]
    fn test_diff_header_added() {
        let r1 = make_request(HttpMethod::GET, "https://api.com", "");
        let mut r2 = make_request(HttpMethod::GET, "https://api.com", "");
        r2.headers = vec![KeyValue { key: "X-New".into(), value: "val".into(), enabled: true }];
        let diffs = diff_requests(&r1, &r2);
        assert_eq!(diffs.len(), 1);
        match &diffs[0] {
            FieldDiff::Header { key, old_value, new_value } => {
                assert_eq!(key, "X-New");
                assert!(old_value.is_none());
                assert!(new_value.is_some());
            }
            _ => panic!("expected Header diff"),
        }
    }
}
