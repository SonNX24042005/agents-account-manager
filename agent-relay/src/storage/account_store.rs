use crate::models::Account;
use crate::storage::secure_file;
use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

pub struct AccountStore {
    base_dir: PathBuf,
}

impl AccountStore {
    pub fn new(base_dir: PathBuf) -> Self {
        let _ = fs::create_dir_all(&base_dir);
        let _ = Self::restrict_base_dir(&base_dir);
        Self { base_dir }
    }

    pub fn load_all(&self) -> Result<Vec<Account>> {
        let mut accounts = Vec::new();
        if !self.base_dir.exists() {
            return Ok(accounts);
        }
        Self::restrict_base_dir(&self.base_dir)?;

        for entry in fs::read_dir(&self.base_dir).context("Failed to read accounts dir")? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "json") {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
                }
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(account) = serde_json::from_str::<Account>(&content) {
                        let file_id = path.file_stem().and_then(|stem| stem.to_str());
                        if Self::validate_id(&account.id).is_ok()
                            && file_id == Some(account.id.as_str())
                        {
                            accounts.push(account);
                        } else {
                            tracing::warn!(
                                "[AccountStore] Ignoring account file with an invalid or mismatched id: {:?}",
                                path
                            );
                        }
                    }
                }
            }
        }

        Ok(accounts)
    }

    pub fn save(&self, account: &Account) -> Result<()> {
        Self::validate_id(&account.id)?;
        fs::create_dir_all(&self.base_dir)?;
        Self::restrict_base_dir(&self.base_dir)?;
        let path = self.base_dir.join(format!("{}.json", account.id));
        let json = serde_json::to_string_pretty(account)?;
        secure_file::atomic_write(&path, json.as_bytes(), 0o600)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn delete(&self, id: &str) -> Result<()> {
        Self::validate_id(id)?;
        let path = self.base_dir.join(format!("{}.json", id));
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn validate_id(id: &str) -> Result<()> {
        let parsed = uuid::Uuid::parse_str(id).context("Invalid account id")?;
        anyhow::ensure!(
            parsed.to_string() == id,
            "Account id must be a canonical UUID"
        );
        Ok(())
    }

    fn restrict_base_dir(path: &std::path::Path) -> Result<()> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
        }
        #[cfg(not(unix))]
        {
            let _ = path;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::AccountStore;

    #[test]
    fn rejects_non_uuid_paths() {
        let dir = std::env::temp_dir().join(format!("aam-store-test-{}", uuid::Uuid::new_v4()));
        let store = AccountStore::new(dir.clone());

        let error = store
            .delete("../../outside")
            .expect_err("path traversal must fail");
        assert!(error.to_string().contains("Invalid account id"));

        let _ = std::fs::remove_dir_all(dir);
    }
}
