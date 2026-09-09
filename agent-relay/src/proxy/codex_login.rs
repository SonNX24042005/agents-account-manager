use super::{
    agents::{jwt_claims, read_json, read_rpc, AgentManager, ProbeDir},
    selection::Agent,
};
use anyhow::{bail, ensure, Context, Result};
use serde::Serialize;
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    process::Stdio,
    sync::Arc,
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    sync::{oneshot, Mutex},
    task::JoinHandle,
};

#[derive(Clone, Serialize)]
pub struct LoginStatus {
    pub id: String,
    pub status: String,
    pub auth_url: Option<String>,
    pub message: String,
}

struct Flow {
    status: Arc<Mutex<LoginStatus>>,
    cancel: Option<oneshot::Sender<()>>,
    worker: JoinHandle<()>,
}
impl Drop for Flow {
    fn drop(&mut self) {
        self.worker.abort();
    }
}

pub struct CodexLogin {
    data_dir: PathBuf,
    program: PathBuf,
    flow: Mutex<Option<Flow>>,
}

impl CodexLogin {
    pub fn new(data_dir: PathBuf) -> Self {
        let program = find_codex_program();
        Self {
            data_dir,
            program,
            flow: Mutex::new(None),
        }
    }

    pub async fn status(&self) -> Option<LoginStatus> {
        match self.flow.lock().await.as_ref() {
            Some(flow) => Some(flow.status.lock().await.clone()),
            None => None,
        }
    }

    pub async fn start(
        &self,
        agents: Arc<AgentManager>,
        client: reqwest::Client,
    ) -> Result<LoginStatus> {
        let mut current = self.flow.lock().await;
        if let Some(flow) = current.as_ref() {
            let status = flow.status.lock().await.clone();
            if status.status == "pending" {
                return Ok(status);
            }
        }

        // Khởi động phiên đăng nhập Codex qua CLI
        let session = tokio::time::timeout(
            Duration::from_secs(20),
            start_session(&self.data_dir, &self.program),
        )
        .await
        .context("Không tạo được liên kết đăng nhập Codex trong thời gian chờ")??;

        let status = LoginStatus {
            id: uuid::Uuid::new_v4().to_string(),
            status: "pending".into(),
            auth_url: Some(session.auth_url.clone()),
            message:
                "Mở liên kết và hoàn tất đăng nhập trong trình duyệt. Liên kết có hiệu lực trong 10 phút."
                    .into(),
        };
        let shared = Arc::new(Mutex::new(status.clone()));
        let worker_status = shared.clone();
        let (cancel, cancelled) = oneshot::channel();
        let worker = tokio::spawn(async move {
            let mut session = session;
            let outcome = tokio::select! {
                biased;
                _ = cancelled => Err(anyhow::anyhow!("Đã hủy đăng nhập Codex")),
                result = tokio::time::timeout(Duration::from_secs(600), session.finish(&agents)) => {
                    result.unwrap_or_else(|_| Err(anyhow::anyhow!("Liên kết đăng nhập Codex đã hết hạn; hãy tạo liên kết mới")))
                }
            };
            // Giải phóng tiến trình và thư mục tạm trước khi báo hoàn tất
            session.stop().await;
            drop(session);
            let mut status = worker_status.lock().await;
            status.auth_url = None;
            match outcome {
                Ok(()) => {
                    // Quét lại quota ngay khi hoàn tất đăng nhập
                    let _ = agents.refresh(&client).await;
                    status.status = "completed".into();
                    status.message =
                        "Đã đăng nhập và thêm tài khoản Codex thành công.".into();
                }
                Err(e) => {
                    status.status = "failed".into();
                    status.message = e.to_string();
                }
            }
        });
        *current = Some(Flow {
            status: shared,
            cancel: Some(cancel),
            worker,
        });
        Ok(status)
    }

    pub async fn cancel(&self, id: &str) -> Result<()> {
        let mut current = self.flow.lock().await;
        let flow = current.as_mut().context("Không có phiên đăng nhập Codex")?;
        let status = flow.status.lock().await.clone();
        ensure!(status.id == id, "Phiên đăng nhập không khớp");
        if status.status == "pending" {
            if let Some(cancel) = flow.cancel.take() {
                let _ = cancel.send(());
            }
            let _ = (&mut flow.worker).await;
            let mut status = flow.status.lock().await;
            if status.status != "completed" {
                status.status = "cancelled".into();
                status.message = "Đã hủy đăng nhập.".into();
            }
        }
        Ok(())
    }
}

fn find_codex_program() -> PathBuf {
    if let Some(path) = std::env::var_os("CODEX_PATH") {
        let pb = PathBuf::from(path);
        if pb.is_file() {
            return pb;
        }
    }
    if let Some(home) = dirs::home_dir() {
        let local_codex = home.join(".local/bin/codex");
        if local_codex.is_file() {
            return local_codex;
        }
    }
    PathBuf::from("codex")
}

type Output = BufReader<tokio::io::Take<tokio::process::ChildStdout>>;
struct Session {
    child: tokio::process::Child,
    input: tokio::process::ChildStdin,
    output: Output,
    dir: ProbeDir,
    login_id: String,
    auth_url: String,
}

impl Session {
    async fn stop(&mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }

    async fn finish(&mut self, agents: &AgentManager) -> Result<()> {
        loop {
            let mut line = String::new();
            ensure!(
                self.output.read_line(&mut line).await? > 0,
                "Tiến trình đăng nhập Codex đã dừng"
            );
            ensure!(line.len() <= 256 * 1024, "Phản hồi đăng nhập Codex quá lớn");
            let message: Value =
                serde_json::from_str(&line).context("Phản hồi đăng nhập Codex không hợp lệ")?;
            if completed(&message, &self.login_id)? {
                break;
            }
        }
        self.input
            .write_all(b"{\"id\":3,\"method\":\"account/read\",\"params\":{\"refreshToken\":false}}\n")
            .await?;
        let result = read_rpc(&mut self.output, 3).await?;
        let credentials = read_json(&self.dir.0.join("auth.json"))?;
        let email_from_result = result["account"]["email"]
            .as_str()
            .filter(|s| !s.trim().is_empty());
        let email_from_jwt = jwt_claims(&credentials["tokens"]["id_token"])
            .and_then(|c| c["email"].as_str().map(|s| s.to_string()));
        let email = email_from_result
            .map(|s| s.to_string())
            .or(email_from_jwt)
            .unwrap_or_else(|| "Tài khoản Codex".to_string());
        agents
            .import(Agent::Codex, email, Some(credentials))
            .await
    }
}

fn completed(message: &Value, login_id: &str) -> Result<bool> {
    if message["method"] != "account/login/completed"
        || message["params"]["loginId"] != login_id
    {
        return Ok(false);
    }
    if message["params"]["success"] != true {
        let err = message["params"]["error"]
            .as_str()
            .unwrap_or("Đăng nhập Codex không thành công hoặc đã bị hủy");
        bail!("{err}");
    }
    Ok(true)
}

fn validate_auth_url(url: &str) -> Result<()> {
    let parsed = reqwest::Url::parse(url).context("Link đăng nhập Codex không hợp lệ")?;
    ensure!(
        parsed.scheme() == "https"
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed.port_or_known_default() == Some(443)
            && parsed.host_str().is_some_and(|h| {
                h == "openai.com"
                    || h.ends_with(".openai.com")
                    || h == "chatgpt.com"
                    || h.ends_with(".chatgpt.com")
            }),
        "Codex trả về địa chỉ đăng nhập không được hỗ trợ"
    );
    Ok(())
}

async fn start_session(data_dir: &Path, program: &Path) -> Result<Session> {
    let dir = ProbeDir(data_dir.join(format!(".codex-login-{}", uuid::Uuid::new_v4())));
    std::fs::create_dir(&dir.0)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&dir.0, std::fs::Permissions::from_mode(0o700))?;
    }
    let mut child = tokio::process::Command::new(program)
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
        .kill_on_drop(true)
        .spawn()
        .context("Không chạy được Codex CLI để đăng nhập")?;
    let input = child.stdin.take().context("Không mở được stdin Codex")?;
    let output = BufReader::new(
        child
            .stdout
            .take()
            .context("Không mở được stdout Codex")?
            .take(1024 * 1024),
    );
    let mut session = Session {
        child,
        input,
        output,
        dir,
        login_id: String::new(),
        auth_url: String::new(),
    };
    let result = async {
        session
            .input
            .write_all(
                format!(
                    "{}\n",
                    json!({"id":0,"method":"initialize","params":{"clientInfo":{"name":"agent_account_manager","version":"1.0.3"}}})
                )
                .as_bytes(),
            )
            .await?;
        read_rpc(&mut session.output, 0).await?;
        session
            .input
            .write_all(b"{\"method\":\"initialized\"}\n{\"id\":1,\"method\":\"account/login/start\",\"params\":{\"type\":\"chatgpt\"}}\n")
            .await?;
        let result = read_rpc(&mut session.output, 1)
            .await
            .context("Không bắt đầu được đăng nhập Codex; kiểm tra có phiên đăng nhập khác đang dùng cổng 1455 hay không")?;
        let url = result["authUrl"]
            .as_str()
            .context("Codex không trả về link đăng nhập")?;
        validate_auth_url(url)?;
        session.auth_url = url.to_string();
        session.login_id = result["loginId"]
            .as_str()
            .filter(|id| !id.is_empty())
            .context("Codex không trả về mã phiên đăng nhập")?
            .to_string();
        Ok::<_, anyhow::Error>(())
    }
    .await;
    if let Err(e) = result {
        session.stop().await;
        return Err(e);
    }
    Ok(session)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_auth_url_accepts_valid_domains() {
        assert!(validate_auth_url("https://auth.openai.com/oauth/authorize?foo=bar").is_ok());
        assert!(validate_auth_url("https://chatgpt.com/auth/login").is_ok());
        assert!(validate_auth_url("https://subdomain.openai.com/login").is_ok());
    }

    #[test]
    fn test_validate_auth_url_rejects_insecure_and_untrusted() {
        assert!(validate_auth_url("http://auth.openai.com/test").is_err());
        assert!(validate_auth_url("https://evil.com/test").is_err());
        assert!(validate_auth_url("https://openai.com.attacker.com/test").is_err());
        assert!(validate_auth_url("not a url").is_err());
    }

    #[test]
    fn test_completed_message_check() {
        let ok_msg = json!({
            "method": "account/login/completed",
            "params": {
                "loginId": "test-id",
                "success": true
            }
        });
        assert_eq!(completed(&ok_msg, "test-id").unwrap(), true);
        assert_eq!(completed(&ok_msg, "other-id").unwrap(), false);

        let fail_msg = json!({
            "method": "account/login/completed",
            "params": {
                "loginId": "test-id",
                "success": false,
                "error": "user cancelled"
            }
        });
        assert!(completed(&fail_msg, "test-id").is_err());
    }
}
