use anyhow::{bail, Context, Result};
use crate::models::{BodyType, HttpMethod, KeyValue, Request};

/// Parse a curl command string into a `Request`.
///
/// Supports common curl flags:
/// - `-X <METHOD>` / `--request <METHOD>` to set the HTTP method
/// - `-H <HEADER>` / `--header <HEADER>` to add headers (repeatable)
/// - `-d <DATA>` / `--data <DATA>` to set the request body
/// - The URL is taken from the first positional argument after `curl`
pub fn parse_curl(input: &str) -> Result<Request> {
    let args = tokenize(input)?;

    if args.is_empty() {
        bail!("Empty curl command");
    }

    let mut args = args.into_iter().peekable();

    // Expect first token to be "curl"
    match args.next().as_deref() {
        Some("curl") => {}
        other => bail!("Expected 'curl' as the first token, got {:?}", other),
    }

    let mut method: Option<HttpMethod> = None;
    let mut url: Option<String> = None;
    let mut headers: Vec<KeyValue> = Vec::new();
    let mut body = String::new();
    let mut body_type = BodyType::None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-X" | "--request" => {
                let val = args
                    .next()
                    .context("Missing value for method flag")?;
                method = Some(parse_method(&val)?);
            }
            "-H" | "--header" => {
                let val = args
                    .next()
                    .context("Missing value for header flag")?;
                let (key, value) = parse_header_value(&val);
                headers.push(KeyValue {
                    key,
                    value,
                    enabled: true,
                });
            }
            "-d" | "--data" | "--data-raw" => {
                let val = args
                    .next()
                    .context("Missing value for data flag")?;
                body = val;
                body_type = BodyType::Json;
            }
            _ => {
                // Positional argument — treat as URL if it looks like one
                if arg.starts_with("http://") || arg.starts_with("https://") {
                    url = Some(arg);
                }
                // Otherwise skip unknown flags (e.g. -s, -v, -k, --compressed, etc.)
            }
        }
    }

    let url = url.context("No URL found in curl command")?;

    // Infer method: if body is present and no explicit method, default to POST
    let method = method.unwrap_or_else(|| {
        if body.is_empty() {
            HttpMethod::GET
        } else {
            HttpMethod::POST
        }
    });

    Ok(Request {
        id: uuid::Uuid::new_v4().to_string(),
        name: format!("{} {}", method, url),
        method,
        url,
        headers,
        params: vec![],
        body,
        body_type,
        auth: crate::models::Auth::None,
    })
}

/// Tokenize a curl command string, respecting single and double quotes.
fn tokenize(input: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = input.trim().chars().peekable();
    let mut in_single_quote = false;
    let mut in_double_quote = false;

    while let Some(&ch) = chars.peek() {
        match ch {
            '\'' if !in_double_quote => {
                chars.next();
                in_single_quote = !in_single_quote;
            }
            '"' if !in_single_quote => {
                chars.next();
                in_double_quote = !in_double_quote;
            }
            '\\' if !in_single_quote => {
                chars.next();
                if let Some(escaped) = chars.next() {
                    current.push(escaped);
                }
            }
            ' ' | '\t' | '\n' if !in_single_quote && !in_double_quote => {
                chars.next();
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }
            _ => {
                chars.next();
                current.push(ch);
            }
        }
    }

    if in_single_quote {
        bail!("Unclosed single quote in curl command");
    }
    if in_double_quote {
        bail!("Unclosed double quote in curl command");
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    Ok(tokens)
}

fn parse_method(s: &str) -> Result<HttpMethod> {
    match s.to_uppercase().as_str() {
        "GET" => Ok(HttpMethod::GET),
        "POST" => Ok(HttpMethod::POST),
        "PUT" => Ok(HttpMethod::PUT),
        "DELETE" => Ok(HttpMethod::DELETE),
        "PATCH" => Ok(HttpMethod::PATCH),
        "HEAD" => Ok(HttpMethod::HEAD),
        "OPTIONS" => Ok(HttpMethod::OPTIONS),
        _ => bail!("Unknown HTTP method: {}", s),
    }
}

fn parse_header_value(s: &str) -> (String, String) {
    if let Some(pos) = s.find(':') {
        let key = s[..pos].trim().to_string();
        let value = s[pos + 1..].trim().to_string();
        (key, value)
    } else {
        (s.to_string(), String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_get() {
        let req = parse_curl("curl https://example.com").unwrap();
        assert_eq!(req.method, HttpMethod::GET);
        assert_eq!(req.url, "https://example.com");
        assert!(req.body.is_empty());
        assert!(req.headers.is_empty());
    }

    #[test]
    fn test_parse_post_with_body() {
        let req = parse_curl(r#"curl -X POST https://example.com -d '{"key":"value"}'"#).unwrap();
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.body, r#"{"key":"value"}"#);
        assert_eq!(req.body_type, BodyType::Json);
    }

    #[test]
    fn test_parse_single_header() {
        let req = parse_curl("curl -H 'Accept: application/json' https://example.com").unwrap();
        assert_eq!(req.method, HttpMethod::GET);
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.headers.len(), 1);
        assert_eq!(req.headers[0].key, "Accept");
        assert_eq!(req.headers[0].value, "application/json");
        assert!(req.headers[0].enabled);
    }

    #[test]
    fn test_parse_multiple_headers() {
        let req = parse_curl("curl -H 'A: 1' -H 'B: 2' https://example.com").unwrap();
        assert_eq!(req.method, HttpMethod::GET);
        assert_eq!(req.url, "https://example.com");
        assert_eq!(req.headers.len(), 2);
        assert_eq!(req.headers[0].key, "A");
        assert_eq!(req.headers[0].value, "1");
        assert_eq!(req.headers[1].key, "B");
        assert_eq!(req.headers[1].value, "2");
    }

    #[test]
    fn test_parse_put_method() {
        let req = parse_curl("curl -X PUT https://example.com").unwrap();
        assert_eq!(req.method, HttpMethod::PUT);
        assert_eq!(req.url, "https://example.com");
    }

    #[test]
    fn test_parse_invalid_input_empty() {
        let result = parse_curl("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_input_no_url() {
        let result = parse_curl("curl -X GET");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_invalid_input_not_curl() {
        let result = parse_curl("wget https://example.com");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_post_infers_method_from_body() {
        let req = parse_curl(r#"curl https://example.com -d '{"a":1}'"#).unwrap();
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.body, r#"{"a":1}"#);
    }

    #[test]
    fn test_parse_request_name() {
        let req = parse_curl("curl https://example.com").unwrap();
        assert_eq!(req.name, "GET https://example.com");
    }

    #[test]
    fn test_parse_with_line_continuations() {
        let input = "curl \\\n  -X POST \\\n  https://example.com";
        let req = parse_curl(input).unwrap();
        assert_eq!(req.method, HttpMethod::POST);
        assert_eq!(req.url, "https://example.com");
    }
}
