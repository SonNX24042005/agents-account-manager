use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub master_key: String,
    pub data_dir: PathBuf,
}

impl Default for Config {
    fn default() -> Self {
        let data_dir = std::env::var_os("AGENT_DATA_DIR")
            .or_else(|| std::env::var_os("ANTIGRAVITY_DATA_DIR"))
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
                let agent_dir = home.join(".agent-relay");
                let legacy_dir = home.join(".antigravity-relay");
                if !agent_dir.exists() && legacy_dir.exists() {
                    legacy_dir
                } else {
                    agent_dir
                }
            });
        let port = std::env::var("AGENT_PORT")
            .or_else(|_| std::env::var("ANTIGRAVITY_PORT"))
            .ok()
            .and_then(|value| value.parse::<u16>().ok())
            .filter(|port| *port != 0)
            .unwrap_or(8045);

        let master_key = Self::load_or_generate_master_key(&data_dir);

        Self {
            host: "127.0.0.1".to_string(),
            port,
            master_key,
            data_dir,
        }
    }
}

impl Config {
    pub fn accounts_dir(&self) -> PathBuf {
        self.data_dir.join("accounts")
    }

    pub fn ensure_directories(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(self.accounts_dir())?;
        Self::restrict_directory(&self.data_dir)?;
        Self::restrict_directory(&self.accounts_dir())?;
        Ok(())
    }

    fn restrict_directory(path: &Path) -> anyhow::Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
        }
        #[cfg(not(unix))]
        {
            let _ = path;
        }
        Ok(())
    }

    /// Load master API key from env var or persisted file. Generates a new random key on first run.
    fn load_or_generate_master_key(data_dir: &Path) -> String {
        // Priority 1: environment variable override
        if let Ok(key) = std::env::var("AGENT_MASTER_KEY").or_else(|_| std::env::var("ANTIGRAVITY_MASTER_KEY")) {
            if !key.is_empty() {
                return key;
            }
        }

        // Priority 2: persisted key file from a previous run
        let key_file = data_dir.join("master.key");
        if let Ok(content) = std::fs::read_to_string(&key_file) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(&key_file, std::fs::Permissions::from_mode(0o600));
            }
            let key = content.trim().to_string();
            if !key.is_empty() {
                return key;
            }
        }

        // Priority 3: generate new random key and persist it with restricted permissions
        let key = format!(
            "sk-aam-{}",
            uuid::Uuid::new_v4().to_string().replace('-', "")
        );
        let _ = std::fs::create_dir_all(data_dir);
        let _ = Self::restrict_directory(data_dir);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        match options.open(&key_file) {
            Ok(mut file) => {
                if file.write_all(key.as_bytes()).is_ok() && file.sync_all().is_ok() {
                    return key;
                }
                let _ = std::fs::remove_file(&key_file);
            }
            Err(_) => {
                if let Ok(content) = std::fs::read_to_string(&key_file) {
                    let existing = content.trim().to_string();
                    if !existing.is_empty() {
                        return existing;
                    }
                }
            }
        }
        key
    }
}
