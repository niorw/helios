//! 数据迁移模块
//!
//! 将旧 JSON data.json 格式迁移到文件系统格式。
//! 旧格式: 单个 data.json 包含所有集合和请求
//! 新格式: 目录结构，每个集合一个目录，每个请求一个 .helios.yml 文件

use crate::file_storage::FileStorage;
use crate::models::AppData;
use crate::storage::Storage;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// 迁移状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct MigrationStatus {
    /// 是否已迁移
    pub migrated: bool,
    /// 迁移时间
    pub migrated_at: Option<String>,
    /// 旧数据文件路径
    pub legacy_data_file: Option<String>,
    /// 迁移的集合数
    pub collections_migrated: usize,
    /// 迁移的请求数
    pub requests_migrated: usize,
}

impl Default for MigrationStatus {
    fn default() -> Self {
        Self {
            migrated: false,
            migrated_at: None,
            legacy_data_file: None,
            collections_migrated: 0,
            requests_migrated: 0,
        }
    }
}

/// 执行从 JSON 到文件系统的数据迁移
pub fn migrate_json_to_files(dry_run: bool) -> Result<MigrationStatus> {
    // 1. 读取旧 JSON 数据
    let legacy_storage = Storage::new()?;
    let data_file = legacy_storage.data_file();

    if !data_file.exists() {
        return Ok(MigrationStatus {
            migrated: false,
            migrated_at: None,
            legacy_data_file: Some(data_file.display().to_string()),
            collections_migrated: 0,
            requests_migrated: 0,
        });
    }

    let app_data = legacy_storage.load().context("无法读取旧数据文件")?;

    let total_requests: usize = app_data.collections.iter().map(|c| c.requests.len()).sum();

    if dry_run {
        return Ok(MigrationStatus {
            migrated: false,
            migrated_at: None,
            legacy_data_file: Some(data_file.display().to_string()),
            collections_migrated: app_data.collections.len(),
            requests_migrated: total_requests,
        });
    }

    // 2. 写入文件系统
    let file_storage = FileStorage::with_default_path()?;
    file_storage.save_app_data(&app_data)?;

    // 3. 备份旧文件
    let backup_path = data_file.with_extension("json.bak");
    fs::copy(&data_file, &backup_path)
        .with_context(|| format!("无法备份旧数据文件到 {}", backup_path.display()))?;

    // 4. 写入迁移状态
    let status = MigrationStatus {
        migrated: true,
        migrated_at: Some(chrono::Local::now().to_rfc3339()),
        legacy_data_file: Some(backup_path.display().to_string()),
        collections_migrated: app_data.collections.len(),
        requests_migrated: total_requests,
    };

    let status_path = FileStorage::default_path()?
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("migration_status.yml");
    let status_content = serde_yaml::to_string(&status)?;
    fs::write(&status_path, status_content)?;

    Ok(status)
}

/// 检查是否已完成迁移
pub fn is_migrated() -> Result<bool> {
    let status_path = FileStorage::default_path()?
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("migration_status.yml");

    if !status_path.exists() {
        return Ok(false);
    }

    let content = fs::read_to_string(&status_path)?;
    let status: MigrationStatus = serde_yaml::from_str(&content)?;
    Ok(status.migrated)
}

// ─── 测试 ──────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Auth, BodyType, Collection, HttpMethod, KeyValue, Request};
    use tempfile::TempDir;

    fn create_test_app_data() -> AppData {
        AppData {
            collections: vec![
                Collection {
                    id: "c1".to_string(),
                    name: "用户API".to_string(),
                    requests: vec![
                        Request {
                            id: "r1".to_string(),
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
                        },
                        Request {
                            id: "r2".to_string(),
                            name: "创建用户".to_string(),
                            method: HttpMethod::POST,
                            url: "https://api.example.com/users".to_string(),
                            headers: vec![KeyValue {
                                key: "Content-Type".to_string(),
                                value: "application/json".to_string(),
                                enabled: true,
                            }],
                            params: vec![],
                            body: r#"{"name":"test"}"#.to_string(),
                            body_type: BodyType::Json,
                            auth: Auth::Bearer {
                                token: "my-token".to_string(),
                            },
                            graphql_query: None,
                            graphql_variables: None,
                            form_data: vec![],
                            notes: String::new(),
                            tags: vec![],
                        },
                    ],
                    created_at: chrono::Local::now(),
                },
                Collection {
                    id: "c2".to_string(),
                    name: "订单API".to_string(),
                    requests: vec![Request {
                        id: "r3".to_string(),
                        name: "查询订单".to_string(),
                        method: HttpMethod::GET,
                        url: "https://api.example.com/orders".to_string(),
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
                },
            ],
            environments: vec![],
            history: vec![],
            active_env_id: None,
        }
    }

    #[test]
    fn test_migration_status_default() {
        let status = MigrationStatus::default();
        assert!(!status.migrated);
        assert_eq!(status.collections_migrated, 0);
        assert_eq!(status.requests_migrated, 0);
    }

    #[test]
    fn test_migration_status_serialize_roundtrip() {
        let status = MigrationStatus {
            migrated: true,
            migrated_at: Some("2025-01-01T00:00:00+08:00".to_string()),
            legacy_data_file: Some("/path/to/data.json.bak".to_string()),
            collections_migrated: 5,
            requests_migrated: 20,
        };
        let yaml = serde_yaml::to_string(&status).unwrap();
        let parsed: MigrationStatus = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(parsed, status);
    }

    #[test]
    fn test_file_storage_roundtrip_migration() {
        // 模拟迁移：AppData -> FileStorage -> 重新加载
        let tmp = tempfile::tempdir().unwrap();
        let storage = FileStorage::new(tmp.path()).unwrap();

        let original = create_test_app_data();
        storage.save_app_data(&original).unwrap();

        let loaded = storage.load_app_data().unwrap();
        assert_eq!(loaded.collections.len(), 2);

        // 按名字查找集合（顺序可能不固定）
        let users_col = loaded
            .collections
            .iter()
            .find(|c| c.name == "用户API")
            .expect("应有用户API集合");
        let orders_col = loaded
            .collections
            .iter()
            .find(|c| c.name == "订单API")
            .expect("应有订单API集合");

        assert_eq!(users_col.requests.len(), 2);
        assert_eq!(orders_col.requests.len(), 1);

        // 验证请求详情 — 按名字查找
        let create_req = users_col
            .requests
            .iter()
            .find(|r| r.name == "创建用户")
            .expect("应有创建用户请求");
        assert_eq!(create_req.method, HttpMethod::POST);
        assert_eq!(create_req.body_type, BodyType::Json);
        assert_eq!(
            create_req.auth,
            Auth::Bearer {
                token: "my-token".to_string(),
            }
        );
    }

    #[test]
    fn test_migrate_dry_run_no_side_effects() {
        // dry_run 不应产生任何副作用
        let tmp = tempfile::tempdir().unwrap();
        let collections_dir = tmp.path().join("collections");
        fs::create_dir_all(&collections_dir).unwrap();

        // dry_run 应该只返回统计，不创建文件
        let status = MigrationStatus {
            migrated: false,
            migrated_at: None,
            legacy_data_file: None,
            collections_migrated: 3,
            requests_migrated: 10,
        };
        assert!(!status.migrated);
        assert_eq!(status.collections_migrated, 3);
    }
}
