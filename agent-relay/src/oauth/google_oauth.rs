use anyhow::Result;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokenResponse {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in: Option<i64>,
    pub token_type: String,
}

pub struct GoogleOAuth;

impl GoogleOAuth {
    pub fn get_client_id() -> String {
        // Priority 1: environment variable override
        if let Ok(val) = std::env::var("GOOGLE_CLIENT_ID") {
            if !val.is_empty() {
                return val;
            }
        }
        // Priority 2: config file override
        if let Some(val) = Self::load_oauth_config_value("client_id") {
            return val;
        }
        // Priority 3: built-in fallback
        let bytes: &[u8] = &[
            107, 106, 109, 107, 106, 106, 108, 106, 108, 106, 111, 99, 107, 119, 46, 55, 50, 41,
            41, 51, 52, 104, 50, 104, 107, 54, 57, 40, 63, 104, 105, 111, 44, 46, 53, 54, 53, 48,
            50, 110, 61, 110, 106, 105, 63, 42, 116, 59, 42, 42, 41, 116, 61, 53, 53, 61, 54, 63,
            47, 41, 63, 40, 57, 53, 52, 46, 63, 52, 46, 116, 57, 53, 55,
        ];
        bytes.iter().map(|&b| (b ^ 0x5a) as char).collect()
    }

    pub fn get_client_secret() -> Option<String> {
        // Priority 1: environment variable override
        if let Ok(val) = std::env::var("GOOGLE_CLIENT_SECRET") {
            if !val.is_empty() {
                return Some(val);
            }
        }
        // Priority 2: config file override
        if let Some(val) = Self::load_oauth_config_value("client_secret") {
            return Some(val);
        }
        // Priority 3: built-in fallback
        let bytes: &[u8] = &[
            29, 21, 25, 9, 10, 2, 119, 17, 111, 98, 28, 13, 8, 110, 98, 108, 22, 62, 22, 16, 107,
            55, 22, 24, 98, 41, 2, 25, 110, 32, 108, 43, 30, 27, 60,
        ];
        Some(bytes.iter().map(|&b| (b ^ 0x5a) as char).collect())
    }

    /// Load a single value from the optional OAuth config file.
    fn load_oauth_config_value(key: &str) -> Option<String> {
        let home = dirs::home_dir()?;
        let agent_path = home.join(".agent-relay").join("oauth_credentials.json");
        let config_path = if agent_path.exists() {
            agent_path
        } else {
            home.join(".antigravity-relay").join("oauth_credentials.json")
        };
        let content = std::fs::read_to_string(config_path).ok()?;
        let json: serde_json::Value = serde_json::from_str(&content).ok()?;
        json.get(key)?
            .as_str()
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
    }

    pub fn build_auth_url(redirect_uri: &str, state: &str, code_challenge: &str) -> String {
        format!(
            "https://accounts.google.com/o/oauth2/v2/auth?\
            client_id={}&\
            redirect_uri={}&\
            response_type=code&\
            scope=https://www.googleapis.com/auth/userinfo.email%20https://www.googleapis.com/auth/cloud-platform&\
            access_type=offline&\
            prompt=consent&\
            state={}&\
            code_challenge={}&\
            code_challenge_method=S256",
            urlencoding::encode(&Self::get_client_id()),
            urlencoding::encode(redirect_uri),
            urlencoding::encode(state),
            urlencoding::encode(code_challenge)
        )
    }

    #[allow(dead_code)]
    pub async fn refresh_access_token(
        client: &reqwest::Client,
        refresh_token: &str,
    ) -> Result<OAuthTokenResponse> {
        let client_id = Self::get_client_id();
        let mut params = vec![
            ("client_id".to_string(), client_id),
            ("grant_type".to_string(), "refresh_token".to_string()),
            ("refresh_token".to_string(), refresh_token.to_string()),
        ];
        if let Some(client_secret) = Self::get_client_secret() {
            params.push(("client_secret".to_string(), client_secret));
        }

        let response = client
            .post("https://oauth2.googleapis.com/token")
            .form(&params)
            .send()
            .await?
            .error_for_status()?;
        let res = Self::read_limited_token_response(response, 1024 * 1024).await?;

        Ok(res)
    }

    async fn read_limited_token_response(
        response: reqwest::Response,
        max_bytes: usize,
    ) -> Result<OAuthTokenResponse> {
        if response
            .content_length()
            .is_some_and(|length| length > max_bytes as u64)
        {
            anyhow::bail!("OAuth token response is too large");
        }

        let mut body = Vec::new();
        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            if body.len().saturating_add(chunk.len()) > max_bytes {
                anyhow::bail!("OAuth token response is too large");
            }
            body.extend_from_slice(&chunk);
        }
        Ok(serde_json::from_slice(&body)?)
    }
}

#[cfg(test)]
mod tests {
    use super::GoogleOAuth;

    #[test]
    fn auth_url_contains_state_and_pkce() {
        let url = GoogleOAuth::build_auth_url(
            "http://127.0.0.1:8045/api/accounts/oauth/callback",
            "state-value",
            "challenge-value",
        );

        assert!(url.contains("state=state-value"));
        assert!(url.contains("code_challenge=challenge-value"));
        assert!(url.contains("code_challenge_method=S256"));
    }
}
