use crate::config;
use crate::models::AppData;
use anyhow::Result;
use directories::ProjectDirs;
use std::fs;
use std::path::PathBuf;

pub struct Storage {
    data_dir: PathBuf,
}

impl Storage {
    pub fn new() -> Result<Self> {
        let proj_dirs = ProjectDirs::from(
            config::APP_QUALIFIER,
            config::APP_ORGANIZATION,
            config::APP_APPLICATION,
        )
        .ok_or_else(|| anyhow::anyhow!("Cannot determine data directory"))?;
        let data_dir = proj_dirs.data_dir().to_path_buf();
        fs::create_dir_all(&data_dir)?;
        Ok(Self { data_dir })
    }

    pub fn data_dir(&self) -> &std::path::Path {
        &self.data_dir
    }

    pub fn data_file(&self) -> PathBuf {
        self.data_dir.join(config::DATA_FILE_NAME)
    }

    pub fn load(&self) -> Result<AppData> {
        let path = self.data_file();
        if !path.exists() {
            return Ok(AppData::default());
        }
        let content = fs::read_to_string(&path)?;
        let data: AppData = serde_json::from_str(&content).unwrap_or_default();
        Ok(data)
    }

    pub fn save(&self, data: &AppData) -> Result<()> {
        let path = self.data_file();
        let content = serde_json::to_string_pretty(data)?;
        fs::write(&path, content)?;
        Ok(())
    }
}
