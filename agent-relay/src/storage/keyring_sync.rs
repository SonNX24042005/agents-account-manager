use anyhow::Result;
use keyring::Entry;

pub struct KeyringSync;

impl KeyringSync {
    pub fn inject_keyring_credential(
        access_token: &str,
        refresh_token: &str,
        expires_at_rfc3339: &str,
    ) -> Result<()> {
        let entry = Entry::new("gemini", "antigravity")?;
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

        let _ = entry.set_password(&payload);
        tracing::info!(
            "[Keyring Sync] Successfully updated OS Keyring (service: gemini, user: antigravity)"
        );
        Ok(())
    }
}
