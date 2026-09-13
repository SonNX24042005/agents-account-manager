use anyhow::Result;
use keyring::Entry;

pub struct KeyringSync;

impl KeyringSync {
    pub fn inject_keyring_credential(
        access_token: &str,
        refresh_token: &str,
        expires_at_rfc3339: &str,
    ) -> Result<()> {
        Self::inject_keyring_credential_with_target(
            "gemini",
            "antigravity",
            access_token,
            refresh_token,
            expires_at_rfc3339,
        )
    }

    pub fn inject_keyring_credential_with_target(
        service: &str,
        user: &str,
        access_token: &str,
        refresh_token: &str,
        expires_at_rfc3339: &str,
    ) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            if std::env::var("DBUS_SESSION_BUS_ADDRESS").is_err() {
                if let Some(runtime_dir) = dirs::runtime_dir() {
                    let bus_path = runtime_dir.join("bus");
                    if bus_path.exists() {
                        std::env::set_var(
                            "DBUS_SESSION_BUS_ADDRESS",
                            format!("unix:path={}", bus_path.display()),
                        );
                    }
                }
            }
        }

        let entry = Entry::new(service, user)?;
        let payload = serde_json::json!({
            "token": {
                "access_token": access_token,
                "token_type": "Bearer",
                "refresh_token": refresh_token,
                "expiry": expires_at_rfc3339
            },
            "auth_method": "consumer"
        })
        .to_string();

        match entry.set_password(&payload) {
            Ok(_) => {
                tracing::info!(
                    "[Keyring Sync] Successfully updated OS Keyring (service: {service}, user: {user})"
                );
                Ok(())
            }
            Err(e) => {
                tracing::warn!(
                    "[Keyring Sync] Failed to update OS Keyring (service: {service}, user: {user}): {e}"
                );
                Err(anyhow::anyhow!("Keyring set_password failed: {e}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_inject_keyring_isolated() {
        let res = KeyringSync::inject_keyring_credential_with_target(
            "aam_test_service",
            "aam_test_user",
            "test_access",
            "test_refresh",
            "2026-09-13T00:00:00Z",
        );
        assert!(res.is_ok());
        if let Ok(entry) = Entry::new("aam_test_service", "aam_test_user") {
            let _ = entry.delete_password();
        }
    }
}
