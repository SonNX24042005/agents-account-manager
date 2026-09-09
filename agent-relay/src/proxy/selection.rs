use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Agent {
    Antigravity,
    Codex,
    Claude,
}

impl Agent {
    pub fn name(self) -> &'static str {
        match self {
            Self::Antigravity => "Antigravity",
            Self::Codex => "Codex",
            Self::Claude => "Claude",
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SelectionFlags {
    pub antigravity: bool,
    pub codex: bool,
    pub claude: bool,
}

impl Default for SelectionFlags {
    fn default() -> Self {
        // Preserve the existing Antigravity behavior on upgrade.
        Self {
            antigravity: true,
            codex: false,
            claude: false,
        }
    }
}

impl SelectionFlags {
    pub fn enabled(&self, agent: Agent) -> bool {
        match agent {
            Agent::Antigravity => self.antigravity,
            Agent::Codex => self.codex,
            Agent::Claude => self.claude,
        }
    }
}

pub struct SelectionSettings {
    path: PathBuf,
    pub flags: Mutex<SelectionFlags>,
}

impl SelectionSettings {
    pub fn new(data_dir: &std::path::Path) -> Result<Self> {
        let path = data_dir.join("agent-selection.json");
        let flags = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => SelectionFlags::default(),
            Err(e) => return Err(e.into()),
        };
        Ok(Self {
            path,
            flags: Mutex::new(flags),
        })
    }

    pub async fn set(&self, agent: Agent, enabled: bool) -> Result<()> {
        let mut flags = self.flags.lock().await;
        let mut next = flags.clone();
        match agent {
            Agent::Antigravity => next.antigravity = enabled,
            Agent::Codex => next.codex = enabled,
            Agent::Claude => next.claude = enabled,
        }
        crate::storage::secure_file::atomic_write(
            &self.path,
            &serde_json::to_vec_pretty(&next)?,
            0o600,
        )?;
        *flags = next;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn settings_persist_independently_and_failed_save_does_not_change_memory() {
        let dir = std::env::temp_dir().join(format!("aam-settings-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let settings = SelectionSettings::new(&dir).unwrap();
        settings.set(Agent::Antigravity, false).await.unwrap();
        settings.set(Agent::Codex, true).await.unwrap();
        let reopened = SelectionSettings::new(&dir).unwrap();
        let flags = reopened.flags.lock().await;
        assert!(!flags.antigravity && flags.codex && !flags.claude);
        drop(flags);
        std::fs::remove_file(&settings.path).unwrap();
        std::fs::create_dir(&settings.path).unwrap();
        assert!(settings.set(Agent::Claude, true).await.is_err());
        assert!(!settings.flags.lock().await.claude);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
