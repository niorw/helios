use serde_json::Value;

/// Parse a JSON string and query it using a simple JSONPath expression.
///
/// Supports:
/// - Dot paths: "data.user.name"
/// - Array indices: "items.0"
/// - Wildcards: "data.*" (returns a JSON array of all values)
pub fn parse_jsonpath(json_str: &str, path: &str) -> Option<String> {
    let value: Value = serde_json::from_str(json_str).ok()?;
    resolve(&value, path)
}

fn resolve(value: &Value, path: &str) -> Option<String> {
    if path.is_empty() {
        return Some(value.to_string());
    }

    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for (i, part) in parts.iter().enumerate() {
        if *part == "*" {
            // Wildcard: collect all values at this level
            let remaining = if i + 1 < parts.len() {
                Some(parts[i + 1..].join("."))
            } else {
                None
            };

            let results: Vec<Value> = match current {
                Value::Object(map) => {
                    if let Some(ref rem) = remaining {
                        map.values()
                            .filter_map(|v| {
                                resolve_inner(v, rem)
                            })
                            .collect()
                    } else {
                        map.values().cloned().collect()
                    }
                }
                Value::Array(arr) => {
                    if let Some(ref rem) = remaining {
                        arr.iter()
                            .filter_map(|v| {
                                resolve_inner(v, rem)
                            })
                            .collect()
                    } else {
                        arr.clone()
                    }
                }
                _ => return None,
            };

            return serde_json::to_string(&results).ok();
        }

        current = match current {
            Value::Object(map) => map.get(*part)?,
            Value::Array(arr) => {
                let idx: usize = part.parse().ok()?;
                arr.get(idx)?
            }
            _ => return None,
        };
    }

    Some(value_to_string(current))
}

fn resolve_inner(value: &Value, path: &str) -> Option<Value> {
    if path.is_empty() {
        return Some(value.clone());
    }

    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for (i, part) in parts.iter().enumerate() {
        if *part == "*" {
            let remaining = if i + 1 < parts.len() {
                Some(parts[i + 1..].join("."))
            } else {
                None
            };

            let results: Vec<Value> = match current {
                Value::Object(map) => {
                    if let Some(ref rem) = remaining {
                        map.values()
                            .filter_map(|v| resolve_inner(v, rem))
                            .collect()
                    } else {
                        map.values().cloned().collect()
                    }
                }
                Value::Array(arr) => {
                    if let Some(ref rem) = remaining {
                        arr.iter()
                            .filter_map(|v| resolve_inner(v, rem))
                            .collect()
                    } else {
                        arr.clone()
                    }
                }
                _ => return None,
            };

            return Some(Value::Array(results));
        }

        current = match current {
            Value::Object(map) => map.get(*part)?,
            Value::Array(arr) => {
                let idx: usize = part.parse().ok()?;
                arr.get(idx)?
            }
            _ => return None,
        };
    }

    Some(current.clone())
}

fn value_to_string(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        // For objects and arrays, return JSON
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_JSON: &str = r#"{
        "data": {
            "user": {
                "name": "Alice",
                "age": 30,
                "active": true
            },
            "items": ["apple", "banana", "cherry"]
        },
        "status": "ok"
    }"#;

    #[test]
    fn test_simple_dot_path() {
        let result = parse_jsonpath(TEST_JSON, "status");
        assert_eq!(result, Some("ok".to_string()));
    }

    #[test]
    fn test_nested_dot_path() {
        let result = parse_jsonpath(TEST_JSON, "data.user.name");
        assert_eq!(result, Some("Alice".to_string()));
    }

    #[test]
    fn test_array_index() {
        let result = parse_jsonpath(TEST_JSON, "data.items.1");
        assert_eq!(result, Some("banana".to_string()));
    }

    #[test]
    fn test_wildcard_returns_array() {
        let result = parse_jsonpath(TEST_JSON, "data.*");
        assert!(result.is_some());
        let parsed: Vec<serde_json::Value> =
            serde_json::from_str(&result.unwrap()).unwrap();
        assert_eq!(parsed.len(), 2); // user object + items array
    }

    #[test]
    fn test_invalid_path_returns_none() {
        let result = parse_jsonpath(TEST_JSON, "data.nonexistent.field");
        assert_eq!(result, None);
    }

    #[test]
    fn test_invalid_json_returns_none() {
        let result = parse_jsonpath("not json", "data");
        assert_eq!(result, None);
    }
}
