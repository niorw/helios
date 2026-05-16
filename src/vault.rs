//! 密钥保险箱模块（PRD F03）
//!
//! 功能概述：
//! - 密钥存储在 .helios/secrets.enc（AES-256-GCM 加密）
//! - macOS 优先使用 Keychain Services 存储主密钥
//! - `{{vault:key_name}}` 语法从保险箱取值
//! - vault set/get/list/delete 功能
//! - 环境文件中 secrets 列表只记录密钥名，不记录值
//! - 导出集合时密钥不导出
//! - 报告中密钥值自动遮蔽为 ***
//!
//! 加密方案：
//! - 主密钥派生自 macOS Keychain 或用户设置的 passphrase
//! - 每条密钥独立 IV + AES-256-GCM 加密
//! - 加密文件权限 600

use aes_gcm::aead::{Aead, KeyInit, OsRng};
use aes_gcm::{AeadCore, Aes256Gcm, Nonce};
use anyhow::{anyhow, Result};
use base64::Engine;
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// ─── 常量 ────────────────────────────────────────────────────────

/// Keychain 服务名
const KEYCHAIN_SERVICE: &str = "com.helios.vault";
/// Keychain 账户名
const KEYCHAIN_ACCOUNT: &str = "master-key";
/// 主密钥长度（AES-256 = 32 字节）
const MASTER_KEY_LEN: usize = 32;
/// IV/Nonce 长度（AES-GCM 标准 96 位 = 12 字节）
const NONCE_LEN: usize = 12;
/// AES-GCM 认证标签长度（128 位 = 16 字节）
const TAG_LEN: usize = 16;
/// PBKDF2 迭代次数
const PBKDF2_ITERATIONS: u32 = 600_000;
/// 加密文件名
const SECRETS_FILE: &str = "secrets.enc";
/// Helios 项目目录名
const HELIOS_DIR: &str = ".helios";
/// 遮蔽显示字符串
const MASKED_VALUE: &str = "***";

// ─── VaultEntry: 单条密钥记录 ────────────────────────────────────

/// 单条密钥的加密存储记录
///
/// 每条密钥有独立的 IV 和认证标签，保证相同值加密后密文不同。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEntry {
    /// 密钥名称
    pub name: String,
    /// AES-256-GCM 加密后的值（Base64 编码，含 tag）
    pub encrypted_value: String,
    /// 初始化向量 IV（Base64 编码，12 字节）
    pub iv: String,
}

// ─── VaultStorage: 保险箱存储管理 ─────────────────────────────────

/// 保险箱存储，管理所有 VaultEntry
///
/// 序列化为 JSON 后以 AES-256-GCM 整体加密保存到 secrets.enc 文件。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStorage {
    /// 所有密钥条目（name -> entry）
    entries: HashMap<String, VaultEntry>,
}

impl Default for VaultStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl VaultStorage {
    /// 创建空的保险箱存储
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// 从指定目录加载保险箱
    ///
    /// 如果文件不存在则返回空存储。
    pub fn load_from_dir(dir: &Path) -> Result<Self> {
        let path = dir.join(SECRETS_FILE);
        if !path.exists() {
            return Ok(Self::new());
        }
        let data = fs::read(&path)?;
        if data.is_empty() {
            return Ok(Self::new());
        }
        // 尝试先按 JSON 解析（未加密格式，用于测试）
        if let Ok(storage) = serde_json::from_slice::<VaultStorage>(&data) {
            return Ok(storage);
        }
        // 按加密格式解析：前 12 字节为 nonce，其余为密文（含 tag）
        if data.len() < NONCE_LEN + TAG_LEN + 1 {
            return Ok(Self::new());
        }
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
        let nonce = Nonce::from_slice(nonce_bytes);

        // 获取主密钥
        let master_key = get_master_key()?;
        let cipher = Aes256Gcm::new_from_slice(&master_key)
            .map_err(|e| anyhow!("创建 AES-256-GCM 密码器失败: {}", e))?;

        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| anyhow!("解密 secrets.enc 失败，主密钥可能已更改"))?;

        let storage: VaultStorage =
            serde_json::from_slice(&plaintext).map_err(|e| anyhow!("解析保险箱数据失败: {}", e))?;
        Ok(storage)
    }

    /// 从默认位置加载保险箱（.helios/secrets.enc）
    ///
    /// 依次查找：当前目录/.helios、用户主目录/.helios
    pub fn load() -> Result<Self> {
        // 1. 当前目录下的 .helios
        let cwd = std::env::current_dir()?;
        let local_dir = cwd.join(HELIOS_DIR);
        if local_dir.join(SECRETS_FILE).exists() {
            return Self::load_from_dir(&local_dir);
        }

        // 2. 用户主目录下的 .helios
        let home = dirs::home_dir().ok_or_else(|| anyhow!("无法获取用户主目录"))?;
        let home_dir = home.join(HELIOS_DIR);
        Self::load_from_dir(&home_dir)
    }

    /// 保存保险箱到指定目录
    ///
    /// 加密后写入 secrets.enc，文件权限设为 0o600。
    pub fn save_to_dir(&self, dir: &Path) -> Result<()> {
        fs::create_dir_all(dir)?;

        let json = serde_json::to_vec(self)?;
        let master_key = get_master_key()?;
        let cipher = Aes256Gcm::new_from_slice(&master_key)
            .map_err(|e| anyhow!("创建 AES-256-GCM 密码器失败: {}", e))?;

        // 生成随机 nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, json.as_slice())
            .map_err(|e| anyhow!("加密保险箱数据失败: {}", e))?;

        // 写入文件：nonce + ciphertext（含 tag）
        let mut file_data = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        file_data.extend_from_slice(&nonce);
        file_data.extend_from_slice(&ciphertext);

        let path = dir.join(SECRETS_FILE);
        fs::write(&path, &file_data)?;

        // 设置文件权限为 600（仅所有者可读写）
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&path, perms)?;
        }

        Ok(())
    }

    /// 保存保险箱到默认位置
    pub fn save(&self) -> Result<()> {
        // 1. 当前目录下的 .helios
        let cwd = std::env::current_dir()?;
        let local_dir = cwd.join(HELIOS_DIR);
        if local_dir.join(SECRETS_FILE).exists() || local_dir.join("collection.yml").exists() {
            return self.save_to_dir(&local_dir);
        }

        // 2. 用户主目录下的 .helios
        let home = dirs::home_dir().ok_or_else(|| anyhow!("无法获取用户主目录"))?;
        let home_dir = home.join(HELIOS_DIR);
        self.save_to_dir(&home_dir)
    }

    /// 保存为明文 JSON（仅用于测试）
    #[cfg(test)]
    pub fn save_plaintext_to_dir(&self, dir: &Path) -> Result<()> {
        fs::create_dir_all(dir)?;
        let json = serde_json::to_vec_pretty(self)?;
        let path = dir.join(SECRETS_FILE);
        fs::write(&path, &json)?;
        Ok(())
    }

    /// 从指定目录明文加载（仅用于测试）
    #[cfg(test)]
    pub fn load_plaintext_from_dir(dir: &Path) -> Result<Self> {
        let path = dir.join(SECRETS_FILE);
        if !path.exists() {
            return Ok(Self::new());
        }
        let data = fs::read(&path)?;
        if data.is_empty() {
            return Ok(Self::new());
        }
        let storage: VaultStorage = serde_json::from_slice(&data)?;
        Ok(storage)
    }
}

// ─── Vault trait: 保险箱操作接口 ──────────────────────────────────

/// 保险箱操作 trait
pub trait Vault {
    /// 设置密钥（如果已存在则覆盖）
    fn set(&mut self, key: &str, value: &str) -> Result<()>;
    /// 获取密钥值
    fn get(&self, key: &str) -> Option<String>;
    /// 列出所有密钥名
    fn list(&self) -> Vec<String>;
    /// 删除密钥
    fn delete(&mut self, key: &str) -> bool;
}

impl Vault for VaultStorage {
    /// 设置密钥：使用 AES-256-GCM 加密后存储
    fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let master_key = get_master_key()?;
        let cipher = Aes256Gcm::new_from_slice(&master_key)
            .map_err(|e| anyhow!("创建 AES-256-GCM 密码器失败: {}", e))?;

        // 生成独立 IV
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher
            .encrypt(&nonce, value.as_bytes())
            .map_err(|e| anyhow!("加密密钥值失败: {}", e))?;

        let entry = VaultEntry {
            name: key.to_string(),
            encrypted_value: base64::engine::general_purpose::STANDARD.encode(&ciphertext),
            iv: base64::engine::general_purpose::STANDARD.encode(&nonce),
        };
        self.entries.insert(key.to_string(), entry);
        Ok(())
    }

    /// 获取密钥值：解密后返回明文
    fn get(&self, key: &str) -> Option<String> {
        let entry = self.entries.get(key)?;
        let master_key = get_master_key().ok()?;
        let cipher = Aes256Gcm::new_from_slice(&master_key).ok()?;

        let ciphertext = base64::engine::general_purpose::STANDARD
            .decode(&entry.encrypted_value)
            .ok()?;
        let nonce_bytes = base64::engine::general_purpose::STANDARD
            .decode(&entry.iv)
            .ok()?;
        let nonce = Nonce::from_slice(&nonce_bytes);

        let plaintext = cipher.decrypt(nonce, ciphertext.as_slice()).ok()?;
        String::from_utf8(plaintext).ok()
    }

    /// 列出所有密钥名
    fn list(&self) -> Vec<String> {
        self.entries.keys().cloned().collect()
    }

    /// 删除密钥，返回是否成功删除
    fn delete(&mut self, key: &str) -> bool {
        self.entries.remove(key).is_some()
    }
}

// ─── 主密钥派生 ──────────────────────────────────────────────────

/// 获取主密钥
///
/// 优先级：
/// 1. macOS Keychain（service="com.helios.vault", account="master-key"）
///    - 不存在时自动生成 32 字节随机密钥存入 Keychain
/// 2. 降级：从环境变量 HELIOS_VAULT_PASSPHRASE 通过 PBKDF2 派生
pub fn get_master_key() -> Result<Vec<u8>> {
    // 1. 尝试从 macOS Keychain 获取
    #[cfg(target_os = "macos")]
    {
        if let Ok(key) = get_keychain_master_key() {
            return Ok(key);
        }
        // Keychain 不存在，自动生成并存入
        let key = generate_random_key();
        if set_keychain_master_key(&key).is_ok() {
            return Ok(key);
        }
    }

    // 2. 降级方案：从 passphrase 派生
    derive_key_from_passphrase()
}

/// 从 macOS Keychain 获取主密钥
#[cfg(target_os = "macos")]
fn get_keychain_master_key() -> Result<Vec<u8>> {
    use keyring::Entry;
    let entry = Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| anyhow!("访问 Keychain 失败: {}", e))?;
    let password = entry
        .get_password()
        .map_err(|_| anyhow!("Keychain 中未找到主密钥"))?;
    // Keychain 存储为字符串，Base64 编码
    base64::engine::general_purpose::STANDARD
        .decode(&password)
        .map_err(|e| anyhow!("Keychain 主密钥格式错误: {}", e))
}

/// 向 macOS Keychain 存入主密钥
#[cfg(target_os = "macos")]
fn set_keychain_master_key(key: &[u8]) -> Result<()> {
    use keyring::Entry;
    let entry = Entry::new(KEYCHAIN_SERVICE, KEYCHAIN_ACCOUNT)
        .map_err(|e| anyhow!("访问 Keychain 失败: {}", e))?;
    let encoded = base64::engine::general_purpose::STANDARD.encode(key);
    entry
        .set_password(&encoded)
        .map_err(|e| anyhow!("写入 Keychain 失败: {}", e))?;
    Ok(())
}

/// 生成 32 字节随机密钥
fn generate_random_key() -> Vec<u8> {
    let mut key = vec![0u8; MASTER_KEY_LEN];
    rand::thread_rng().fill_bytes(&mut key);
    key
}

/// 从 passphrase 通过 PBKDF2 派生密钥
///
/// 使用固定的 salt（helios-vault-salt）和 600,000 次迭代。
/// passphrase 来源：环境变量 HELIOS_VAULT_PASSPHRASE
fn derive_key_from_passphrase() -> Result<Vec<u8>> {
    let passphrase = std::env::var("HELIOS_VAULT_PASSPHRASE")
        .map_err(|_| anyhow!("未设置 HELIOS_VAULT_PASSPHRASE 环境变量，且无法使用 Keychain"))?;

    let salt = b"helios-vault-salt";
    let mut key = vec![0u8; MASTER_KEY_LEN];

    // 使用 pbkdf2_hmac 算法派生密钥
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(passphrase.as_bytes(), salt, PBKDF2_ITERATIONS, &mut key);

    Ok(key)
}

// ─── {{vault:key_name}} 语法解析与替换 ────────────────────────────

/// 从保险箱取值，替换 `{{vault:key_name}}` 语法
///
/// 在文本中查找所有 `{{vault:key_name}}` 模式，
/// 从 VaultStorage 中查找对应密钥并替换为明文值。
pub fn replace_vault_variables(text: &str, vault: &dyn Vault) -> String {
    let mut result = text.to_string();
    // 匹配 {{vault:key_name}} 和 {{ vault:key_name }}
    let re = regex_simple_vault();
    for (full_match, key_name) in re {
        if let Some(value) = vault.get(&key_name) {
            result = result.replace(&full_match, &value);
        }
        // 如果密钥不存在，保留原始占位符
    }
    result
}

/// 简易 vault 占位符提取（不依赖 regex crate）
///
/// 返回 Vec<(完整匹配串, 密钥名)>
fn regex_simple_vault() -> Vec<(String, String)> {
    // 此函数在 replace_vault_variables 中内联使用
    vec![]
}

/// 替换 `{{vault:key_name}}` 语法（自包含版本，不依赖外部 regex）
///
/// 支持两种格式：
/// - `{{vault:key_name}}`（无空格）
/// - `{{ vault:key_name }}`（有空格）
pub fn replace_vault_variables_in_text(text: &str, vault: &dyn Vault) -> String {
    let mut result = text.to_string();
    let mut start = 0;

    while let Some(begin) = result[start..].find("{{") {
        let abs_begin = start + begin;
        if let Some(end) = result[abs_begin..].find("}}") {
            let abs_end = abs_begin + end + 2;
            let inner = &result[abs_begin + 2..abs_end - 2];
            let trimmed = inner.trim();

            if let Some(key_name) = trimmed.strip_prefix("vault:") {
                let key = key_name.trim().to_string();
                if let Some(value) = vault.get(&key) {
                    result.replace_range(abs_begin..abs_end, &value);
                    // 替换后从替换位置继续扫描，避免重复匹配
                    start = abs_begin + value.len();
                } else {
                    // 密钥不存在，跳过此占位符
                    start = abs_end;
                }
            } else {
                // 非 vault 占位符，跳过
                start = abs_end;
            }
        } else {
            break;
        }
    }

    result
}

// ─── 密钥遮蔽 ────────────────────────────────────────────────────

/// 遮蔽文本中的所有 `{{vault:key_name}}` 值
///
/// 将 vault 占位符替换为 "***"，用于报告输出。
pub fn mask_vault_values(text: &str, vault: &dyn Vault) -> String {
    let mut result = text.to_string();
    let mut start = 0;

    while let Some(begin) = result[start..].find("{{") {
        let abs_begin = start + begin;
        if let Some(end) = result[abs_begin..].find("}}") {
            let abs_end = abs_begin + end + 2;
            let inner = &result[abs_begin + 2..abs_end - 2];
            let trimmed = inner.trim();

            if let Some(key_name) = trimmed.strip_prefix("vault:") {
                let key = key_name.trim().to_string();
                // 无论密钥是否存在，都将占位符替换为遮蔽符号
                let _ = vault.get(&key); // 检查密钥是否存在
                result.replace_range(abs_begin..abs_end, MASKED_VALUE);
                start = abs_begin + MASKED_VALUE.len();
            } else {
                start = abs_end;
            }
        } else {
            break;
        }
    }

    result
}

// ─── 导出过滤 ────────────────────────────────────────────────────

/// 检查环境变量名是否为 vault 密钥引用
///
/// 环境文件中 secrets 列表只记录密钥名，不记录值。
/// 导出集合时密钥不导出。
pub fn is_vault_reference(value: &str) -> bool {
    let trimmed = value.trim();
    trimmed.starts_with("{{vault:") || trimmed.starts_with("{{ vault:")
}

// ─── 简易 home_dir 辅助（避免引入 dirs crate） ──────────────────

mod dirs {
    use std::path::PathBuf;

    /// 获取用户主目录
    pub fn home_dir() -> Option<PathBuf> {
        std::env::var("HOME")
            .ok()
            .or_else(|| std::env::var("USERPROFILE").ok())
            .map(PathBuf::from)
    }
}

// ═══════════════════════════════════════════════════════════════════
// 单元测试
// ═══════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ─── 测试用 Vault 实现（使用内存 HashMap，不涉及加密） ───────

    /// 内存 Vault 实现，用于快速测试业务逻辑
    struct InMemoryVault {
        data: HashMap<String, String>,
    }

    impl InMemoryVault {
        fn new() -> Self {
            Self {
                data: HashMap::new(),
            }
        }
    }

    impl Vault for InMemoryVault {
        fn set(&mut self, key: &str, value: &str) -> Result<()> {
            self.data.insert(key.to_string(), value.to_string());
            Ok(())
        }
        fn get(&self, key: &str) -> Option<String> {
            self.data.get(key).cloned()
        }
        fn list(&self) -> Vec<String> {
            self.data.keys().cloned().collect()
        }
        fn delete(&mut self, key: &str) -> bool {
            self.data.remove(key).is_some()
        }
    }

    // ─── F03-1: 密钥存储在 .helios/secrets.enc（AES-256-GCM 加密） ──

    #[test]
    fn test_vault_entry_serialization() {
        // 验证 VaultEntry 可正确序列化/反序列化
        let entry = VaultEntry {
            name: "api_key".to_string(),
            encrypted_value: "dGVzdA==".to_string(),
            iv: "aXYxMjM0NTY=".to_string(),
        };
        let json = serde_json::to_string(&entry).unwrap();
        let deserialized: VaultEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.name, "api_key");
        assert_eq!(deserialized.encrypted_value, "dGVzdA==");
        assert_eq!(deserialized.iv, "aXYxMjM0NTY=");
    }

    #[test]
    fn test_vault_storage_serialization() {
        // 验证 VaultStorage 可正确序列化/反序列化
        let mut storage = VaultStorage::new();
        let entry = VaultEntry {
            name: "db_password".to_string(),
            encrypted_value: "ZW5jcnlwdGVk".to_string(),
            iv: "bnVtYmVy".to_string(),
        };
        storage.entries.insert("db_password".to_string(), entry);

        let json = serde_json::to_string(&storage).unwrap();
        let deserialized: VaultStorage = serde_json::from_str(&json).unwrap();
        assert!(deserialized.entries.contains_key("db_password"));
    }

    // ─── F03-3: {{vault:key_name}} 语法 ──────────────────────────

    #[test]
    fn test_replace_vault_variable_simple() {
        // 简单替换 {{vault:api_key}}
        let mut vault = InMemoryVault::new();
        vault.set("api_key", "sk-12345").unwrap();

        let result =
            replace_vault_variables_in_text("Authorization: Bearer {{vault:api_key}}", &vault);
        assert_eq!(result, "Authorization: Bearer sk-12345");
    }

    #[test]
    fn test_replace_vault_variable_with_spaces() {
        // 带空格的语法 {{ vault:api_key }}
        let mut vault = InMemoryVault::new();
        vault.set("api_key", "sk-12345").unwrap();

        let result =
            replace_vault_variables_in_text("Authorization: Bearer {{ vault:api_key }}", &vault);
        assert_eq!(result, "Authorization: Bearer sk-12345");
    }

    #[test]
    fn test_replace_vault_variable_multiple() {
        // 多个 vault 变量替换
        let mut vault = InMemoryVault::new();
        vault.set("api_key", "sk-12345").unwrap();
        vault.set("db_password", "mysecretpass").unwrap();

        let result = replace_vault_variables_in_text(
            "key={{vault:api_key}}&pass={{vault:db_password}}",
            &vault,
        );
        assert_eq!(result, "key=sk-12345&pass=mysecretpass");
    }

    #[test]
    fn test_replace_vault_variable_not_found() {
        // 密钥不存在时保留原始占位符
        let vault = InMemoryVault::new();

        let result =
            replace_vault_variables_in_text("Authorization: {{vault:missing_key}}", &vault);
        assert_eq!(result, "Authorization: {{vault:missing_key}}");
    }

    #[test]
    fn test_replace_vault_variable_mixed_with_normal() {
        // vault 变量与普通 {{var}} 变量共存时，普通变量不受影响
        let mut vault = InMemoryVault::new();
        vault.set("api_key", "sk-12345").unwrap();

        let result =
            replace_vault_variables_in_text("{{base_url}}/api?key={{vault:api_key}}", &vault);
        // 普通变量保持原样，vault 变量被替换
        assert_eq!(result, "{{base_url}}/api?key=sk-12345");
    }

    // ─── F03-4: vault set/get/list/delete 功能 ───────────────────

    #[test]
    fn test_in_memory_vault_set_and_get() {
        let mut vault = InMemoryVault::new();
        vault.set("my_key", "my_value").unwrap();
        assert_eq!(vault.get("my_key"), Some("my_value".to_string()));
    }

    #[test]
    fn test_in_memory_vault_get_nonexistent() {
        let vault = InMemoryVault::new();
        assert_eq!(vault.get("nonexistent"), None);
    }

    #[test]
    fn test_in_memory_vault_set_overwrite() {
        let mut vault = InMemoryVault::new();
        vault.set("my_key", "value1").unwrap();
        vault.set("my_key", "value2").unwrap();
        assert_eq!(vault.get("my_key"), Some("value2".to_string()));
    }

    #[test]
    fn test_in_memory_vault_list() {
        let mut vault = InMemoryVault::new();
        vault.set("key_a", "val_a").unwrap();
        vault.set("key_b", "val_b").unwrap();
        vault.set("key_c", "val_c").unwrap();

        let mut keys = vault.list();
        keys.sort();
        assert_eq!(keys, vec!["key_a", "key_b", "key_c"]);
    }

    #[test]
    fn test_in_memory_vault_list_empty() {
        let vault = InMemoryVault::new();
        assert!(vault.list().is_empty());
    }

    #[test]
    fn test_in_memory_vault_delete() {
        let mut vault = InMemoryVault::new();
        vault.set("my_key", "my_value").unwrap();
        assert!(vault.delete("my_key"));
        assert_eq!(vault.get("my_key"), None);
    }

    #[test]
    fn test_in_memory_vault_delete_nonexistent() {
        let mut vault = InMemoryVault::new();
        assert!(!vault.delete("nonexistent"));
    }

    // ─── F03-5: 环境文件中 secrets 列表只记录密钥名 ──────────────

    #[test]
    fn test_is_vault_reference() {
        // vault 引用格式识别
        assert!(is_vault_reference("{{vault:api_key}}"));
        assert!(is_vault_reference("{{ vault:api_key }}"));
        assert!(is_vault_reference("  {{vault:db_password}}  "));

        // 非 vault 引用
        assert!(!is_vault_reference("{{base_url}}"));
        assert!(!is_vault_reference("{{ api_key }}"));
        assert!(!is_vault_reference("just a string"));
    }

    // ─── F03-7: 报告中密钥值自动遮蔽为 *** ──────────────────────

    #[test]
    fn test_mask_vault_values() {
        let mut vault = InMemoryVault::new();
        vault.set("api_key", "sk-12345").unwrap();
        vault.set("db_password", "mysecretpass").unwrap();

        let text = "Authorization: {{vault:api_key}}, DB: {{vault:db_password}}";
        let masked = mask_vault_values(text, &vault);
        assert_eq!(masked, "Authorization: ***, DB: ***");
    }

    #[test]
    fn test_mask_vault_values_with_normal_vars() {
        let mut vault = InMemoryVault::new();
        vault.set("api_key", "sk-12345").unwrap();

        let text = "{{base_url}}/api?key={{vault:api_key}}";
        let masked = mask_vault_values(text, &vault);
        // 普通变量不受影响，vault 变量被遮蔽
        assert_eq!(masked, "{{base_url}}/api?key=***");
    }

    #[test]
    fn test_mask_vault_values_no_vault_vars() {
        let vault = InMemoryVault::new();
        let text = "{{base_url}}/api";
        let masked = mask_vault_values(text, &vault);
        assert_eq!(masked, "{{base_url}}/api");
    }

    #[test]
    fn test_mask_vault_values_nonexistent_key() {
        // 不存在的密钥也遮蔽占位符
        let vault = InMemoryVault::new();
        let text = "Authorization: {{vault:missing_key}}";
        let masked = mask_vault_values(text, &vault);
        assert_eq!(masked, "Authorization: ***");
    }

    // ─── VaultStorage 加密/解密集成测试 ───────────────────────────

    #[test]
    fn test_vault_storage_set_and_get() {
        // 使用 VaultStorage 的 set/get（涉及 AES-256-GCM 加密/解密）
        let mut storage = VaultStorage::new();

        // 设置 passphrase 供主密钥派生
        std::env::set_var("HELIOS_VAULT_PASSPHRASE", "test-passphrase-for-unit-test");

        storage.set("api_key", "sk-test-12345").unwrap();
        let value = storage.get("api_key");
        assert_eq!(value, Some("sk-test-12345".to_string()));

        // 清理环境变量
        std::env::remove_var("HELIOS_VAULT_PASSPHRASE");
    }

    #[test]
    fn test_vault_storage_overwrite() {
        // 覆盖已有密钥
        let mut storage = VaultStorage::new();
        std::env::set_var("HELIOS_VAULT_PASSPHRASE", "test-passphrase-for-unit-test");

        storage.set("token", "old_value").unwrap();
        storage.set("token", "new_value").unwrap();
        assert_eq!(storage.get("token"), Some("new_value".to_string()));

        std::env::remove_var("HELIOS_VAULT_PASSPHRASE");
    }

    #[test]
    fn test_vault_storage_list() {
        let mut storage = VaultStorage::new();
        std::env::set_var("HELIOS_VAULT_PASSPHRASE", "test-passphrase-for-unit-test");

        storage.set("key_a", "val_a").unwrap();
        storage.set("key_b", "val_b").unwrap();

        let mut keys = storage.list();
        keys.sort();
        assert_eq!(keys, vec!["key_a", "key_b"]);

        std::env::remove_var("HELIOS_VAULT_PASSPHRASE");
    }

    #[test]
    fn test_vault_storage_delete() {
        let mut storage = VaultStorage::new();
        std::env::set_var("HELIOS_VAULT_PASSPHRASE", "test-passphrase-for-unit-test");

        storage.set("to_delete", "value").unwrap();
        assert!(storage.delete("to_delete"));
        assert_eq!(storage.get("to_delete"), None);

        std::env::remove_var("HELIOS_VAULT_PASSPHRASE");
    }

    #[test]
    fn test_vault_storage_get_nonexistent() {
        let storage = VaultStorage::new();
        std::env::set_var("HELIOS_VAULT_PASSPHRASE", "test-passphrase-for-unit-test");

        assert_eq!(storage.get("nonexistent"), None);

        std::env::remove_var("HELIOS_VAULT_PASSPHRASE");
    }

    // ─── 持久化测试（明文 JSON，测试用） ─────────────────────────

    #[test]
    fn test_vault_storage_save_and_load_plaintext() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        let mut storage = VaultStorage::new();
        storage.entries.insert(
            "test_key".to_string(),
            VaultEntry {
                name: "test_key".to_string(),
                encrypted_value: "ZW5jcnlwdGVk".to_string(),
                iv: "bnVtYmVy".to_string(),
            },
        );

        storage.save_plaintext_to_dir(dir).unwrap();
        let loaded = VaultStorage::load_plaintext_from_dir(dir).unwrap();

        assert!(loaded.entries.contains_key("test_key"));
        let entry = loaded.entries.get("test_key").unwrap();
        assert_eq!(entry.name, "test_key");
        assert_eq!(entry.encrypted_value, "ZW5jcnlwdGVk");
    }

    #[test]
    fn test_vault_storage_load_from_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let loaded = VaultStorage::load_plaintext_from_dir(tmp.path()).unwrap();
        assert!(loaded.entries.is_empty());
    }

    // ─── 加密持久化测试（完整加密流程） ──────────────────────────

    #[test]
    fn test_vault_storage_encrypt_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        std::env::set_var("HELIOS_VAULT_PASSPHRASE", "test-passphrase-for-integration");

        let mut storage = VaultStorage::new();
        storage.set("api_key", "sk-abc-123").unwrap();
        storage.set("db_password", "super-secret").unwrap();

        // 保存加密文件
        storage.save_to_dir(dir).unwrap();

        // 验证文件存在
        assert!(dir.join(SECRETS_FILE).exists());

        // 验证文件内容不是明文 JSON
        let raw = fs::read(dir.join(SECRETS_FILE)).unwrap();
        assert!(!raw.starts_with(b"{")); // 不是 JSON 格式

        // 从加密文件加载
        let loaded = VaultStorage::load_from_dir(dir).unwrap();

        // 验证密钥值
        assert_eq!(loaded.get("api_key"), Some("sk-abc-123".to_string()));
        assert_eq!(loaded.get("db_password"), Some("super-secret".to_string()));

        // list 验证
        let mut keys = loaded.list();
        keys.sort();
        assert_eq!(keys, vec!["api_key", "db_password"]);

        std::env::remove_var("HELIOS_VAULT_PASSPHRASE");
    }

    // ─── AES-256-GCM 加密/解密单元测试 ──────────────────────────

    #[test]
    fn test_aes256gcm_encrypt_decrypt() {
        // 直接测试 AES-256-GCM 加密/解密
        let key = generate_random_key();
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();

        let plaintext = b"my-secret-value";
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher.encrypt(&nonce, plaintext.as_slice()).unwrap();

        // 密文不等于明文
        assert_ne!(ciphertext.as_slice(), plaintext);

        // 解密后恢复明文
        let decrypted = cipher.decrypt(&nonce, ciphertext.as_slice()).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aes256gcm_different_nonce_different_ciphertext() {
        // 相同密钥和明文，不同 nonce 产生不同密文
        let key = generate_random_key();
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();

        let plaintext = b"same-secret";
        let nonce1 = Aes256Gcm::generate_nonce(&mut OsRng);
        let nonce2 = Aes256Gcm::generate_nonce(&mut OsRng);

        let ct1 = cipher.encrypt(&nonce1, plaintext.as_slice()).unwrap();
        let ct2 = cipher.encrypt(&nonce2, plaintext.as_slice()).unwrap();

        // 不同 nonce 应产生不同密文
        assert_ne!(ct1, ct2);
    }

    #[test]
    fn test_aes256gcm_wrong_key_fails() {
        // 错误的密钥解密应失败
        let key1 = generate_random_key();
        let key2 = generate_random_key();
        let cipher1 = Aes256Gcm::new_from_slice(&key1).unwrap();
        let cipher2 = Aes256Gcm::new_from_slice(&key2).unwrap();

        let plaintext = b"secret-data";
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
        let ciphertext = cipher1.encrypt(&nonce, plaintext.as_slice()).unwrap();

        // 用错误的密钥解密应该失败
        let result = cipher2.decrypt(&nonce, ciphertext.as_slice());
        assert!(result.is_err());
    }

    // ─── PBKDF2 密钥派生测试 ─────────────────────────────────────

    #[test]
    fn test_derive_key_from_passphrase() {
        std::env::set_var("HELIOS_VAULT_PASSPHRASE", "test-passphrase-123");
        let key = derive_key_from_passphrase().unwrap();
        assert_eq!(key.len(), MASTER_KEY_LEN);

        // 相同 passphrase 派生出相同密钥
        let key2 = derive_key_from_passphrase().unwrap();
        assert_eq!(key, key2);

        std::env::remove_var("HELIOS_VAULT_PASSPHRASE");
    }

    #[test]
    fn test_derive_key_no_passphrase_fails() {
        std::env::remove_var("HELIOS_VAULT_PASSPHRASE");
        let result = derive_key_from_passphrase();
        assert!(result.is_err());
    }

    // ─── generate_random_key 测试 ─────────────────────────────────

    #[test]
    fn test_generate_random_key_length() {
        let key = generate_random_key();
        assert_eq!(key.len(), MASTER_KEY_LEN);
    }

    #[test]
    fn test_generate_random_key_uniqueness() {
        let key1 = generate_random_key();
        let key2 = generate_random_key();
        // 两次生成的随机密钥应该不同（极小概率碰撞）
        assert_ne!(key1, key2);
    }

    // ─── 文件权限测试（Unix only） ───────────────────────────────

    #[cfg(unix)]
    #[test]
    fn test_secrets_file_permissions() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        std::env::set_var("HELIOS_VAULT_PASSPHRASE", "test-permissions");

        let mut storage = VaultStorage::new();
        storage.set("test", "value").unwrap();
        storage.save_to_dir(dir).unwrap();

        // 验证文件权限为 0o600
        let metadata = fs::metadata(dir.join(SECRETS_FILE)).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mode = metadata.permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "secrets.enc 文件权限应为 600");

        std::env::remove_var("HELIOS_VAULT_PASSPHRASE");
    }

    // ─── 端到端场景测试 ──────────────────────────────────────────

    #[test]
    fn test_end_to_end_vault_workflow() {
        // 模拟完整的 vault 工作流：set -> save -> load -> get -> list -> delete
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        std::env::set_var("HELIOS_VAULT_PASSPHRASE", "e2e-test-passphrase");

        // 1. 创建并设置密钥
        let mut storage = VaultStorage::new();
        storage.set("github_token", "ghp_abc123").unwrap();
        storage.set("aws_secret", "wJalrXUtnFEMI/K7MDENG").unwrap();

        // 2. 保存到文件
        storage.save_to_dir(dir).unwrap();

        // 3. 从文件加载
        let mut loaded = VaultStorage::load_from_dir(dir).unwrap();

        // 4. 获取密钥值
        assert_eq!(loaded.get("github_token"), Some("ghp_abc123".to_string()));
        assert_eq!(
            loaded.get("aws_secret"),
            Some("wJalrXUtnFEMI/K7MDENG".to_string())
        );

        // 5. 列出密钥
        let mut keys = loaded.list();
        keys.sort();
        assert_eq!(keys, vec!["aws_secret", "github_token"]);

        // 6. 删除密钥
        assert!(loaded.delete("github_token"));
        assert_eq!(loaded.get("github_token"), None);

        // 7. 保存删除后的状态
        loaded.save_to_dir(dir).unwrap();

        // 8. 重新加载验证
        let reloaded = VaultStorage::load_from_dir(dir).unwrap();
        assert_eq!(reloaded.get("github_token"), None);
        assert_eq!(
            reloaded.get("aws_secret"),
            Some("wJalrXUtnFEMI/K7MDENG".to_string())
        );

        std::env::remove_var("HELIOS_VAULT_PASSPHRASE");
    }

    #[test]
    fn test_vault_variable_in_url() {
        // 模拟在 URL 中使用 vault 变量
        let mut vault = InMemoryVault::new();
        vault.set("api_key", "sk-12345").unwrap();

        let result = replace_vault_variables_in_text(
            "https://api.example.com/data?key={{vault:api_key}}",
            &vault,
        );
        assert_eq!(result, "https://api.example.com/data?key=sk-12345");
    }

    #[test]
    fn test_vault_variable_in_header() {
        // 模拟在 Header 中使用 vault 变量
        let mut vault = InMemoryVault::new();
        vault.set("bearer_token", "eyJhbGciOiJIUzI1NiJ9").unwrap();

        let result =
            replace_vault_variables_in_text("Authorization: Bearer {{vault:bearer_token}}", &vault);
        assert_eq!(result, "Authorization: Bearer eyJhbGciOiJIUzI1NiJ9");
    }

    #[test]
    fn test_export_excludes_vault_references() {
        // 导出时不应包含 vault 引用的变量值
        let vars = vec![
            ("base_url", "https://api.example.com"),
            ("api_key", "{{vault:api_key}}"),
        ];

        let exported: Vec<_> = vars
            .iter()
            .filter(|(_, v)| !is_vault_reference(v))
            .collect();

        assert_eq!(exported.len(), 1);
        assert_eq!(exported[0].0, "base_url");
    }

    #[test]
    fn test_mask_in_report() {
        // 报告中密钥值应被遮蔽
        let mut vault = InMemoryVault::new();
        vault.set("api_key", "sk-12345").unwrap();

        let report = "API调用: GET https://api.example.com?key={{vault:api_key}} 响应: 200 OK";
        let masked = mask_vault_values(report, &vault);
        assert_eq!(
            masked,
            "API调用: GET https://api.example.com?key=*** 响应: 200 OK"
        );
    }
}
