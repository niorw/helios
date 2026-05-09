use crate::models::{HistoryItem, Request, Response};
use anyhow::Result;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

pub const MAX_HISTORY_ITEMS: usize = 50;
pub const HISTORY_FILE_NAME: &str = "history.json";

/// Extended history entry with additional metadata
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HistoryEntry {
    pub id: String,
    pub timestamp: DateTime<Local>,
    pub request: Request,
    pub response_status: Option<u16>,
    pub response_size: Option<usize>,
    pub duration_ms: Option<u64>,
}

impl HistoryEntry {
    pub fn new(request: Request) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: Local::now(),
            request,
            response_status: None,
            response_size: None,
            duration_ms: None,
        }
    }

    pub fn from_history_item(item: &HistoryItem) -> Self {
        Self {
            id: item.id.clone(),
            timestamp: item.timestamp.into(),
            request: item.request.clone(),
            response_status: Some(item.response.status),
            response_size: Some(item.response.body.len()),
            duration_ms: Some(item.response.duration_ms),
        }
    }

    pub fn with_response(mut self, response: &Response) -> Self {
        self.response_status = Some(response.status);
        self.response_size = Some(response.body.len());
        self.duration_ms = Some(response.duration_ms);
        self
    }

    /// Format for display in history list
    pub fn display_name(&self) -> String {
        let method = format!("{}", self.request.method);
        let url = self
            .request
            .url
            .trim_start_matches("https://")
            .trim_start_matches("http://");
        let url_display = if url.len() > 40 {
            format!("{}...", &url[..37])
        } else {
            url.to_string()
        };

        let status = self
            .response_status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "--".to_string());

        format!("{} {} ({})", method, url_display, status)
    }
}

/// History manager for saving and loading request history
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HistoryManager {
    pub entries: Vec<HistoryEntry>,
}

impl HistoryManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a new entry, maintaining max size limit
    pub fn add_entry(&mut self, entry: HistoryEntry) {
        self.entries.insert(0, entry);
        if self.entries.len() > MAX_HISTORY_ITEMS {
            self.entries.truncate(MAX_HISTORY_ITEMS);
        }
    }

    /// Search entries by URL or method
    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        let query = query.to_lowercase();
        self
            .entries
            .iter()
            .filter(|e| {
                e.request.url.to_lowercase().contains(&query)
                    || format!("{}", e.request.method).to_lowercase().contains(&query)
            })
            .collect()
    }

    /// Get entry by ID
    pub fn get_by_id(&self, id: &str) -> Option<&HistoryEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Remove an entry by ID
    pub fn remove_by_id(&mut self, id: &str) -> bool {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            self.entries.remove(pos);
            true
        } else {
            false
        }
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get all entries (clone for UI access)
    pub fn get_all_entries(&self) -> Vec<HistoryEntry> {
        self.entries.clone()
    }
}

/// History storage with file persistence
pub struct HistoryStorage {
    data_dir: PathBuf,
}

impl HistoryStorage {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    fn history_file(&self) -> PathBuf {
        self.data_dir.join(HISTORY_FILE_NAME)
    }

    pub fn load(&self) -> Result<HistoryManager> {
        let path = self.history_file();
        if !path.exists() {
            return Ok(HistoryManager::new());
        }
        let content = fs::read_to_string(&path)?;
        let manager: HistoryManager = serde_json::from_str(&content).unwrap_or_default();
        Ok(manager)
    }

    pub fn save(&self, manager: &HistoryManager) -> Result<()> {
        let path = self.history_file();
        fs::create_dir_all(&self.data_dir)?;
        let content = serde_json::to_string_pretty(manager)?;
        fs::write(&path, content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{HttpMethod};

    fn create_test_request(url: &str, method: HttpMethod) -> Request {
        Request::new("Test", method, url)
    }

    fn create_test_response(status: u16, body: &str) -> Response {
        Response {
            status,
            status_text: "OK".to_string(),
            headers: std::collections::HashMap::new(),
            body: body.to_string(),
            duration_ms: 100,
            timestamp: Local::now(),
        }
    }

    #[test]
    fn test_history_entry_creation() {
        let request = create_test_request("https://api.example.com/users", HttpMethod::GET);
        let entry = HistoryEntry::new(request.clone());

        assert_eq!(entry.request.url, "https://api.example.com/users");
        assert_eq!(entry.request.method, HttpMethod::GET);
        assert!(entry.response_status.is_none());
        assert!(entry.response_size.is_none());
        assert!(!entry.id.is_empty());
    }

    #[test]
    fn test_history_entry_with_response() {
        let request = create_test_request("https://api.example.com/users", HttpMethod::GET);
        let response = create_test_response(200, "{\"data\": []}");  // 12 bytes
        let entry = HistoryEntry::new(request).with_response(&response);

        assert_eq!(entry.response_status, Some(200));
        assert_eq!(entry.response_size, Some(12));  // 实际长度是12
        assert_eq!(entry.duration_ms, Some(100));
    }

    #[test]
    fn test_history_entry_display_name() {
        let request = create_test_request("https://api.example.com/users", HttpMethod::GET);
        let response = create_test_response(200, "body");
        let entry = HistoryEntry::new(request).with_response(&response);

        let display = entry.display_name();
        assert!(display.contains("GET"));
        assert!(display.contains("api.example.com/users"));
        assert!(display.contains("200"));
    }

    #[test]
    fn test_history_manager_add_entry() {
        let mut manager = HistoryManager::new();
        let request = create_test_request("https://api.example.com/users", HttpMethod::GET);
        let entry = HistoryEntry::new(request);

        manager.add_entry(entry.clone());

        assert_eq!(manager.len(), 1);
        assert_eq!(manager.entries[0].id, entry.id);
    }

    #[test]
    fn test_history_manager_max_limit() {
        let mut manager = HistoryManager::new();

        // Add more than MAX_HISTORY_ITEMS
        for i in 0..MAX_HISTORY_ITEMS + 10 {
            let request = create_test_request(&format!("https://api{}.com", i), HttpMethod::GET);
            manager.add_entry(HistoryEntry::new(request));
        }

        assert_eq!(manager.len(), MAX_HISTORY_ITEMS);
    }

    #[test]
    fn test_history_manager_search() {
        let mut manager = HistoryManager::new();

        let req1 = create_test_request("https://api.example.com/users", HttpMethod::GET);
        let req2 = create_test_request("https://api.github.com/repos", HttpMethod::POST);
        let req3 = create_test_request("https://api.example.com/articles", HttpMethod::GET);  // 避免 "posts" 包含 "post"

        manager.add_entry(HistoryEntry::new(req1));
        manager.add_entry(HistoryEntry::new(req2));
        manager.add_entry(HistoryEntry::new(req3));

        let results = manager.search("example");
        assert_eq!(results.len(), 2);

        let results = manager.search("POST");  // 只匹配 req2 的 method
        assert_eq!(results.len(), 1);

        let results = manager.search("nonexistent");
        assert_eq!(results.len(), 0);
    }

    #[test]
    fn test_history_manager_get_by_id() {
        let mut manager = HistoryManager::new();
        let request = create_test_request("https://api.example.com", HttpMethod::GET);
        let entry = HistoryEntry::new(request);
        let id = entry.id.clone();

        manager.add_entry(entry);

        assert!(manager.get_by_id(&id).is_some());
        assert!(manager.get_by_id("nonexistent").is_none());
    }

    #[test]
    fn test_history_manager_remove_by_id() {
        let mut manager = HistoryManager::new();
        let request = create_test_request("https://api.example.com", HttpMethod::GET);
        let entry = HistoryEntry::new(request);
        let id = entry.id.clone();

        manager.add_entry(entry);
        assert_eq!(manager.len(), 1);

        assert!(manager.remove_by_id(&id));
        assert_eq!(manager.len(), 0);

        assert!(!manager.remove_by_id(&id));
    }

    #[test]
    fn test_history_manager_clear() {
        let mut manager = HistoryManager::new();
        
        for i in 0..5 {
            let request = create_test_request(&format!("https://api{}.com", i), HttpMethod::GET);
            manager.add_entry(HistoryEntry::new(request));
        }

        assert_eq!(manager.len(), 5);
        manager.clear();
        assert!(manager.is_empty());
    }

    #[test]
    fn test_history_storage_save_and_load() {
        use std::env;
        use std::fs;
        
        let temp_dir = env::temp_dir().join("helios_test_history");
        let _ = fs::remove_dir_all(&temp_dir);
        
        let storage = HistoryStorage::new(temp_dir.clone());
        let mut manager = HistoryManager::new();
        
        let request = create_test_request("https://api.example.com/users", HttpMethod::GET);
        let response = create_test_response(200, "{\"data\": []}");
        let entry = HistoryEntry::new(request).with_response(&response);
        manager.add_entry(entry);

        // Save
        storage.save(&manager).unwrap();

        // Load
        let loaded = storage.load().unwrap();
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.entries[0].request.url, "https://api.example.com/users");
        assert_eq!(loaded.entries[0].response_status, Some(200));

        // Cleanup
        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_history_entry_from_history_item() {
        let request = create_test_request("https://api.example.com", HttpMethod::POST);
        let response = create_test_response(201, "created");
        let item = HistoryItem {
            id: uuid::Uuid::new_v4().to_string(),
            request: request.clone(),
            response,
            timestamp: Local::now(),
        };

        let entry = HistoryEntry::from_history_item(&item);
        assert_eq!(entry.request.url, "https://api.example.com");
        assert_eq!(entry.response_status, Some(201));
    }
}
