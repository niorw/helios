//! 文件系统存储后端
//!
//! Phase 0 - F01: 集合即目录，请求即文件。
//! 目录结构:
//!   collections/
//!     my-collection/
//!       collection.yml          (集合元信息)
//!       get-users.helios.yml   (请求1)
//!       create-order.helios.yml(请求2)
//!     another-collection/
//!       ...

use crate::helios_format::{
    helios_yml_to_request, load_collection_from_dir, request_to_helios_yml, save_helios_yml,
    HeliosInfo, HeliosYml,
};
use crate::models::{AppData, Collection, Request};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 集合目录的元信息文件
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CollectionYml {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

/// 文件系统存储后端
pub struct FileStorage {
    /// 集合根目录
    base_dir: PathBuf,
}

impl FileStorage {
    /// 创建 FileStorage，指定集合根目录
    pub fn new(base_dir: impl Into<PathBuf>) -> Result<Self> {
        let base_dir = base_dir.into();
        fs::create_dir_all(&base_dir)
            .with_context(|| format!("无法创建集合目录: {}", base_dir.display()))?;
        Ok(Self { base_dir })
    }

    /// 使用默认路径（XDG data dir 下的 helios/collections）
    pub fn default_path() -> Result<PathBuf> {
        let proj_dirs = directories::ProjectDirs::from("com", "helios", "helios")
            .context("无法确定项目数据目录")?;
        Ok(proj_dirs.data_dir().join("collections"))
    }

    /// 使用默认路径创建
    pub fn with_default_path() -> Result<Self> {
        let path = Self::default_path()?;
        Self::new(path)
    }

    /// 列出所有集合目录名
    pub fn list_collections(&self) -> Result<Vec<String>> {
        let mut names = Vec::new();
        let entries = fs::read_dir(&self.base_dir)
            .with_context(|| format!("无法读取目录: {}", self.base_dir.display()))?;
        for entry in entries {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                if let Some(name) = entry.file_name().to_str() {
                    names.push(name.to_string());
                }
            }
        }
        names.sort();
        Ok(names)
    }

    /// 加载指定集合
    pub fn load_collection(&self, name: &str) -> Result<Collection> {
        let dir = self.base_dir.join(name);
        if !dir.exists() {
            anyhow::bail!("集合不存在: {}", name);
        }
        load_collection_from_dir(&dir)
    }

    /// 加载所有集合
    pub fn load_all_collections(&self) -> Result<Vec<Collection>> {
        let names = self.list_collections()?;
        let mut collections = Vec::new();
        for name in names {
            match self.load_collection(&name) {
                Ok(col) => collections.push(col),
                Err(e) => eprintln!("警告: 加载集合 '{}' 失败: {}", name, e),
            }
        }
        Ok(collections)
    }

    /// 加载为 AppData（兼容现有 Storage trait）
    pub fn load_app_data(&self) -> Result<AppData> {
        let collections = self.load_all_collections()?;
        Ok(AppData {
            collections,
            environments: vec![],
            history: vec![],
            active_env_id: None,
        })
    }

    /// 创建新集合
    pub fn create_collection(&self, name: &str) -> Result<PathBuf> {
        let dir = self.base_dir.join(name);
        if dir.exists() {
            anyhow::bail!("集合已存在: {}", name);
        }
        fs::create_dir_all(&dir)?;
        let col_yml = CollectionYml {
            name: name.to_string(),
            description: String::new(),
        };
        let content = serde_yaml::to_string(&col_yml)?;
        fs::write(dir.join("collection.yml"), content)?;
        Ok(dir)
    }

    /// 删除集合
    pub fn delete_collection(&self, name: &str) -> Result<()> {
        let dir = self.base_dir.join(name);
        if !dir.exists() {
            anyhow::bail!("集合不存在: {}", name);
        }
        fs::remove_dir_all(&dir)?;
        Ok(())
    }

    /// 保存请求到集合
    pub fn save_request(&self, collection_name: &str, req: &Request) -> Result<()> {
        let dir = self.base_dir.join(collection_name);
        if !dir.exists() {
            anyhow::bail!("集合不存在: {}", collection_name);
        }
        let yml = request_to_helios_yml(req)?;
        // 文件名: {请求名slug}.helios.yml
        let slug = slugify(&req.name);
        let filename = format!("{}.helios.yml", slug);
        save_helios_yml(&yml, &dir.join(filename))
    }

    /// 删除请求
    pub fn delete_request(&self, collection_name: &str, req_name: &str) -> Result<()> {
        let dir = self.base_dir.join(collection_name);
        let slug = slugify(req_name);
        let filename = format!("{}.helios.yml", slug);
        let path = dir.join(&filename);
        if path.exists() {
            fs::remove_file(path)?;
        } else {
            // 尝试模糊匹配
            let entries = fs::read_dir(&dir)?;
            for entry in entries {
                let entry = entry?;
                let fname = entry.file_name();
                let fname_str = fname.to_string_lossy();
                if fname_str.ends_with(".helios.yml") {
                    if let Ok(yml) = crate::helios_format::load_helios_yml(&entry.path()) {
                        if yml.info.name == req_name {
                            fs::remove_file(entry.path())?;
                            return Ok(());
                        }
                    }
                }
            }
            anyhow::bail!("请求不存在: {}", req_name);
        }
        Ok(())
    }

    /// 保存整个 AppData（批量写入所有集合）
    pub fn save_app_data(&self, data: &AppData) -> Result<()> {
        // 确保每个集合目录存在
        for col in &data.collections {
            let dir = self.base_dir.join(&col.name);
            if !dir.exists() {
                self.create_collection(&col.name)?;
            }
            // 保存每个请求
            for req in &col.requests {
                self.save_request(&col.name, req)?;
            }
        }
        Ok(())
    }
}

/// 生成文件名友好的 slug
fn slugify(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

// ─── 测试 ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Auth, BodyType, HttpMethod, KeyValue};

    #[test]
    fn test_file_storage_create_and_list_collections() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = FileStorage::new(tmp.path()).unwrap();

        storage.create_collection("users-api").unwrap();
        storage.create_collection("orders-api").unwrap();

        let names = storage.list_collections().unwrap();
        assert_eq!(names, vec!["orders-api", "users-api"]);
    }

    #[test]
    fn test_file_storage_create_duplicate_collection() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = FileStorage::new(tmp.path()).unwrap();

        storage.create_collection("test").unwrap();
        let result = storage.create_collection("test");
        assert!(result.is_err(), "重复创建应失败");
    }

    #[test]
    fn test_file_storage_delete_collection() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = FileStorage::new(tmp.path()).unwrap();

        storage.create_collection("to-delete").unwrap();
        storage.delete_collection("to-delete").unwrap();

        let names = storage.list_collections().unwrap();
        assert!(!names.contains(&"to-delete".to_string()));
    }

    #[test]
    fn test_file_storage_save_and_load_request() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = FileStorage::new(tmp.path()).unwrap();

        storage.create_collection("api").unwrap();

        let req = Request {
            id: "test-001".to_string(),
            name: "获取用户列表".to_string(),
            method: HttpMethod::GET,
            url: "https://api.example.com/users".to_string(),
            headers: vec![KeyValue {
                key: "Accept".to_string(),
                value: "application/json".to_string(),
                enabled: true,
            }],
            params: vec![],
            body: String::new(),
            body_type: BodyType::None,
            auth: Auth::None,
            graphql_query: None,
            graphql_variables: None,
            form_data: vec![],
            notes: String::new(),
            tags: vec![],
        };

        storage.save_request("api", &req).unwrap();

        let col = storage.load_collection("api").unwrap();
        assert_eq!(col.requests.len(), 1);
        assert_eq!(col.requests[0].name, "获取用户列表");
        assert_eq!(col.requests[0].method, HttpMethod::GET);
    }

    #[test]
    fn test_file_storage_load_all_collections() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = FileStorage::new(tmp.path()).unwrap();

        storage.create_collection("col-a").unwrap();
        storage.create_collection("col-b").unwrap();

        let req = Request {
            id: "r1".to_string(),
            name: "健康检查".to_string(),
            method: HttpMethod::GET,
            url: "https://api.example.com/health".to_string(),
            headers: vec![],
            params: vec![],
            body: String::new(),
            body_type: BodyType::None,
            auth: Auth::None,
            graphql_query: None,
            graphql_variables: None,
            form_data: vec![],
            notes: String::new(),
            tags: vec![],
        };
        storage.save_request("col-a", &req).unwrap();

        let collections = storage.load_all_collections().unwrap();
        assert_eq!(collections.len(), 2);
    }

    #[test]
    fn test_file_storage_save_and_load_app_data() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = FileStorage::new(tmp.path()).unwrap();

        let data = AppData {
            collections: vec![Collection {
                id: "c1".to_string(),
                name: "my-api".to_string(),
                requests: vec![Request {
                    id: "r1".to_string(),
                    name: "获取状态".to_string(),
                    method: HttpMethod::GET,
                    url: "https://api.example.com/status".to_string(),
                    headers: vec![],
                    params: vec![],
                    body: String::new(),
                    body_type: BodyType::None,
                    auth: Auth::None,
                    graphql_query: None,
                    graphql_variables: None,
                    form_data: vec![],
                    notes: String::new(),
                    tags: vec![],
                }],
                created_at: chrono::Local::now(),
            }],
            environments: vec![],
            history: vec![],
            active_env_id: None,
        };

        storage.save_app_data(&data).unwrap();
        let loaded = storage.load_app_data().unwrap();
        assert_eq!(loaded.collections.len(), 1);
        assert_eq!(loaded.collections[0].name, "my-api");
        assert_eq!(loaded.collections[0].requests.len(), 1);
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("获取用户列表"), "获取用户列表");
        assert_eq!(slugify("Create Order"), "create-order");
        assert_eq!(slugify("GET /api/v1/users"), "get-api-v1-users");
        assert_eq!(slugify("test   spaces"), "test-spaces");
    }

    #[test]
    fn test_file_storage_delete_request() {
        let tmp = tempfile::tempdir().unwrap();
        let storage = FileStorage::new(tmp.path()).unwrap();
        storage.create_collection("api").unwrap();

        let req = Request {
            id: "r1".to_string(),
            name: "删除测试请求".to_string(),
            method: HttpMethod::DELETE,
            url: "https://api.example.com/test".to_string(),
            headers: vec![],
            params: vec![],
            body: String::new(),
            body_type: BodyType::None,
            auth: Auth::None,
            graphql_query: None,
            graphql_variables: None,
            form_data: vec![],
            notes: String::new(),
            tags: vec![],
        };

        storage.save_request("api", &req).unwrap();
        let col = storage.load_collection("api").unwrap();
        assert_eq!(col.requests.len(), 1);

        storage.delete_request("api", "删除测试请求").unwrap();
        let col = storage.load_collection("api").unwrap();
        assert_eq!(col.requests.len(), 0);
    }
}
