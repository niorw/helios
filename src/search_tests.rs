/// 请求搜索测试
/// 测试 search_requests 逻辑（独立于 App 结构体）
#[cfg(test)]
mod search_tests {
    use crate::models::*;

    fn test_collections() -> Vec<Collection> {
        vec![
            Collection {
                id: "1".into(),
                name: "API".into(),
                requests: vec![
                    Request::new("Get Users", HttpMethod::GET, "https://api.example.com/users"),
                    Request::new("Create User", HttpMethod::POST, "https://api.example.com/users"),
                ],
                created_at: chrono::Local::now(),
            },
            Collection {
                id: "2".into(),
                name: "Auth".into(),
                requests: vec![
                    Request::new("Login", HttpMethod::POST, "https://auth.example.com/login"),
                ],
                created_at: chrono::Local::now(),
            },
        ]
    }

    /// 搜索请求的纯函数版本（与 App::search_requests 逻辑相同）
    fn search_requests(query: &str, collections: &[Collection]) -> Vec<(usize, usize)> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();
        for (ci, col) in collections.iter().enumerate() {
            for (ri, req) in col.requests.iter().enumerate() {
                if req.name.to_lowercase().contains(&query_lower)
                    || req.url.to_lowercase().contains(&query_lower)
                {
                    results.push((ci, ri));
                }
            }
        }
        results
    }

    #[test]
    fn test_search_by_name() {
        let cols = test_collections();
        let results = search_requests("users", &cols);
        assert_eq!(results.len(), 2); // "Get Users" and "Create User"
    }

    #[test]
    fn test_search_by_url() {
        let cols = test_collections();
        let results = search_requests("auth.example", &cols);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], (1, 0)); // Auth collection, Login request
    }

    #[test]
    fn test_search_case_insensitive() {
        let cols = test_collections();
        let results = search_requests("USERS", &cols);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_no_match() {
        let cols = test_collections();
        let results = search_requests("nonexistent", &cols);
        assert!(results.is_empty());
    }
}
