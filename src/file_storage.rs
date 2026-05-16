//! 文件系统存储后端
//!
//! Phase 0 - F01: 集合即目录，请求即文件。
//! Phase 0 - F02: 文件夹层级支持。
//! 目录结构:
//!   collections/
//!     my-collection/
//!       collection.yml          (集合元信息)
//!       get-users.helios.yml   (请求1)
//!       create-order.helios.yml(请求2)
//!       users/                  (文件夹)
//!         folder.yml            (文件夹元信息)
//!         get-user.helios.yml   (请求)
//!         admin/                (嵌套文件夹)
//!           folder.yml
//!           ...
//!     another-collection/
//!       ...

use crate::helios_format::{load_collection_from_dir, request_to_helios_yml, save_helios_yml};
use crate::models::{AppData, Collection, Folder, Request};
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

/// 文件夹目录的元信息文件 (folder.yml)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct FolderYml {
    pub name: String,
    #[serde(default)]
    pub seq: u32,
    #[serde(default)]
    pub variables: HashMap<String, String>,
    #[serde(default)]
    pub docs: String,
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

    /// 加载指定集合（含文件夹层级）
    pub fn load_collection(&self, name: &str) -> Result<Collection> {
        let dir = self.base_dir.join(name);
        if !dir.exists() {
            anyhow::bail!("集合不存在: {}", name);
        }
        load_collection_with_folders(&dir)
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
            // 保存文件夹
            for folder in &col.folders {
                save_folder_to_dir(folder, &dir)?;
            }
        }
        Ok(())
    }
}

// ─── 文件夹加载/保存函数 ──────────────────────────────────────────

/// 从目录加载集合（含文件夹层级）
///
/// 读取 collection.yml + 扫描 .helios.yml 请求文件 + 递归加载子目录作为 Folder
fn load_collection_with_folders(dir: &Path) -> Result<Collection> {
    // 读取 collection.yml 获取集合名称
    let col_yml_path = dir.join("collection.yml");
    let col_name = if col_yml_path.exists() {
        let content = fs::read_to_string(&col_yml_path)?;
        let col_yml: CollectionYml = serde_yaml::from_str(&content)?;
        col_yml.name
    } else {
        dir.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    };

    // 扫描目录下所有 .helios.yml 文件（仅当前目录级别）
    let mut requests = Vec::new();
    let mut folders = Vec::new();

    let entries =
        fs::read_dir(dir).with_context(|| format!("无法读取集合目录: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            // 读取 .helios.yml 请求文件
            if let Some(name) = path.file_name() {
                let name = name.to_string_lossy();
                if name.ends_with(".helios.yml") {
                    if let Ok(yml) = crate::helios_format::load_helios_yml(&path) {
                        let file_stem = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default()
                            .replace(".helios", "");
                        if let Ok(req) =
                            crate::helios_format::helios_yml_to_request(&yml, &file_stem)
                        {
                            requests.push((yml.info.seq, req));
                        }
                    }
                }
            }
        } else if path.is_dir() {
            // 子目录可能是一个 Folder
            if path.join("folder.yml").exists() {
                if let Ok(folder) = load_folder_from_dir(&path) {
                    folders.push(folder);
                }
            }
        }
    }

    // 按 seq 排序请求
    requests.sort_by_key(|(seq, _)| *seq);
    // 按 seq 排序文件夹
    folders.sort_by_key(|f| f.seq);

    Ok(Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name: col_name,
        folders,
        requests: requests.into_iter().map(|(_, req)| req).collect(),
        created_at: chrono::Local::now(),
    })
}

/// 从目录加载文件夹（递归）
///
/// 读取 folder.yml + 扫描 .helios.yml 请求文件 + 递归加载子目录
pub fn load_folder_from_dir(dir: &Path) -> Result<Folder> {
    let folder_yml_path = dir.join("folder.yml");
    let folder_yml: FolderYml = if folder_yml_path.exists() {
        let content = fs::read_to_string(&folder_yml_path)
            .with_context(|| format!("无法读取 folder.yml: {}", folder_yml_path.display()))?;
        serde_yaml::from_str(&content)
            .with_context(|| format!("无法解析 folder.yml: {}", folder_yml_path.display()))?
    } else {
        // 如果没有 folder.yml，用目录名作为名称
        FolderYml {
            name: dir
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            seq: 0,
            variables: HashMap::new(),
            docs: String::new(),
        }
    };

    // 扫描目录下 .helios.yml 请求文件
    let mut requests = Vec::new();
    let mut sub_folders = Vec::new();

    let entries =
        fs::read_dir(dir).with_context(|| format!("无法读取文件夹目录: {}", dir.display()))?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(name) = path.file_name() {
                let name = name.to_string_lossy();
                if name.ends_with(".helios.yml") {
                    if let Ok(yml) = crate::helios_format::load_helios_yml(&path) {
                        let file_stem = path
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default()
                            .replace(".helios", "");
                        if let Ok(req) =
                            crate::helios_format::helios_yml_to_request(&yml, &file_stem)
                        {
                            requests.push((yml.info.seq, req));
                        }
                    }
                }
            }
        } else if path.is_dir() {
            // 递归加载子文件夹
            if path.join("folder.yml").exists() {
                if let Ok(sub) = load_folder_from_dir(&path) {
                    sub_folders.push(sub);
                }
            }
        }
    }

    // 按 seq 排序
    requests.sort_by_key(|(seq, _)| *seq);
    sub_folders.sort_by_key(|f| f.seq);

    Ok(Folder {
        id: uuid::Uuid::new_v4().to_string(),
        name: folder_yml.name,
        seq: folder_yml.seq,
        variables: folder_yml.variables,
        docs: folder_yml.docs,
        folders: sub_folders,
        requests: requests.into_iter().map(|(_, req)| req).collect(),
        created_at: chrono::Local::now(),
    })
}

/// 保存文件夹到目录（递归）
///
/// 写 folder.yml + 递归创建子目录 + 写请求文件
pub fn save_folder_to_dir(folder: &Folder, parent_dir: &Path) -> Result<()> {
    let dir = parent_dir.join(slugify(&folder.name));
    fs::create_dir_all(&dir).with_context(|| format!("无法创建文件夹目录: {}", dir.display()))?;

    // 写 folder.yml
    let folder_yml = FolderYml {
        name: folder.name.clone(),
        seq: folder.seq,
        variables: folder.variables.clone(),
        docs: folder.docs.clone(),
    };
    let content = serde_yaml::to_string(&folder_yml)?;
    fs::write(dir.join("folder.yml"), content)?;

    // 写请求文件
    for req in &folder.requests {
        let yml = request_to_helios_yml(req)?;
        let slug = slugify(&req.name);
        let filename = format!("{}.helios.yml", slug);
        save_helios_yml(&yml, &dir.join(filename))?;
    }

    // 递归保存子文件夹
    for sub in &folder.folders {
        save_folder_to_dir(sub, &dir)?;
    }

    Ok(())
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
                folders: vec![],
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
        };

        storage.save_request("api", &req).unwrap();
        let col = storage.load_collection("api").unwrap();
        assert_eq!(col.requests.len(), 1);

        storage.delete_request("api", "删除测试请求").unwrap();
        let col = storage.load_collection("api").unwrap();
        assert_eq!(col.requests.len(), 0);
    }

    // ─── F02-TDD 测试 ──────────────────────────────────────────────

    /// 辅助函数：创建一个简单的 Request
    fn make_request(id: &str, name: &str, method: HttpMethod, url: &str) -> Request {
        Request {
            id: id.to_string(),
            name: name.to_string(),
            method,
            url: url.to_string(),
            headers: vec![],
            params: vec![],
            body: String::new(),
            body_type: BodyType::None,
            auth: Auth::None,
            graphql_query: None,
            graphql_variables: None,
            form_data: vec![],
            notes: String::new(),
        }
    }

    #[test]
    fn test_folder_yml_parse() {
        // 测试 FolderYml 从 YAML 解析
        let yaml_content = r#"
name: 用户管理
seq: 1
variables:
  base_url: https://api.example.com
  token: abc123
docs: 用户相关的API接口
"#;
        let folder_yml: FolderYml = serde_yaml::from_str(yaml_content).unwrap();
        assert_eq!(folder_yml.name, "用户管理");
        assert_eq!(folder_yml.seq, 1);
        assert_eq!(
            folder_yml.variables.get("base_url"),
            Some(&"https://api.example.com".to_string())
        );
        assert_eq!(
            folder_yml.variables.get("token"),
            Some(&"abc123".to_string())
        );
        assert_eq!(folder_yml.docs, "用户相关的API接口");
    }

    #[test]
    fn test_folder_yml_parse_minimal() {
        // 测试 FolderYml 最小化 YAML 解析（只有 name）
        let yaml_content = "name: 简单文件夹\n";
        let folder_yml: FolderYml = serde_yaml::from_str(yaml_content).unwrap();
        assert_eq!(folder_yml.name, "简单文件夹");
        assert_eq!(folder_yml.seq, 0);
        assert!(folder_yml.variables.is_empty());
        assert!(folder_yml.docs.is_empty());
    }

    #[test]
    fn test_folder_yml_serialize() {
        // 测试 FolderYml 序列化后再反解析应保持一致
        let mut variables = HashMap::new();
        variables.insert("env".to_string(), "staging".to_string());
        variables.insert("version".to_string(), "v2".to_string());

        let original = FolderYml {
            name: "订单模块".to_string(),
            seq: 2,
            variables,
            docs: "订单相关API".to_string(),
        };

        let yaml_str = serde_yaml::to_string(&original).unwrap();
        let parsed: FolderYml = serde_yaml::from_str(&yaml_str).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_load_collection_with_folders() {
        // 测试从包含子文件夹的目录加载集合
        let tmp = tempfile::tempdir().unwrap();
        let col_dir = tmp.path().join("my-api");
        fs::create_dir_all(&col_dir).unwrap();

        // 创建 collection.yml
        let col_yml = CollectionYml {
            name: "我的API".to_string(),
            description: "测试集合".to_string(),
        };
        fs::write(
            col_dir.join("collection.yml"),
            serde_yaml::to_string(&col_yml).unwrap(),
        )
        .unwrap();

        // 创建根级别请求
        let req_yml = r#"
info:
  name: 健康检查
  type: http
  seq: 1
http:
  method: GET
  url: https://api.example.com/health
"#;
        fs::write(col_dir.join("health.helios.yml"), req_yml).unwrap();

        // 创建子文件夹 users/
        let users_dir = col_dir.join("users");
        fs::create_dir_all(&users_dir).unwrap();

        let folder_yml = FolderYml {
            name: "用户管理".to_string(),
            seq: 1,
            variables: {
                let mut v = HashMap::new();
                v.insert(
                    "base_url".to_string(),
                    "https://api.example.com/users".to_string(),
                );
                v
            },
            docs: "用户模块".to_string(),
        };
        fs::write(
            users_dir.join("folder.yml"),
            serde_yaml::to_string(&folder_yml).unwrap(),
        )
        .unwrap();

        // 创建子文件夹内的请求
        let user_req_yml = r#"
info:
  name: 获取用户列表
  type: http
  seq: 1
http:
  method: GET
  url: https://api.example.com/users
"#;
        fs::write(users_dir.join("list-users.helios.yml"), user_req_yml).unwrap();

        // 加载集合
        let col = load_collection_with_folders(&col_dir).unwrap();
        assert_eq!(col.name, "我的API");
        assert_eq!(col.requests.len(), 1, "根级别应有1个请求");
        assert_eq!(col.requests[0].name, "健康检查");
        assert_eq!(col.folders.len(), 1, "应有1个文件夹");
        assert_eq!(col.folders[0].name, "用户管理");
        assert_eq!(col.folders[0].seq, 1);
        assert_eq!(
            col.folders[0].variables.get("base_url"),
            Some(&"https://api.example.com/users".to_string())
        );
        assert_eq!(col.folders[0].docs, "用户模块");
        assert_eq!(col.folders[0].requests.len(), 1, "文件夹内应有1个请求");
        assert_eq!(col.folders[0].requests[0].name, "获取用户列表");
    }

    #[test]
    fn test_save_collection_with_folders() {
        // 测试保存含文件夹的集合
        let tmp = tempfile::tempdir().unwrap();
        let storage = FileStorage::new(tmp.path()).unwrap();

        let mut vars = HashMap::new();
        vars.insert(
            "base_url".to_string(),
            "https://api.example.com".to_string(),
        );

        let folder = Folder {
            id: "f1".to_string(),
            name: "用户管理".to_string(),
            seq: 1,
            variables: vars,
            docs: "用户相关API".to_string(),
            folders: vec![],
            requests: vec![make_request(
                "r1",
                "获取用户",
                HttpMethod::GET,
                "https://api.example.com/users",
            )],
            created_at: chrono::Local::now(),
        };

        let data = AppData {
            collections: vec![Collection {
                id: "c1".to_string(),
                name: "my-api".to_string(),
                folders: vec![folder],
                requests: vec![make_request(
                    "r0",
                    "健康检查",
                    HttpMethod::GET,
                    "https://api.example.com/health",
                )],
                created_at: chrono::Local::now(),
            }],
            environments: vec![],
            history: vec![],
            active_env_id: None,
        };

        storage.save_app_data(&data).unwrap();

        // 验证文件结构
        let col_dir = tmp.path().join("my-api");
        assert!(
            col_dir.join("collection.yml").exists(),
            "应有 collection.yml"
        );
        assert!(
            col_dir.join("健康检查.helios.yml").exists(),
            "根级别应有请求文件"
        );

        let folder_dir = col_dir.join("用户管理");
        assert!(
            folder_dir.join("folder.yml").exists(),
            "文件夹目录应有 folder.yml"
        );
        assert!(
            folder_dir.join("获取用户.helios.yml").exists(),
            "文件夹内应有请求文件"
        );

        // 验证 folder.yml 内容
        let folder_content = fs::read_to_string(folder_dir.join("folder.yml")).unwrap();
        let parsed_yml: FolderYml = serde_yaml::from_str(&folder_content).unwrap();
        assert_eq!(parsed_yml.name, "用户管理");
        assert_eq!(parsed_yml.seq, 1);
        assert_eq!(
            parsed_yml.variables.get("base_url"),
            Some(&"https://api.example.com".to_string())
        );
        assert_eq!(parsed_yml.docs, "用户相关API");
    }

    #[test]
    fn test_nested_folder_roundtrip() {
        // 测试嵌套文件夹的完整读写往返
        let tmp = tempfile::tempdir().unwrap();
        let col_dir = tmp.path().join("ecommerce");
        fs::create_dir_all(&col_dir).unwrap();

        // 创建集合
        let col_yml = CollectionYml {
            name: "电商平台".to_string(),
            description: String::new(),
        };
        fs::write(
            col_dir.join("collection.yml"),
            serde_yaml::to_string(&col_yml).unwrap(),
        )
        .unwrap();

        // 根级别请求
        fs::write(
            col_dir.join("ping.helios.yml"),
            r#"
info:
  name: Ping
  type: http
  seq: 1
http:
  method: GET
  url: https://api.shop.com/ping
"#,
        )
        .unwrap();

        // 创建一级文件夹 orders/
        let orders_dir = col_dir.join("orders");
        fs::create_dir_all(&orders_dir).unwrap();
        let orders_yml = FolderYml {
            name: "订单管理".to_string(),
            seq: 1,
            variables: HashMap::new(),
            docs: "订单模块".to_string(),
        };
        fs::write(
            orders_dir.join("folder.yml"),
            serde_yaml::to_string(&orders_yml).unwrap(),
        )
        .unwrap();
        fs::write(
            orders_dir.join("list-orders.helios.yml"),
            r#"
info:
  name: 获取订单列表
  type: http
  seq: 1
http:
  method: GET
  url: https://api.shop.com/orders
"#,
        )
        .unwrap();

        // 创建二级文件夹 orders/refunds/
        let refunds_dir = orders_dir.join("refunds");
        fs::create_dir_all(&refunds_dir).unwrap();
        let mut refund_vars = HashMap::new();
        refund_vars.insert(
            "refund_api".to_string(),
            "https://api.shop.com/refunds".to_string(),
        );
        let refunds_yml = FolderYml {
            name: "退款管理".to_string(),
            seq: 1,
            variables: refund_vars,
            docs: "退款流程".to_string(),
        };
        fs::write(
            refunds_dir.join("folder.yml"),
            serde_yaml::to_string(&refunds_yml).unwrap(),
        )
        .unwrap();
        fs::write(
            refunds_dir.join("create-refund.helios.yml"),
            r#"
info:
  name: 创建退款
  type: http
  seq: 1
http:
  method: POST
  url: https://api.shop.com/refunds
"#,
        )
        .unwrap();

        // 创建三级文件夹 orders/refunds/appeals/
        let appeals_dir = refunds_dir.join("appeals");
        fs::create_dir_all(&appeals_dir).unwrap();
        let appeals_yml = FolderYml {
            name: "申诉管理".to_string(),
            seq: 1,
            variables: HashMap::new(),
            docs: "退款申诉".to_string(),
        };
        fs::write(
            appeals_dir.join("folder.yml"),
            serde_yaml::to_string(&appeals_yml).unwrap(),
        )
        .unwrap();
        fs::write(
            appeals_dir.join("list-appeals.helios.yml"),
            r#"
info:
  name: 获取申诉列表
  type: http
  seq: 1
http:
  method: GET
  url: https://api.shop.com/refunds/appeals
"#,
        )
        .unwrap();

        // 加载集合
        let col = load_collection_with_folders(&col_dir).unwrap();
        assert_eq!(col.name, "电商平台");
        assert_eq!(col.requests.len(), 1, "根级别1个请求");
        assert_eq!(col.folders.len(), 1, "1个一级文件夹");

        let orders = &col.folders[0];
        assert_eq!(orders.name, "订单管理");
        assert_eq!(orders.seq, 1);
        assert_eq!(orders.docs, "订单模块");
        assert_eq!(orders.requests.len(), 1, "orders 内1个请求");
        assert_eq!(orders.folders.len(), 1, "1个二级文件夹");

        let refunds = &orders.folders[0];
        assert_eq!(refunds.name, "退款管理");
        assert_eq!(
            refunds.variables.get("refund_api"),
            Some(&"https://api.shop.com/refunds".to_string())
        );
        assert_eq!(refunds.docs, "退款流程");
        assert_eq!(refunds.requests.len(), 1, "refunds 内1个请求");
        assert_eq!(refunds.folders.len(), 1, "1个三级文件夹");

        let appeals = &refunds.folders[0];
        assert_eq!(appeals.name, "申诉管理");
        assert_eq!(appeals.docs, "退款申诉");
        assert_eq!(appeals.requests.len(), 1, "appeals 内1个请求");
        assert_eq!(appeals.requests[0].name, "获取申诉列表");
        assert!(appeals.folders.is_empty(), "三级文件夹无子文件夹");

        // 保存并重新加载（roundtrip）
        let tmp2 = tempfile::tempdir().unwrap();
        save_folder_to_dir(orders, tmp2.path()).unwrap();

        let reloaded = load_folder_from_dir(&tmp2.path().join("订单管理")).unwrap();
        assert_eq!(reloaded.name, "订单管理");
        assert_eq!(reloaded.requests.len(), 1);
        assert_eq!(reloaded.folders.len(), 1);
        assert_eq!(reloaded.folders[0].name, "退款管理");
        assert_eq!(reloaded.folders[0].folders.len(), 1);
        assert_eq!(reloaded.folders[0].folders[0].name, "申诉管理");
        assert_eq!(
            reloaded.folders[0].folders[0].requests[0].name,
            "获取申诉列表"
        );
    }

    #[test]
    fn test_load_folder_from_dir_without_folder_yml() {
        // 测试没有 folder.yml 时用目录名作为名称
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("some-folder");
        fs::create_dir_all(&dir).unwrap();

        // 只放一个请求文件，不放 folder.yml
        fs::write(
            dir.join("test.helios.yml"),
            r#"
info:
  name: 测试请求
  type: http
  seq: 1
http:
  method: GET
  url: https://example.com/test
"#,
        )
        .unwrap();

        // 此时需要 folder.yml 才能被识别为 Folder
        // 所以加载应该成功但只有用 load_folder_from_dir 直接调用
        let folder = load_folder_from_dir(&dir).unwrap();
        assert_eq!(folder.name, "some-folder", "无 folder.yml 时用目录名");
        assert_eq!(folder.requests.len(), 1);
    }

    #[test]
    fn test_folder_yml_roundtrip_with_empty_fields() {
        // 测试空字段的 FolderYml 往返
        let original = FolderYml {
            name: "空文件夹".to_string(),
            seq: 0,
            variables: HashMap::new(),
            docs: String::new(),
        };
        let yaml_str = serde_yaml::to_string(&original).unwrap();
        let parsed: FolderYml = serde_yaml::from_str(&yaml_str).unwrap();
        assert_eq!(parsed, original);
    }

    #[test]
    fn test_folder_seq_sorting() {
        // 测试文件夹按 seq 排序
        let tmp = tempfile::tempdir().unwrap();
        let col_dir = tmp.path().join("sorted-api");
        fs::create_dir_all(&col_dir).unwrap();

        // 创建集合
        let col_yml = CollectionYml {
            name: "排序测试".to_string(),
            description: String::new(),
        };
        fs::write(
            col_dir.join("collection.yml"),
            serde_yaml::to_string(&col_yml).unwrap(),
        )
        .unwrap();

        // 创建三个文件夹，seq 分别为 3, 1, 2
        for (dir_name, seq) in &[("c-folder", 3u32), ("a-folder", 1u32), ("b-folder", 2u32)] {
            let dir = col_dir.join(dir_name);
            fs::create_dir_all(&dir).unwrap();
            let yml = FolderYml {
                name: format!("文件夹{}", seq),
                seq: *seq,
                variables: HashMap::new(),
                docs: String::new(),
            };
            fs::write(dir.join("folder.yml"), serde_yaml::to_string(&yml).unwrap()).unwrap();
        }

        let col = load_collection_with_folders(&col_dir).unwrap();
        assert_eq!(col.folders.len(), 3);
        assert_eq!(col.folders[0].name, "文件夹1", "seq=1 应排在第一");
        assert_eq!(col.folders[1].name, "文件夹2", "seq=2 应排在第二");
        assert_eq!(col.folders[2].name, "文件夹3", "seq=3 应排在第三");
    }
}
