use super::selection::{Agent, SelectionSettings};
use crate::models::account::{QuotaBucketInfo, QuotaGroupInfo};
use crate::storage::secure_file;
use anyhow::{anyhow, bail, ensure, Context, Result};
use base64::Engine;
use chrono::{DateTime, Utc};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    sync::Mutex,
};

const MAX_JSON: usize = 256 * 1024;
const FRESH_SECONDS: i64 = 600;

#[derive(Clone, Serialize, Deserialize)]
struct NativeAccount {
    id: String,
    agent: Agent,
    email: String,
    credentials: Value,
    #[serde(default)]
    quota_groups: Vec<QuotaGroupInfo>,
    #[serde(default)]
    checked_at: Option<DateTime<Utc>>,
    #[serde(default)]
    next_check: Option<DateTime<Utc>>,
    #[serde(default)]
    error: Option<String>,
}

impl NativeAccount {
    fn score(&self) -> Option<f64> {
        if self.error.is_some()
            || self
                .checked_at
                .is_none_or(|t| !(0..FRESH_SECONDS).contains(&(Utc::now() - t).num_seconds()))
        {
            return None;
        }
        if self.agent == Agent::Claude
            && self.credentials["claudeAiOauth"]["expiresAt"]
                .as_i64()
                .is_none_or(|t| t <= Utc::now().timestamp_millis())
        {
            return None;
        }
        let buckets: Vec<_> = self.quota_groups.iter().flat_map(|g| &g.buckets).collect();
        if buckets.is_empty()
            || buckets.iter().any(|b| {
                b.remaining_percentage <= 0.0
                    || b.reset_time.as_ref().is_some_and(|r| {
                        DateTime::parse_from_rfc3339(r).is_ok_and(|t| t <= Utc::now())
                    })
            })
        {
            return None;
        }
        // The tightest window is the actual remaining capacity; never assume a reset restored quota.
        buckets
            .iter()
            .map(|b| b.remaining_percentage)
            .reduce(f64::min)
    }
}

#[derive(Serialize)]
pub struct NativeAccountView {
    id: String,
    email: String,
    is_active: bool,
    quota_groups: Vec<QuotaGroupInfo>,
    quota_percentage: Option<f64>,
    checked_at: Option<DateTime<Utc>>,
    error: Option<String>,
}

pub struct AgentManager {
    path: PathBuf,
    codex_dir: PathBuf,
    claude_dir: PathBuf,
    accounts: Mutex<Vec<NativeAccount>>,
    settings: Arc<SelectionSettings>,
    refresh_gate: Mutex<()>,
}

impl AgentManager {
    pub fn new(data_dir: &Path, settings: Arc<SelectionSettings>) -> Result<Self> {
        let home = dirs::home_dir().context("Không tìm thấy thư mục người dùng")?;
        Self::with_paths(
            data_dir,
            std::env::var_os("CODEX_HOME")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".codex")),
            std::env::var_os("CLAUDE_CONFIG_DIR")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from)
                .unwrap_or_else(|| home.join(".claude")),
            settings,
        )
    }

    fn with_paths(
        data_dir: &Path,
        codex_dir: PathBuf,
        claude_dir: PathBuf,
        settings: Arc<SelectionSettings>,
    ) -> Result<Self> {
        let path = data_dir.join("native-accounts.json");
        let accounts: Vec<NativeAccount> = match std::fs::read(&path) {
            Ok(bytes) => {
                serde_json::from_slice(&bytes).context("Không đọc được kho tài khoản agent")?
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => vec![],
            Err(e) => return Err(e.into()),
        };
        for account in &accounts {
            ensure!(
                account.agent != Agent::Antigravity,
                "Sai loại tài khoản trong kho agent"
            );
            uuid::Uuid::parse_str(&account.id)?;
            validate_credentials(account.agent, &account.credentials)?;
        }
        Ok(Self {
            path,
            codex_dir,
            claude_dir,
            accounts: Mutex::new(accounts),
            settings,
            refresh_gate: Mutex::new(()),
        })
    }

    fn auth_path(&self, agent: Agent) -> Result<PathBuf> {
        match agent {
            Agent::Codex => Ok(self.codex_dir.join("auth.json")),
            Agent::Claude => Ok(self.claude_dir.join(".credentials.json")),
            Agent::Antigravity => bail!("Antigravity dùng kho tài khoản Google"),
        }
    }

    fn ensure_file_storage(&self, agent: Agent) -> Result<()> {
        if agent == Agent::Codex {
            let config_path = self.codex_dir.join("config.toml");
            if config_path.exists() {
                let content = std::fs::read_to_string(config_path)?;
                // Refuse non-file storage; never rewrite the user's config or keyring.
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with('[') {
                        break;
                    }
                    if let Some((key, value)) = line.split_once('=') {
                        if key.trim().trim_matches(['\'', '"']) == "cli_auth_credentials_store" {
                            let value = value.split('#').next().unwrap_or("").trim();
                            ensure!(value == "\"file\"" || value == "'file'", "Codex cần cli_auth_credentials_store = \"file\" trong config.toml để chuyển tài khoản bằng tệp");
                        }
                    }
                }
            }
        }
        #[cfg(target_os = "macos")]
        if agent == Agent::Claude {
            bail!("Claude trên macOS dùng Keychain; chuyển tài khoản bằng tệp hiện hỗ trợ Linux và Windows");
        }
        Ok(())
    }

    fn save(&self, accounts: &[NativeAccount]) -> Result<()> {
        secure_file::atomic_write(&self.path, &serde_json::to_vec_pretty(accounts)?, 0o600)
    }

    fn current_credentials(&self, agent: Agent) -> Result<Option<Value>> {
        let path = self.auth_path(agent)?;
        if !path.exists() {
            return Ok(None);
        }
        read_json(&path).map(Some)
    }

    fn sync_current(&self, agent: Agent, accounts: &mut [NativeAccount]) -> Result<Option<String>> {
        let Some(current) = self.current_credentials(agent)? else {
            return Ok(None);
        };
        let found = accounts
            .iter_mut()
            .find(|a| a.agent == agent && same_identity(agent, &a.credentials, &current));
        if let Some(account) = found {
            validate_credentials(agent, &current)?;
            if account.credentials != current {
                account.credentials = current;
                account.next_check = None;
            }
            Ok(Some(account.id.clone()))
        } else {
            Ok(None)
        }
    }

    pub async fn list(&self, agent: Agent) -> Result<Vec<NativeAccountView>> {
        let mut accounts = self.accounts.lock().await;
        let active = self.sync_current(agent, &mut accounts)?;
        self.save(&accounts)?;
        Ok(accounts.iter().filter(|a| a.agent == agent).map(|a| NativeAccountView {
            id: a.id.clone(), email: a.email.clone(), is_active: active.as_ref() == Some(&a.id),
            quota_groups: a.quota_groups.clone(), quota_percentage: a.score(), checked_at: a.checked_at,
            error: a.error.clone().or_else(|| if active.is_none() && self.current_credentials(agent).ok().flatten().is_some() { Some("Phiên đang dùng chưa có trong danh sách; hãy nhập lại trước khi tự động chọn".into()) } else { None }).or_else(|| if a.score().is_none() { Some("Quota chưa có, đã hết hoặc cần làm mới".into()) } else { None }),
        }).collect())
    }

    pub async fn import(
        &self,
        agent: Agent,
        email: String,
        credentials: Option<Value>,
    ) -> Result<()> {
        self.ensure_file_storage(agent)?;
        let email = email.trim().to_string();
        ensure!(
            !email.is_empty() && email.len() <= 254 && !email.chars().any(char::is_control),
            "Nhập email hoặc tên tài khoản hợp lệ"
        );
        let credentials = match credentials {
            Some(value) => value,
            None => self
                .current_credentials(agent)?
                .context("Chưa có tệp đăng nhập. Hãy đăng nhập bằng CLI của agent trước")?,
        };
        validate_credentials(agent, &credentials)?;
        let mut accounts = self.accounts.lock().await;
        let mut next = accounts.clone();
        if let Some(account) = next
            .iter_mut()
            .find(|a| a.agent == agent && same_identity(agent, &a.credentials, &credentials))
        {
            account.email = email;
            account.credentials = credentials;
            account.next_check = None;
            account.checked_at = None;
            account.error = None;
        } else {
            ensure!(
                next.iter().filter(|a| a.agent == agent).count() < 100,
                "Tối đa 100 tài khoản cho mỗi agent"
            );
            next.push(NativeAccount {
                id: uuid::Uuid::new_v4().to_string(),
                agent,
                email,
                credentials,
                quota_groups: vec![],
                checked_at: None,
                next_check: None,
                error: None,
            });
        }
        self.save(&next)?;
        *accounts = next;
        Ok(())
    }

    pub async fn delete(&self, agent: Agent, id: &str) -> Result<()> {
        let mut accounts = self.accounts.lock().await;
        let mut next = accounts.clone();
        let pos = next
            .iter()
            .position(|a| a.agent == agent && a.id == id)
            .context("Không tìm thấy tài khoản của agent này")?;
        next.remove(pos);
        self.save(&next)?;
        *accounts = next;
        Ok(())
    }

    fn activate(&self, account: &NativeAccount) -> Result<()> {
        self.ensure_file_storage(account.agent)?;
        let path = self.auth_path(account.agent)?;
        if let Some(current) = self.current_credentials(account.agent)? {
            if current == account.credentials {
                return Ok(());
            }
            // A fixed backup prevents unbounded secret-file accumulation.
            let backup = path.with_extension("aam.bak");
            secure_file::atomic_write(&backup, &serde_json::to_vec_pretty(&current)?, 0o600)?;
        }
        secure_file::atomic_write(
            &path,
            &serde_json::to_vec_pretty(&account.credentials)?,
            0o600,
        )
    }

    pub async fn switch(&self, agent: Agent, id: &str) -> Result<()> {
        let mut accounts = self.accounts.lock().await;
        self.sync_current(agent, &mut accounts)?;
        self.save(&accounts)?;
        let account = accounts
            .iter()
            .find(|a| a.agent == agent && a.id == id)
            .context("Không tìm thấy tài khoản của agent này")?;
        self.activate(account)
    }

    pub async fn auto_select(&self, agent: Agent) -> Result<()> {
        // Hold the setting until the write finishes, so disabling waits out an in-flight switch.
        let flags = self.settings.flags.lock().await;
        if !flags.enabled(agent) {
            return Ok(());
        }
        let mut accounts = self.accounts.lock().await;
        let active = self.sync_current(agent, &mut accounts)?;
        self.save(&accounts)?;
        // An unknown local login may have refreshed outside the manager; require import instead of overwriting it.
        if self.current_credentials(agent)?.is_some() && active.is_none() {
            bail!("Đăng nhập hiện tại chưa có trong danh sách; hãy nhập lại tài khoản đang dùng");
        }
        let best = accounts
            .iter()
            .filter(|a| a.agent == agent && a.score().is_some())
            .max_by(|a, b| {
                a.score()
                    .partial_cmp(&b.score())
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| {
                        (active.as_ref() == Some(&a.id)).cmp(&(active.as_ref() == Some(&b.id)))
                    })
            });
        if let Some(best) = best {
            if active.as_ref() != Some(&best.id) {
                self.activate(best)?;
            }
        }
        Ok(())
    }

    pub async fn refresh(&self, client: &reqwest::Client) {
        let Ok(_gate) = self.refresh_gate.try_lock() else {
            return;
        };
        for agent in [Agent::Codex, Agent::Claude] {
            let snapshot = {
                let mut accounts = self.accounts.lock().await;
                if self.sync_current(agent, &mut accounts).is_err() {
                    continue;
                }
                if self.save(&accounts).is_err() {
                    continue;
                }
                accounts
                    .iter()
                    .filter(|a| a.agent == agent && a.next_check.is_none_or(|t| t <= Utc::now()))
                    .cloned()
                    .collect::<Vec<_>>()
            };
            for account in snapshot {
                let result = match agent {
                    Agent::Codex => {
                        fetch_codex_quota(&account.credentials, self.path.parent().unwrap()).await
                    }
                    Agent::Claude => Ok((
                        fetch_claude_quota(client, &account.credentials).await,
                        account.credentials.clone(),
                    )),
                    Agent::Antigravity => unreachable!(),
                };
                let mut accounts = self.accounts.lock().await;
                if let Some(stored) = accounts
                    .iter_mut()
                    .find(|a| a.id == account.id && a.credentials == account.credentials)
                {
                    stored.next_check = Some(Utc::now() + chrono::Duration::seconds(300));
                    match result {
                        Ok((groups, credentials)) => {
                            // Only update the active cache if another process has not replaced it meanwhile.
                            if credentials != account.credentials
                                && self.current_credentials(agent).ok().flatten().as_ref()
                                    == Some(&account.credentials)
                            {
                                if let Err(e) = secure_file::atomic_write(
                                    &self.auth_path(agent).unwrap(),
                                    &serde_json::to_vec(&credentials).unwrap(),
                                    0o600,
                                ) {
                                    stored.error =
                                        Some(format!("Không đồng bộ được phiên mới: {e}"));
                                    continue;
                                }
                            }
                            stored.credentials = credentials;
                            match groups {
                                Ok(groups) => {
                                    stored.quota_groups = groups;
                                    stored.checked_at = Some(Utc::now());
                                    stored.error = None;
                                }
                                Err(e) => {
                                    if let Some(retry) = e.downcast_ref::<QuotaRetry>() {
                                        stored.next_check =
                                            Some(Utc::now() + chrono::Duration::seconds(retry.0));
                                    }
                                    stored.error = Some(e.to_string());
                                }
                            }
                        }
                        Err(e) => stored.error = Some(e.to_string()),
                    }
                    if let Err(e) = self.save(&accounts) {
                        tracing::warn!("Không lưu được quota agent: {e}");
                    }
                }
            }
            if let Err(e) = self.auto_select(agent).await {
                tracing::warn!("{}: {e}", agent.name());
            }
        }
    }
}

pub(super) fn read_json(path: &Path) -> Result<Value> {
    use std::io::Read;
    let file = std::fs::File::open(path).context("Không mở được tệp đăng nhập")?;
    let mut bytes = Vec::new();
    file.take(MAX_JSON as u64 + 1).read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= MAX_JSON, "Tệp đăng nhập quá lớn");
    serde_json::from_slice(&bytes).context("Tệp đăng nhập không phải JSON hợp lệ")
}

fn nonempty(value: &Value) -> bool {
    value.as_str().is_some_and(|s| !s.trim().is_empty())
}

fn validate_credentials(agent: Agent, value: &Value) -> Result<()> {
    ensure!(
        serde_json::to_vec(value)?.len() <= MAX_JSON,
        "Thông tin đăng nhập quá lớn"
    );
    let valid = match agent {
        Agent::Codex => {
            nonempty(&value["OPENAI_API_KEY"])
                || (nonempty(&value["tokens"]["access_token"])
                    && nonempty(&value["tokens"]["refresh_token"])
                    && nonempty(&value["tokens"]["id_token"]))
        }
        Agent::Claude => {
            nonempty(&value["claudeAiOauth"]["accessToken"])
                && nonempty(&value["claudeAiOauth"]["refreshToken"])
                && value["claudeAiOauth"]["expiresAt"].as_i64().is_some()
        }
        Agent::Antigravity => false,
    };
    ensure!(
        valid,
        "Thông tin đăng nhập không đúng định dạng của {}",
        agent.name()
    );
    Ok(())
}

pub(super) fn jwt_claims(token: &Value) -> Option<Value> {
    let payload = token.as_str()?.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn codex_identity(value: &Value) -> Option<String> {
    let claims = jwt_claims(&value["tokens"]["id_token"])?;
    Some(format!(
        "{}:{}",
        claims["sub"].as_str()?,
        value["tokens"]["account_id"].as_str().unwrap_or("")
    ))
}

fn same_identity(agent: Agent, a: &Value, b: &Value) -> bool {
    if agent == Agent::Codex {
        if let (Some(a), Some(b)) = (codex_identity(a), codex_identity(b)) {
            return a == b;
        }
    }
    let paths = match agent {
        Agent::Codex => vec![
            "/tokens/refresh_token",
            "/tokens/access_token",
            "/OPENAI_API_KEY",
        ],
        Agent::Claude => vec!["/claudeAiOauth/refreshToken", "/claudeAiOauth/accessToken"],
        Agent::Antigravity => vec![],
    };
    paths.iter().any(|p| {
        a.pointer(p)
            .is_some_and(|v| nonempty(v) && Some(v) == b.pointer(p))
    })
}

fn quota_bucket(window: String, used: f64, reset: Option<i64>) -> Result<QuotaBucketInfo> {
    ensure!(used.is_finite() && used >= 0.0, "Quota trả về không hợp lệ");
    Ok(QuotaBucketInfo {
        window,
        remaining_percentage: (100.0 - used).clamp(0.0, 100.0),
        reset_time: reset
            .and_then(|r| DateTime::<Utc>::from_timestamp(r, 0))
            .map(|r| r.to_rfc3339()),
    })
}

fn parse_codex_quota(value: &Value) -> Result<Vec<QuotaGroupInfo>> {
    let limits = value
        .get("rateLimitsByLimitId")
        .and_then(|m| m.get("codex"))
        .or_else(|| value.get("rateLimits"))
        .context("Codex chưa trả về quota")?;
    let mut buckets = vec![];
    for key in ["primary", "secondary"] {
        let window = &limits[key];
        if window.is_null() {
            continue;
        }
        let minutes = window["windowDurationMins"].as_i64();
        let name = match minutes {
            Some(300) => "FIVE_HOUR".into(),
            Some(10080) => "WEEKLY".into(),
            Some(n) => format!("{n} phút"),
            None => key.into(),
        };
        buckets.push(quota_bucket(
            name,
            window["usedPercent"]
                .as_f64()
                .context("Codex trả về quota không hợp lệ")?,
            window["resetsAt"].as_i64(),
        )?);
    }
    ensure!(
        !buckets.is_empty(),
        "Tài khoản Codex chưa có quota gói ChatGPT; API key không có quota phần trăm"
    );
    Ok(vec![QuotaGroupInfo {
        name: "Codex".into(),
        buckets,
    }])
}

fn parse_claude_quota(value: &Value) -> Result<Vec<QuotaGroupInfo>> {
    let mut buckets = vec![];
    for (key, label) in [("five_hour", "FIVE_HOUR"), ("seven_day", "WEEKLY")] {
        let window = &value[key];
        if window.is_null() {
            continue;
        }
        let reset = window["resets_at"]
            .as_str()
            .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
            .map(|d| d.timestamp());
        buckets.push(quota_bucket(
            label.into(),
            window["utilization"]
                .as_f64()
                .context("Claude trả về quota không hợp lệ")?,
            reset,
        )?);
    }
    ensure!(!buckets.is_empty(), "Claude chưa trả về quota gói thuê bao");
    Ok(vec![QuotaGroupInfo {
        name: "Claude".into(),
        buckets,
    }])
}

#[derive(Debug)]
struct QuotaRetry(i64);
impl std::fmt::Display for QuotaRetry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Claude giới hạn tần suất đọc quota; thử lại sau {} giây",
            self.0
        )
    }
}
impl std::error::Error for QuotaRetry {}

async fn fetch_claude_quota(
    client: &reqwest::Client,
    credentials: &Value,
) -> Result<Vec<QuotaGroupInfo>> {
    let oauth = &credentials["claudeAiOauth"];
    ensure!(
        oauth["expiresAt"]
            .as_i64()
            .is_some_and(|t| t > Utc::now().timestamp_millis()),
        "Phiên Claude hết hạn; đăng nhập lại trong Claude rồi nhập lại tài khoản"
    );
    let response = client
        .get("https://api.anthropic.com/api/oauth/usage")
        .timeout(Duration::from_secs(20))
        .bearer_auth(
            oauth["accessToken"]
                .as_str()
                .context("Thiếu token Claude")?,
        )
        .header("anthropic-beta", "oauth-2025-04-20")
        .send()
        .await
        .map_err(|_| anyhow!("Không kết nối được dịch vụ quota Claude"))?;
    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry = response
            .headers()
            .get("retry-after")
            .and_then(|h| h.to_str().ok())
            .and_then(|v| {
                v.parse::<i64>().ok().or_else(|| {
                    DateTime::parse_from_rfc2822(v)
                        .ok()
                        .map(|d| (d.with_timezone(&Utc) - Utc::now()).num_seconds())
                })
            })
            .unwrap_or(300)
            .clamp(300, 86400);
        return Err(QuotaRetry(retry).into());
    }
    ensure!(
        response.status().is_success(),
        "Không đọc được quota Claude (HTTP {}); sẽ thử lại sau 5 phút",
        response.status().as_u16()
    );
    let mut bytes = vec![];
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|_| anyhow!("Lỗi đọc quota Claude"))?;
        ensure!(
            bytes.len() + chunk.len() <= MAX_JSON,
            "Phản hồi quota quá lớn"
        );
        bytes.extend_from_slice(&chunk);
    }
    parse_claude_quota(&serde_json::from_slice(&bytes).context("Quota Claude không phải JSON")?)
}

pub(super) struct ProbeDir(pub(super) PathBuf);
impl Drop for ProbeDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn fetch_codex_quota(
    credentials: &Value,
    data_dir: &Path,
) -> Result<(Result<Vec<QuotaGroupInfo>>, Value)> {
    ensure!(
        !nonempty(&credentials["OPENAI_API_KEY"]),
        "API key không có quota gói ChatGPT để so sánh tự động"
    );
    let dir = ProbeDir(data_dir.join(format!(".codex-probe-{}", uuid::Uuid::new_v4())));
    std::fs::create_dir(&dir.0)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o700))?;
    }
    secure_file::atomic_write(
        &dir.0.join("auth.json"),
        &serde_json::to_vec(credentials)?,
        0o600,
    )?;
    let mut command = tokio::process::Command::new("codex");
    command
        .args(["app-server", "-c", "cli_auth_credentials_store=\"file\""])
        .env("CODEX_HOME", &dir.0)
        .env("TOKIO_WORKER_THREADS", "2")
        .env_remove("OPENAI_API_KEY")
        .env_remove("CODEX_API_KEY")
        .env_remove("CODEX_ACCESS_TOKEN")
        .current_dir(&dir.0)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    let mut child = command.spawn().context(
        "Không chạy được codex app-server; hãy cài Codex CLI và kiểm tra PATH của dịch vụ",
    )?;
    let result = tokio::time::timeout(Duration::from_secs(25), async {
        let mut input = child.stdin.take().context("Không mở được stdin Codex")?;
        let output = child.stdout.take().context("Không mở được stdout Codex")?;
        let mut reader = BufReader::new(output.take((MAX_JSON * 4) as u64));
        input.write_all(format!("{}\n", json!({"id":0,"method":"initialize","params":{"clientInfo":{"name":"agent_account_manager","version":"1.0.3"}}})).as_bytes()).await?;
        read_rpc(&mut reader, 0).await?;
        input.write_all(b"{\"method\":\"initialized\"}\n").await?;
        let needs_refresh = jwt_claims(&credentials["tokens"]["access_token"])
            .and_then(|claims| claims["exp"].as_i64())
            .is_some_and(|expires| expires <= Utc::now().timestamp() + 300);
        if needs_refresh {
            input.write_all(b"{\"id\":1,\"method\":\"account/read\",\"params\":{\"refreshToken\":true}}\n").await?;
            read_rpc(&mut reader, 1).await?;
        }
        input.write_all(b"{\"id\":2,\"method\":\"account/rateLimits/read\"}\n").await?;
        let result = read_rpc(&mut reader, 2).await?;
        parse_codex_quota(&result)
    }).await.unwrap_or_else(|_| Err(anyhow!("Đọc quota Codex quá thời gian chờ")));
    let _ = child.kill().await;
    let _ = child.wait().await;
    let refreshed = read_json(&dir.0.join("auth.json"))?;
    validate_credentials(Agent::Codex, &refreshed)?;
    Ok((result, refreshed))
}

pub(super) async fn read_rpc<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R, id: i64) -> Result<Value> {
    loop {
        let mut line = String::new();
        ensure!(
            reader.read_line(&mut line).await? > 0,
            "Codex app-server đã dừng"
        );
        ensure!(line.len() <= MAX_JSON, "Phản hồi Codex quá lớn");
        let message: Value = serde_json::from_str(&line).context("Phản hồi Codex không hợp lệ")?;
        if message["id"].as_i64() == Some(id) {
            ensure!(
                message.get("error").is_none(),
                "Codex từ chối đọc quota; kiểm tra đăng nhập và phiên bản CLI"
            );
            return message
                .get("result")
                .cloned()
                .context("Phản hồi Codex thiếu kết quả");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture {
        dir: PathBuf,
        manager: AgentManager,
    }
    impl Fixture {
        fn new() -> Self {
            let dir = std::env::temp_dir().join(format!("aam-agents-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&dir).unwrap();
            let settings = Arc::new(SelectionSettings::new(&dir).unwrap());
            let manager =
                AgentManager::with_paths(&dir, dir.join("codex"), dir.join("claude"), settings)
                    .unwrap();
            Self { dir, manager }
        }
        async fn add(&self, agent: Agent, name: &str, quota: f64) -> String {
            self.manager
                .import(agent, name.into(), Some(credentials(agent, name)))
                .await
                .unwrap();
            let mut accounts = self.manager.accounts.lock().await;
            let account = accounts
                .iter_mut()
                .find(|a| a.agent == agent && a.email == name)
                .unwrap();
            account.quota_groups = vec![QuotaGroupInfo {
                name: agent.name().into(),
                buckets: vec![quota_bucket("FIVE_HOUR".into(), 100.0 - quota, None).unwrap()],
            }];
            account.checked_at = Some(Utc::now());
            account.id.clone()
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn credentials(agent: Agent, key: &str) -> Value {
        match agent {
            Agent::Codex => {
                json!({"tokens":{"access_token":format!("access-{key}"),"refresh_token":format!("refresh-{key}"),"id_token":"fixture"}})
            }
            Agent::Claude => {
                json!({"claudeAiOauth":{"accessToken":format!("access-{key}"),"refreshToken":format!("refresh-{key}"),"expiresAt":Utc::now().timestamp_millis()+3600000}})
            }
            _ => unreachable!(),
        }
    }

    #[tokio::test]
    async fn independent_selection_manual_mode_and_restart() {
        let f = Fixture::new();
        let low = f.add(Agent::Codex, "same@example.com", 20.0).await;
        let high = f.add(Agent::Codex, "other@example.com", 80.0).await;
        let claude = f.add(Agent::Claude, "same@example.com", 99.0).await;
        f.manager.switch(Agent::Codex, &low).await.unwrap();
        f.manager.auto_select(Agent::Codex).await.unwrap();
        assert!(f
            .manager
            .list(Agent::Codex)
            .await
            .unwrap()
            .iter()
            .any(|a| a.id == low && a.is_active));
        f.manager.settings.set(Agent::Codex, true).await.unwrap();
        f.manager.auto_select(Agent::Codex).await.unwrap();
        assert!(f
            .manager
            .list(Agent::Codex)
            .await
            .unwrap()
            .iter()
            .any(|a| a.id == high && a.is_active));
        assert!(!f.manager.auth_path(Agent::Claude).unwrap().exists());
        assert!(f.manager.switch(Agent::Codex, &claude).await.is_err());
        f.manager.settings.set(Agent::Codex, false).await.unwrap();
        f.manager.switch(Agent::Codex, &low).await.unwrap();
        f.manager.auto_select(Agent::Codex).await.unwrap();
        assert!(f
            .manager
            .list(Agent::Codex)
            .await
            .unwrap()
            .iter()
            .any(|a| a.id == low && a.is_active));
        let settings = Arc::new(SelectionSettings::new(&f.dir).unwrap());
        let flags = settings.flags.lock().await.clone();
        assert!(!flags.codex && !flags.claude && flags.antigravity);
        let reloaded =
            AgentManager::with_paths(&f.dir, f.dir.join("codex"), f.dir.join("claude"), settings)
                .unwrap();
        assert_eq!(reloaded.list(Agent::Codex).await.unwrap().len(), 2);
        assert_eq!(reloaded.list(Agent::Claude).await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn exhausted_stale_unknown_and_failed_quota_are_not_selected() {
        let f = Fixture::new();
        let good = f.add(Agent::Claude, "good", 30.0).await;
        f.add(Agent::Claude, "weekly-empty", 100.0).await;
        f.add(Agent::Claude, "stale", 100.0).await;
        f.add(Agent::Claude, "unknown", 100.0).await;
        f.add(Agent::Claude, "failed", 100.0).await;
        {
            let mut accounts = f.manager.accounts.lock().await;
            for a in accounts.iter_mut() {
                match a.email.as_str() {
                    "weekly-empty" => a.quota_groups[0]
                        .buckets
                        .push(quota_bucket("WEEKLY".into(), 100.0, None).unwrap()),
                    "stale" => {
                        a.checked_at = Some(Utc::now() - chrono::Duration::seconds(FRESH_SECONDS))
                    }
                    "unknown" => a.checked_at = None,
                    "failed" => a.error = Some("HTTP 429".into()),
                    _ => (),
                }
            }
        }
        f.manager.settings.set(Agent::Claude, true).await.unwrap();
        f.manager.auto_select(Agent::Claude).await.unwrap();
        assert!(f
            .manager
            .list(Agent::Claude)
            .await
            .unwrap()
            .iter()
            .any(|a| a.id == good && a.is_active));
    }

    #[tokio::test]
    async fn ties_keep_active_and_unknown_login_is_not_overwritten() {
        let f = Fixture::new();
        let a = f.add(Agent::Codex, "a", 50.0).await;
        f.add(Agent::Codex, "b", 50.0).await;
        f.manager.switch(Agent::Codex, &a).await.unwrap();
        f.manager.settings.set(Agent::Codex, true).await.unwrap();
        f.manager.auto_select(Agent::Codex).await.unwrap();
        assert!(f
            .manager
            .list(Agent::Codex)
            .await
            .unwrap()
            .iter()
            .any(|v| v.id == a && v.is_active));
        let path = f.manager.auth_path(Agent::Codex).unwrap();
        let unknown = credentials(Agent::Codex, "external");
        secure_file::atomic_write(&path, &serde_json::to_vec(&unknown).unwrap(), 0o600).unwrap();
        assert!(f.manager.auto_select(Agent::Codex).await.is_err());
        assert_eq!(read_json(&path).unwrap(), unknown);
    }

    #[tokio::test]
    async fn import_validates_provider_and_does_not_expose_credentials() {
        let f = Fixture::new();
        assert!(f
            .manager
            .import(
                Agent::Claude,
                "x".into(),
                Some(credentials(Agent::Codex, "x"))
            )
            .await
            .is_err());
        assert!(f
            .manager
            .import(
                Agent::Codex,
                "x".into(),
                Some(json!({"access_token":"bad"}))
            )
            .await
            .is_err());
        let id = f.add(Agent::Codex, "x", 40.0).await;
        f.manager
            .import(
                Agent::Codex,
                "renamed".into(),
                Some(credentials(Agent::Codex, "x")),
            )
            .await
            .unwrap();
        let view = f.manager.list(Agent::Codex).await.unwrap();
        assert_eq!(view.len(), 1);
        let public = serde_json::to_string(&view).unwrap();
        for secret in ["access-x", "refresh-x", "credentials", "id_token"] {
            assert!(!public.contains(secret));
        }
        assert!(f.manager.delete(Agent::Claude, &id).await.is_err());
        assert!(f
            .manager
            .switch(Agent::Codex, "../../outside")
            .await
            .is_err());
        f.manager.delete(Agent::Codex, &id).await.unwrap();
        assert!(f.manager.list(Agent::Codex).await.unwrap().is_empty());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(&f.manager.path)
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
        }
    }

    #[tokio::test]
    async fn failed_switch_preserves_original_login_and_keyring_mode_is_rejected() {
        let f = Fixture::new();
        let a = f.add(Agent::Codex, "a", 20.0).await;
        let b = f.add(Agent::Codex, "b", 80.0).await;
        f.manager.switch(Agent::Codex, &a).await.unwrap();
        let path = f.manager.auth_path(Agent::Codex).unwrap();
        let original = read_json(&path).unwrap();
        std::fs::create_dir(path.with_extension("aam.bak")).unwrap();
        assert!(f.manager.switch(Agent::Codex, &b).await.is_err());
        assert_eq!(read_json(&path).unwrap(), original);
        std::fs::write(
            f.manager.codex_dir.join("config.toml"),
            "cli_auth_credentials_store = 'keyring'\n",
        )
        .unwrap();
        assert!(f.manager.ensure_file_storage(Agent::Codex).is_err());
    }

    #[test]
    fn quota_parsers_handle_windows_null_and_invalid_payloads() {
        let codex = parse_codex_quota(&json!({"rateLimitsByLimitId":{"codex":{"primary":{"usedPercent":25,"windowDurationMins":300},"secondary":{"usedPercent":80,"windowDurationMins":10080}}}})).unwrap();
        assert_eq!(codex[0].buckets[0].remaining_percentage, 75.0);
        assert_eq!(codex[0].buckets[1].window, "WEEKLY");
        let weekly = parse_codex_quota(&json!({"rateLimits":{"primary":{"usedPercent":25,"windowDurationMins":10080},"secondary":null}})).unwrap();
        assert_eq!(weekly[0].buckets[0].window, "WEEKLY");
        let claude = parse_claude_quota(&json!({"five_hour":{"utilization":10,"resets_at":"2030-01-01T00:00:00Z"},"seven_day":null})).unwrap();
        assert_eq!(claude[0].buckets[0].remaining_percentage, 90.0);
        assert!(parse_claude_quota(&json!({"five_hour":{"utilization":"bad"}})).is_err());
        assert!(parse_codex_quota(&json!({"rateLimits":{}})).is_err());
        assert!(parse_claude_quota(&json!({})).is_err());
        assert!(quota_bucket("5h".into(), -1.0, None).is_err());
    }

    #[tokio::test]
    async fn reset_windows_and_expired_credentials_require_a_new_quota_read() {
        let f = Fixture::new();
        f.add(Agent::Claude, "expired", 90.0).await;
        let mut accounts = f.manager.accounts.lock().await;
        let account = &mut accounts[0];
        account.credentials["claudeAiOauth"]["expiresAt"] =
            json!(Utc::now().timestamp_millis() - 1);
        assert!(account.score().is_none());
        account.credentials["claudeAiOauth"]["expiresAt"] =
            json!(Utc::now().timestamp_millis() + 3600000);
        account.quota_groups[0].buckets[0].reset_time =
            Some((Utc::now() - chrono::Duration::seconds(1)).to_rfc3339());
        assert!(account.score().is_none());
        account.quota_groups[0].buckets[0].reset_time =
            Some((Utc::now() + chrono::Duration::hours(1)).to_rfc3339());
        assert_eq!(account.score(), Some(90.0));
    }

    #[tokio::test]
    async fn rpc_ignores_notifications_and_reports_errors_without_secret_details() {
        let data = b"{\"method\":\"notification\"}\n{\"id\":1,\"result\":{\"ok\":true}}\n";
        let mut reader = BufReader::new(&data[..]);
        assert_eq!(read_rpc(&mut reader, 1).await.unwrap(), json!({"ok":true}));
        let data = b"{\"id\":1,\"error\":{\"message\":\"secret token\"}}\n";
        let mut reader = BufReader::new(&data[..]);
        assert!(!read_rpc(&mut reader, 1)
            .await
            .unwrap_err()
            .to_string()
            .contains("secret token"));
    }
}
