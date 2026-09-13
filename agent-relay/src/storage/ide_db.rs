use std::path::PathBuf;
use anyhow::{Context, Result};
use rusqlite::Connection;
use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;

pub struct IdeDbSync;

impl IdeDbSync {
    pub fn find_antigravity_ide_db_paths() -> Vec<PathBuf> {
        let home = match dirs::home_dir() {
            Some(h) => h,
            None => return vec![],
        };

        let mut candidate_paths = vec![];

        #[cfg(target_os = "linux")]
        {
            candidate_paths.push(home.join(".config/Antigravity IDE/User/globalStorage/state.vscdb"));
            candidate_paths.push(home.join(".config/Antigravity/User/globalStorage/state.vscdb"));
            candidate_paths.push(home.join(".config/Code/User/globalStorage/state.vscdb"));
        }

        #[cfg(target_os = "windows")]
        {
            candidate_paths.push(home.join("AppData/Roaming/Antigravity IDE/User/globalStorage/state.vscdb"));
            candidate_paths.push(home.join("AppData/Roaming/Antigravity/User/globalStorage/state.vscdb"));
            candidate_paths.push(home.join("AppData/Roaming/Code/User/globalStorage/state.vscdb"));
        }

        #[cfg(target_os = "macos")]
        {
            candidate_paths.push(home.join("Library/Application Support/Antigravity IDE/User/globalStorage/state.vscdb"));
            candidate_paths.push(home.join("Library/Application Support/Antigravity/User/globalStorage/state.vscdb"));
            candidate_paths.push(home.join("Library/Application Support/Code/User/globalStorage/state.vscdb"));
        }

        candidate_paths.into_iter().filter(|p| p.exists()).collect()
    }

    fn encode_varint(mut val: u64) -> Vec<u8> {
        let mut res = Vec::new();
        while val >= 0x80 {
            res.push((val as u8 & 0x7F) | 0x80);
            val >>= 7;
        }
        res.push(val as u8);
        res
    }

    fn encode_tag_len(tag_num: u32, wire_type: u32, data: &[u8]) -> Vec<u8> {
        let tag = (tag_num << 3) | wire_type;
        let mut out = Self::encode_varint(tag as u64);
        out.extend(Self::encode_varint(data.len() as u64));
        out.extend_from_slice(data);
        out
    }

    pub fn build_unified_oauth_token(access_token: &str, refresh_token: &str, expiry_secs: i64) -> String {
        // 1. Build inner OAuthTokenInfo proto
        let mut inner_proto = Vec::new();
        inner_proto.extend(Self::encode_tag_len(1, 2, access_token.as_bytes()));
        inner_proto.extend(Self::encode_tag_len(2, 2, b"Bearer"));
        if !refresh_token.is_empty() {
            inner_proto.extend(Self::encode_tag_len(3, 2, refresh_token.as_bytes()));
        }
        
        let mut exp_inner = Vec::new();
        exp_inner.push((1 << 3) | 0);
        exp_inner.extend(Self::encode_varint(expiry_secs as u64));
        inner_proto.extend(Self::encode_tag_len(4, 2, &exp_inner));

        let inner_b64 = BASE64.encode(&inner_proto);
        let auth_state_json = r#"{"state":"signedIn","context":{"project":"","showProjectError":false,"errorMessage":"","ineligibleMessage":"","verificationUrl":"","isGcpTos":false,"browserOpenFailed":false,"appealUrl":"","appealLinkText":""}}"#;

        // 2. Build outer unified proto
        let mut outer = Vec::new();
        
        let mut entry1 = Vec::new();
        entry1.extend(Self::encode_tag_len(1, 2, b"authStateWithContextSentinelKey"));
        entry1.extend(Self::encode_tag_len(2, 2, auth_state_json.as_bytes()));
        outer.extend(Self::encode_tag_len(1, 2, &entry1));

        let mut entry2 = Vec::new();
        entry2.extend(Self::encode_tag_len(1, 2, b"oauthTokenInfoSentinelKey"));
        entry2.extend(Self::encode_tag_len(2, 2, inner_b64.as_bytes()));
        outer.extend(Self::encode_tag_len(1, 2, &entry2));

        BASE64.encode(&outer)
    }

    pub fn inject_credential(
        db_path: &PathBuf,
        email: &str,
        access_token: &str,
        refresh_token: &str,
        expires_at_secs: i64,
        machine_id: &str,
    ) -> Result<()> {
        let conn = Connection::open(db_path).context("Failed to open Antigravity IDE SQLite DB")?;

        let query = "INSERT OR REPLACE INTO ItemTable (key, value) VALUES (?1, ?2)";
        
        let unified_oauth = Self::build_unified_oauth_token(access_token, refresh_token, expires_at_secs);
        conn.execute(query, ["antigravityUnifiedStateSync.oauthToken", &unified_oauth])?;

        let auth_json = serde_json::json!({
            "access_token": access_token,
            "refresh_token": refresh_token,
            "email": email,
            "machine_id": machine_id
        }).to_string();

        conn.execute(query, ["antigravity.token", &auth_json])?;
        conn.execute(query, ["gemini.token", &auth_json])?;
        conn.execute(query, ["antigravity.currentAccount", email])?;

        tracing::info!("[IDE Sync] Successfully injected credential & unified token for {} into {:?}", email, db_path);
        Ok(())
    }
}
