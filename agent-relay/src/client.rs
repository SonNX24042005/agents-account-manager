use crate::config::Config;
use crate::models::account::{QuotaBucketInfo, QuotaGroupInfo};
use crate::proxy::selection::Agent;
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicAccountDto {
    pub id: String,
    pub email: String,
    pub expires_at: DateTime<Utc>,
    pub custom_label: Option<String>,
    pub quota_percentage: Option<f64>,
    pub checked_at: Option<DateTime<Utc>>,
    pub error: Option<String>,
    #[serde(default)]
    pub quota_groups: Vec<QuotaGroupInfo>,
    #[serde(default)]
    pub is_active: bool,
    pub rate_limit_until: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NativeAccountDto {
    pub id: String,
    pub email: String,
    #[serde(default)]
    pub is_active: bool,
    #[serde(default)]
    pub quota_groups: Vec<QuotaGroupInfo>,
    pub quota_percentage: Option<f64>,
    pub checked_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub quota_stale: bool,
    #[serde(default)]
    pub quota_status: String,
    pub quota_message: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedAccountDto {
    pub id: String,
    pub agent: Agent,
    pub email: String,
    pub is_active: bool,
    pub quota_percentage: Option<f64>,
    pub quota_groups: Vec<QuotaGroupInfo>,
    pub checked_at: Option<DateTime<Utc>>,
    pub status: String,
    pub error: Option<String>,
}

impl UnifiedAccountDto {
    pub fn primary_reset_countdown(&self) -> Option<String> {
        let mut best_bucket: Option<&QuotaBucketInfo> = None;
        for group in &self.quota_groups {
            for bucket in &group.buckets {
                if bucket.effective_percentage() < 100.0 && bucket.reset_time.is_some() {
                    match best_bucket {
                        None => best_bucket = Some(bucket),
                        Some(prev) => {
                            let curr_pct = bucket.effective_percentage();
                            let prev_pct = prev.effective_percentage();
                            if curr_pct < prev_pct {
                                best_bucket = Some(bucket);
                            } else if (curr_pct - prev_pct).abs() < f64::EPSILON {
                                if let (Some(ref r_curr), Some(ref r_prev)) = (&bucket.reset_time, &prev.reset_time) {
                                    if r_curr < r_prev {
                                        best_bucket = Some(bucket);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        best_bucket.and_then(|b| b.reset_countdown())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionFlagsDto {
    pub antigravity: bool,
    pub codex: bool,
    pub claude: bool,
}

impl SelectionFlagsDto {
    pub fn is_enabled(&self, agent: Agent) -> bool {
        match agent {
            Agent::Antigravity => self.antigravity,
            Agent::Codex => self.codex,
            Agent::Claude => self.claude,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRoutingStateDto {
    pub preference: String,
    pub detected_category: String,
    pub last_detected_source: String,
    pub last_updated_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginStatusDto {
    pub id: String,
    pub status: String,
    pub auth_url: Option<String>,
    pub message: String,
}

#[derive(Clone)]
pub struct ApiClient {
    port: u16,
    master_key: String,
    client: reqwest::Client,
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}

impl ApiClient {
    pub fn new() -> Self {
        let config = Config::default();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            port: config.port,
            master_key: config.master_key,
            client,
        }
    }

    #[allow(dead_code)]
    pub fn port(&self) -> u16 {
        self.port
    }

    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    pub fn is_service_running(&self) -> bool {
        let address = SocketAddr::from(([127, 0, 0, 1], self.port));
        TcpStream::connect_timeout(&address, Duration::from_millis(300)).is_ok()
    }

    pub async fn ensure_service_ready(&self) -> Result<()> {
        if self.is_service_running() {
            return Ok(());
        }

        println!("[aam] Dịch vụ chưa chạy, đang tự động khởi động dịch vụ ngầm...");
        crate::cli::Cli::start_service()?;

        for _ in 0..25 {
            tokio::time::sleep(Duration::from_millis(150)).await;
            if self.is_service_running() {
                // Kiểm tra thêm health check endpoint
                if let Ok(res) = self.client.get(format!("{}/api/health", self.base_url())).send().await {
                    if res.status().is_success() {
                        return Ok(());
                    }
                }
            }
        }

        bail!("Không thể kết nối tới dịch vụ Agent Relay trên cổng {}", self.port);
    }

    pub async fn list_antigravity_accounts(&self) -> Result<Vec<PublicAccountDto>> {
        self.ensure_service_ready().await?;
        let res = self
            .client
            .get(format!("{}/api/accounts", self.base_url()))
            .bearer_auth(&self.master_key)
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            bail!("Lỗi khi lấy danh sách tài khoản Antigravity: {}", error_text);
        }

        let accounts: Vec<PublicAccountDto> = res
            .json()
            .await
            .context("Không giải mã được dữ liệu tài khoản Antigravity")?;
        Ok(accounts)
    }

    pub async fn list_native_accounts(&self, agent: Agent) -> Result<Vec<NativeAccountDto>> {
        self.ensure_service_ready().await?;
        let agent_param = match agent {
            Agent::Antigravity => "antigravity",
            Agent::Codex => "codex",
            Agent::Claude => "claude",
        };

        let res = self
            .client
            .get(format!("{}/api/agents/accounts", self.base_url()))
            .query(&[("agent", agent_param)])
            .bearer_auth(&self.master_key)
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            let error_text = res.text().await.unwrap_or_default();
            bail!("Lỗi khi lấy danh sách tài khoản {}: {}", agent.name(), error_text);
        }

        let accounts: Vec<NativeAccountDto> = res
            .json()
            .await
            .context("Không giải mã được dữ liệu tài khoản")?;
        Ok(accounts)
    }

    pub async fn list_accounts_for_agent(&self, agent: Agent) -> Result<Vec<UnifiedAccountDto>> {
        match agent {
            Agent::Antigravity => {
                let accounts = self.list_antigravity_accounts().await?;
                Ok(accounts
                    .into_iter()
                    .map(|a| {
                        let is_exhausted = a.quota_percentage == Some(0.0)
                            || a.quota_groups.iter().flat_map(|g| &g.buckets).any(|b| b.effective_percentage() <= 0.0);
                        let status = if a.is_active {
                            "Đang hoạt động".to_string()
                        } else if a.error.is_some() {
                            "Lỗi quota".to_string()
                        } else if is_exhausted {
                            "Hết hạn ngạch".to_string()
                        } else {
                            "Sẵn sàng".to_string()
                        };
                        UnifiedAccountDto {
                            id: a.id,
                            agent: Agent::Antigravity,
                            email: a.email,
                            is_active: a.is_active,
                            quota_percentage: a.quota_percentage,
                            quota_groups: a.quota_groups,
                            checked_at: a.checked_at,
                            status,
                            error: a.error,
                        }
                    })
                    .collect())
            }
            native_agent => {
                let accounts = self.list_native_accounts(native_agent).await?;
                Ok(accounts
                    .into_iter()
                    .map(|a| {
                        let status = match a.quota_status.as_str() {
                            "ready" => "Sẵn sàng".to_string(),
                            "exhausted" => "Hết hạn ngạch".to_string(),
                            "stale" => "Chờ cập nhật".to_string(),
                            "error" => "Lỗi quota".to_string(),
                            _ => "Chưa có quota".to_string(),
                        };

                        let effective_quota = if let Some(q) = a.quota_percentage {
                            Some(q)
                        } else if a.quota_status == "exhausted" {
                            Some(0.0)
                        } else {
                            let buckets: Vec<_> = a.quota_groups.iter().flat_map(|g| &g.buckets).collect();
                            if !buckets.is_empty() {
                                buckets.iter().map(|b| b.effective_percentage()).reduce(f64::min)
                            } else {
                                None
                            }
                        };

                        let error = if a.quota_status == "error" {
                            a.error.or(a.quota_message)
                        } else {
                            a.error
                        };

                        UnifiedAccountDto {
                            id: a.id,
                            agent: native_agent,
                            email: a.email,
                            is_active: a.is_active,
                            quota_percentage: effective_quota,
                            quota_groups: a.quota_groups,
                            checked_at: a.checked_at,
                            status,
                            error,
                        }
                    })
                    .collect())
            }
        }
    }

    pub async fn list_all_accounts(&self) -> Result<Vec<UnifiedAccountDto>> {
        let mut all = Vec::new();
        if let Ok(mut anti) = self.list_accounts_for_agent(Agent::Antigravity).await {
            all.append(&mut anti);
        }
        if let Ok(mut codex) = self.list_accounts_for_agent(Agent::Codex).await {
            all.append(&mut codex);
        }
        if let Ok(mut claude) = self.list_accounts_for_agent(Agent::Claude).await {
            all.append(&mut claude);
        }
        Ok(all)
    }

    pub async fn switch_account(&self, agent: Agent, account_id: &str) -> Result<()> {
        self.ensure_service_ready().await?;
        let (endpoint, payload) = match agent {
            Agent::Antigravity => (
                format!("{}/api/accounts/switch", self.base_url()),
                json!({ "account_id": account_id }),
            ),
            native => {
                let agent_str = match native {
                    Agent::Codex => "codex",
                    Agent::Claude => "claude",
                    _ => "antigravity",
                };
                (
                    format!("{}/api/agents/switch", self.base_url()),
                    json!({ "agent": agent_str, "account_id": account_id }),
                )
            }
        };

        let res = self
            .client
            .post(&endpoint)
            .bearer_auth(&self.master_key)
            .json(&payload)
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            let error_val: Value = res.json().await.unwrap_or(Value::Null);
            let msg = error_val["error"].as_str().unwrap_or("Chuyển tài khoản thất bại");
            bail!("{}", msg);
        }
        Ok(())
    }

    pub async fn delete_account(&self, agent: Agent, account_id: &str) -> Result<()> {
        self.ensure_service_ready().await?;
        let (endpoint, payload) = match agent {
            Agent::Antigravity => (
                format!("{}/api/accounts/delete", self.base_url()),
                json!({ "account_id": account_id }),
            ),
            native => {
                let agent_str = match native {
                    Agent::Codex => "codex",
                    Agent::Claude => "claude",
                    _ => "antigravity",
                };
                (
                    format!("{}/api/agents/delete", self.base_url()),
                    json!({ "agent": agent_str, "account_id": account_id }),
                )
            }
        };

        let res = self
            .client
            .post(&endpoint)
            .bearer_auth(&self.master_key)
            .json(&payload)
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            let error_val: Value = res.json().await.unwrap_or(Value::Null);
            let msg = error_val["error"].as_str().unwrap_or("Xóa tài khoản thất bại");
            bail!("{}", msg);
        }
        Ok(())
    }

    pub async fn refresh_quota(&self, agent: Option<Agent>) -> Result<()> {
        self.ensure_service_ready().await?;
        let agents_to_refresh = match agent {
            Some(a) => vec![a],
            None => vec![Agent::Antigravity, Agent::Codex, Agent::Claude],
        };

        for a in agents_to_refresh {
            let agent_str = match a {
                Agent::Antigravity => "antigravity",
                Agent::Codex => "codex",
                Agent::Claude => "claude",
            };
            let _ = self
                .client
                .post(format!("{}/api/agents/refresh", self.base_url()))
                .bearer_auth(&self.master_key)
                .json(&json!({ "agent": agent_str }))
                .send()
                .await;
        }
        Ok(())
    }

    pub async fn auto_select(&self, agent: Agent) -> Result<String> {
        self.ensure_service_ready().await?;
        match agent {
            Agent::Antigravity => {
                let res = self
                    .client
                    .post(format!("{}/api/accounts/auto-select", self.base_url()))
                    .bearer_auth(&self.master_key)
                    .send()
                    .await
                    .context("Không thể kết nối đến máy chủ")?;

                if !res.status().is_success() {
                    let err: Value = res.json().await.unwrap_or(Value::Null);
                    bail!("{}", err["error"].as_str().unwrap_or("Tự động chọn thất bại"));
                }
                let body: Value = res.json().await?;
                let msg = body["message"]
                    .as_str()
                    .unwrap_or("Đã tự động chọn tài khoản tối ưu");
                Ok(msg.to_string())
            }
            native => {
                let agent_str = match native {
                    Agent::Codex => "codex",
                    Agent::Claude => "claude",
                    _ => "antigravity",
                };
                let accounts = self.list_native_accounts(native).await?;
                let best = accounts
                    .iter()
                    .filter(|a| a.quota_percentage.is_some())
                    .max_by(|a, b| {
                        a.quota_percentage
                            .unwrap_or(0.0)
                            .partial_cmp(&b.quota_percentage.unwrap_or(0.0))
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });

                if let Some(best_acc) = best {
                    self.switch_account(native, &best_acc.id).await?;
                    Ok(format!(
                        "Đã chuyển sang tài khoản {} ({}) có hạn ngạch cao nhất ({:.0}%)",
                        best_acc.email,
                        agent_str,
                        best_acc.quota_percentage.unwrap_or(0.0)
                    ))
                } else {
                    bail!("Không tìm thấy tài khoản {} khả dụng để tự động chọn", agent_str);
                }
            }
        }
    }

    pub async fn get_preference(&self) -> Result<ModelRoutingStateDto> {
        self.ensure_service_ready().await?;
        let res = self
            .client
            .get(format!("{}/api/preference", self.base_url()))
            .bearer_auth(&self.master_key)
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            bail!("Không lấy được cấu hình ưu tiên mô hình");
        }

        let state: ModelRoutingStateDto = res.json().await?;
        Ok(state)
    }

    pub async fn set_preference(&self, pref: &str) -> Result<ModelRoutingStateDto> {
        self.ensure_service_ready().await?;
        let normalized = match pref.to_lowercase().as_str() {
            "auto" | "tu_dong" | "tự_động" => "auto",
            "gemini" => "gemini",
            "claude" | "claude_gpt" | "gpt" => "claude_gpt",
            _ => bail!("Chế độ ưu tiên không hợp lệ. Chỉ chấp nhận: auto, gemini, claude_gpt"),
        };

        let res = self
            .client
            .post(format!("{}/api/preference", self.base_url()))
            .bearer_auth(&self.master_key)
            .json(&json!({ "preference": normalized }))
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            bail!("Không cập nhật được cấu hình ưu tiên");
        }

        let state: ModelRoutingStateDto = res.json().await?;
        Ok(state)
    }

    pub async fn get_agent_settings(&self) -> Result<SelectionFlagsDto> {
        self.ensure_service_ready().await?;
        let res = self
            .client
            .get(format!("{}/api/agents/settings", self.base_url()))
            .bearer_auth(&self.master_key)
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            bail!("Không lấy được cài đặt agent");
        }

        let flags: SelectionFlagsDto = res.json().await?;
        Ok(flags)
    }

    pub async fn set_agent_setting(&self, agent: Agent, enabled: bool) -> Result<()> {
        self.ensure_service_ready().await?;
        let agent_str = match agent {
            Agent::Antigravity => "antigravity",
            Agent::Codex => "codex",
            Agent::Claude => "claude",
        };

        let res = self
            .client
            .post(format!("{}/api/agents/settings", self.base_url()))
            .bearer_auth(&self.master_key)
            .json(&json!({ "agent": agent_str, "enabled": enabled }))
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            bail!("Không cập nhật được cài đặt agent {}", agent_str);
        }
        Ok(())
    }

    pub async fn add_antigravity_account(
        &self,
        email: &str,
        access_token: &str,
        refresh_token: Option<&str>,
        expires_in: Option<i64>,
    ) -> Result<()> {
        self.ensure_service_ready().await?;
        let payload = json!({
            "email": email,
            "access_token": access_token,
            "refresh_token": refresh_token,
            "expires_in": expires_in,
        });

        let res = self
            .client
            .post(format!("{}/api/accounts/add", self.base_url()))
            .bearer_auth(&self.master_key)
            .json(&payload)
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            let err: Value = res.json().await.unwrap_or(Value::Null);
            bail!("{}", err["error"].as_str().unwrap_or("Thêm tài khoản thất bại"));
        }
        Ok(())
    }

    pub async fn import_native_account(
        &self,
        agent: Agent,
        email: &str,
        credentials: Option<Value>,
    ) -> Result<()> {
        self.ensure_service_ready().await?;
        let agent_str = match agent {
            Agent::Codex => "codex",
            Agent::Claude => "claude",
            _ => bail!("Chỉ hỗ trợ import cho Codex hoặc Claude"),
        };

        let payload = json!({
            "agent": agent_str,
            "email": email,
            "credentials": credentials,
        });

        let res = self
            .client
            .post(format!("{}/api/agents/import", self.base_url()))
            .bearer_auth(&self.master_key)
            .json(&payload)
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            let err: Value = res.json().await.unwrap_or(Value::Null);
            bail!("{}", err["error"].as_str().unwrap_or("Nhập tài khoản thất bại"));
        }
        Ok(())
    }

    pub async fn start_oauth_flow(&self) -> Result<String> {
        self.ensure_service_ready().await?;
        let res = self
            .client
            .get(format!("{}/api/accounts/oauth/start", self.base_url()))
            .bearer_auth(&self.master_key)
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            bail!("Không thể khởi tạo phiên đăng nhập Google OAuth");
        }

        let body: Value = res.json().await?;
        let url = body["auth_url"]
            .as_str()
            .context("Phản hồi thiếu URL xác thực")?;
        Ok(url.to_string())
    }

    pub async fn start_codex_login(&self) -> Result<LoginStatusDto> {
        self.ensure_service_ready().await?;
        let res = self
            .client
            .post(format!("{}/api/agents/codex/login", self.base_url()))
            .bearer_auth(&self.master_key)
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            let err: Value = res.json().await.unwrap_or(Value::Null);
            bail!("{}", err["error"].as_str().unwrap_or("Không khởi tạo được đăng nhập Codex"));
        }

        let status: LoginStatusDto = res.json().await?;
        Ok(status)
    }

    pub async fn get_codex_login_status(&self) -> Result<Option<LoginStatusDto>> {
        self.ensure_service_ready().await?;
        let res = self
            .client
            .get(format!("{}/api/agents/codex/login", self.base_url()))
            .bearer_auth(&self.master_key)
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            return Ok(None);
        }

        let status: Option<LoginStatusDto> = res.json().await.ok();
        Ok(status)
    }

    #[allow(dead_code)]
    pub async fn cancel_codex_login(&self, id: &str) -> Result<()> {
        self.ensure_service_ready().await?;
        let _ = self
            .client
            .post(format!("{}/api/agents/codex/login/cancel", self.base_url()))
            .bearer_auth(&self.master_key)
            .json(&json!({ "id": id }))
            .send()
            .await;
        Ok(())
    }

    pub async fn reset_cooldowns(&self) -> Result<()> {
        self.ensure_service_ready().await?;
        let res = self
            .client
            .post(format!("{}/api/accounts/reset", self.base_url()))
            .bearer_auth(&self.master_key)
            .send()
            .await
            .context("Không thể kết nối đến máy chủ")?;

        if !res.status().is_success() {
            bail!("Không thể đặt lại thời gian chờ");
        }
        Ok(())
    }
}
