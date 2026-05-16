use std::collections::HashMap;

pub fn replace_variables(text: &str, vars: &HashMap<String, String>) -> String {
    let mut result = text.to_string();
    for (key, value) in vars {
        let pattern = format!("{{{{{}}}}}", key);
        result = result.replace(&pattern, value);

        let pattern2 = format!("{{{{ {} }}}}", key);
        result = result.replace(&pattern2, value);
    }
    result
}

/// 解析内置动态变量: {{$timestamp}}, {{$uuid}}, {{$randomInt}}, {{$randomStr}}, {{$date}}
pub fn resolve_builtin_variables(text: &str) -> String {
    let mut result = text.to_string();

    // {{$timestamp}} -> 毫秒时间戳
    if result.contains("{{$timestamp}}") {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        result = result.replace("{{$timestamp}}", &ts.to_string());
    }

    // {{$uuid}} -> UUID v4
    if result.contains("{{$uuid}}") {
        result = result.replace("{{$uuid}}", &uuid::Uuid::new_v4().to_string());
    }

    // {{$randomInt}} -> 0-9999 随机整数
    if result.contains("{{$randomInt}}") {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let rand_int = RandomState::new().build_hasher().finish() % 10000;
        result = result.replace("{{$randomInt}}", &rand_int.to_string());
    }

    // {{$randomStr}} -> 8位随机字母数字
    if result.contains("{{$randomStr}}") {
        let s: String = (0..8)
            .map(|_| {
                use std::collections::hash_map::RandomState;
                use std::hash::{BuildHasher, Hasher};
                let b = (RandomState::new().build_hasher().finish() % 36) as u8;
                if b < 10 {
                    (b'0' + b) as char
                } else {
                    (b'a' + b - 10) as char
                }
            })
            .collect();
        result = result.replace("{{$randomStr}}", &s);
    }

    // {{$date}} -> YYYY-MM-DD
    if result.contains("{{$date}}") {
        let now = chrono::Local::now();
        result = result.replace("{{$date}}", &now.format("%Y-%m-%d").to_string());
    }

    result
}
