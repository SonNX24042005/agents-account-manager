use crate::models::Account;
use crate::storage::ide_db::IdeDbSync;
use crate::storage::keyring_sync::KeyringSync;
use crate::storage::secure_file;
use anyhow::Result;
use std::fs;

pub struct AccountSwitcher;

impl AccountSwitcher {
    pub fn switch_active_account(account: &Account) -> Result<()> {
        // 1. Inject into OS Keyring (service=gemini, user=antigravity) used primarily by agy CLI
        let expires_at_rfc3339 = account.expires_at.to_rfc3339();
        let _ = KeyringSync::inject_keyring_credential(
            &account.access_token,
            &account.refresh_token,
            &expires_at_rfc3339,
        );

        // 2. Inject into all Antigravity IDE and Code SQLite DBs (state.vscdb)
        for db_path in IdeDbSync::find_antigravity_ide_db_paths() {
            let _ = IdeDbSync::inject_credential(
                &db_path,
                &account.email,
                &account.access_token,
                &account.refresh_token,
                account.expires_at.timestamp(),
                &account.device_profile.machine_id,
            );
        }

        // 3. Inject into native agy CLI auth credentials stores
        if let Some(home) = dirs::home_dir() {
            let auth_data = serde_json::json!({
                "email": account.email,
                "access_token": account.access_token,
                "refresh_token": account.refresh_token,
                "expires_at": account.expires_at,
                "device_profile": account.device_profile
            });
            let auth_str = serde_json::to_string_pretty(&auth_data)?;

            // Locations for ~/.antigravity
            let ant_dir = home.join(".antigravity");
            let _ = fs::create_dir_all(&ant_dir);
            for name in ["auth.json", "credentials.json"] {
                let p = ant_dir.join(name);
                secure_file::atomic_write(&p, auth_str.as_bytes(), 0o600)?;
            }

            // Locations for ~/.gemini/antigravity-cli
            let gemini_cli_dir = home.join(".gemini").join("antigravity-cli");
            let _ = fs::create_dir_all(&gemini_cli_dir);
            for name in ["auth.json", "credentials.json"] {
                let p = gemini_cli_dir.join(name);
                secure_file::atomic_write(&p, auth_str.as_bytes(), 0o600)?;
            }

            // Clean proxy settings from config.json and settings.json so agy runs natively
            let config_file = ant_dir.join("config.json");
            let settings_file = ant_dir.join("settings.json");

            for file in [config_file, settings_file] {
                if file.exists() {
                    let mut json: serde_json::Value =
                        serde_json::from_str(&fs::read_to_string(&file).unwrap_or_default())
                            .unwrap_or_else(|_| serde_json::json!({}));
                    if let Some(obj) = json.as_object_mut() {
                        obj.remove("proxy");
                        obj.remove("http_proxy");
                        obj.remove("https_proxy");
                        obj.remove("base_url");
                        obj.remove("GOOGLE_GEMINI_BASE_URL");
                        obj.remove("security");
                    }
                    let json = serde_json::to_string_pretty(&json)?;
                    let mode = secure_file::existing_mode_or(&file, 0o600);
                    secure_file::atomic_write(&file, json.as_bytes(), mode)?;
                }
            }

            // Remove proxy env vars from shell profiles
            Self::clean_shell_profiles();
        }

        tracing::info!(
            "[AccountSwitcher] Switched active account to: {}",
            account.email
        );
        Ok(())
    }

    fn clean_shell_profiles() {
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return,
        };

        for profile_path in [home.join(".bashrc"), home.join(".zshrc")] {
            if profile_path.exists() {
                if let Ok(content) = fs::read_to_string(&profile_path) {
                    let new_content = Self::remove_managed_proxy_block(&content);
                    if new_content == content {
                        continue;
                    }
                    let mode = secure_file::existing_mode_or(&profile_path, 0o600);
                    if let Err(error) = secure_file::backup(&profile_path) {
                        tracing::warn!(
                            "[AccountSwitcher] Refusing to update {:?} without a backup: {}",
                            profile_path,
                            error
                        );
                        continue;
                    }
                    if let Err(error) =
                        secure_file::atomic_write(&profile_path, new_content.as_bytes(), mode)
                    {
                        tracing::warn!(
                            "[AccountSwitcher] Failed to update {:?}: {}",
                            profile_path,
                            error
                        );
                    }
                }
            }
        }
    }

    fn remove_managed_proxy_block(content: &str) -> String {
        const START: &str = "# Antigravity Relay Proxy Config";
        const END: &str = "# End Antigravity Relay Proxy Config";
        let mut output = Vec::new();
        let mut in_managed_block = false;
        let mut legacy_exports_left = 0_u8;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed == START {
                in_managed_block = true;
                legacy_exports_left = 2;
                continue;
            }
            if in_managed_block && trimmed == END {
                in_managed_block = false;
                legacy_exports_left = 0;
                continue;
            }
            if in_managed_block && legacy_exports_left == 0 {
                in_managed_block = false;
            }
            if in_managed_block {
                let is_managed_export = trimmed.starts_with("export GOOGLE_GEMINI_BASE_URL=")
                    || trimmed.starts_with("export ANTIGRAVITY_BASE_URL=");
                if is_managed_export && legacy_exports_left > 0 {
                    legacy_exports_left -= 1;
                    continue;
                }
                in_managed_block = false;
                legacy_exports_left = 0;
            }
            output.push(line);
        }

        let mut result = output.join("\n");
        if content.ends_with('\n') && !result.is_empty() {
            result.push('\n');
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::AccountSwitcher;

    #[test]
    fn removes_only_the_managed_shell_block() {
        let input = "export GOOGLE_GEMINI_BASE_URL=https://custom.example\n\
# Antigravity Relay Proxy Config\n\
export GOOGLE_GEMINI_BASE_URL=http://127.0.0.1:8045/v1\n\
export ANTIGRAVITY_BASE_URL=http://127.0.0.1:8045/v1\n\
# End Antigravity Relay Proxy Config\n\
export HTTPS_PROXY=http://127.0.0.1:8045\n";
        let output = AccountSwitcher::remove_managed_proxy_block(input);

        assert!(output.contains("https://custom.example"));
        assert!(output.contains("export HTTPS_PROXY=http://127.0.0.1:8045"));
        assert!(!output.contains("# Antigravity Relay Proxy Config"));
        assert!(!output.contains("export ANTIGRAVITY_BASE_URL="));
    }
}
