use crate::client::{ApiClient, UnifiedAccountDto};
use crate::proxy::selection::Agent;
use anyhow::{bail, Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct Cli;

impl Cli {
    pub async fn handle_args(args: &[String]) -> Option<Result<()>> {
        if args.len() < 2 {
            return Some(Self::handle_list(None, &[]).await);
        }

        let first = args[1].to_lowercase();
        // Kiểm tra xem tham số đầu tiên có phải là tên agent hay không
        if let Ok(agent) = Self::parse_agent(&first) {
            return Some(Self::handle_agent_command(agent, &args[2..]).await);
        }

        match first.as_str() {
            "run" | "daemon" => None,
            "tui" => Some(crate::tui::run_tui().await),
            "check" | "overview" => Some(Self::handle_overview().await),
            "accounts" | "list" | "ls" => Some(Self::handle_list(None, &args[2..]).await),
            "switch" => Some(Self::handle_switch(None, &args[2..]).await),
            "refresh" => Some(Self::handle_refresh(None, &args[2..]).await),
            "preference" | "pref" => Some(Self::handle_preference(&args[2..]).await),
            "auto-select" | "select" => Some(Self::handle_auto_select(None, &args[2..]).await),
            "delete" | "rm" => Some(Self::handle_delete(None, &args[2..]).await),
            "settings" => Some(Self::handle_settings(None, &args[2..]).await),
            "add" => Some(Self::handle_add(None, &args[2..]).await),
            "import" => Some(Self::handle_import(None, &args[2..]).await),
            "login" => Some(Self::handle_login(None, &args[2..]).await),
            "reset" => Some(Self::handle_reset().await),
            "open" | "ui" | "web" | "dashboard" => Some(Self::open_dashboard()),
            "start" => Some(Self::start_service()),
            "stop" => Some(Self::stop_service()),
            "restart" => Some(Self::restart_service()),
            "status" => Some(Self::status_service()),
            "enable" | "autostart" => Some(Self::enable_autostart()),
            "disable" => Some(Self::disable_autostart()),
            "install" => Some(Self::install_binary()),
            "reinstall" => Some(Self::reinstall()),
            "uninstall" | "remove" => {
                if args.iter().skip(2).any(|arg| arg == "--help" || arg == "-h") {
                    println!("Cách sử dụng: aam uninstall [tùy chọn]");
                    println!();
                    println!("Tùy chọn:");
                    println!("  --purge, -p       Xóa toàn bộ thư mục dữ liệu cấu hình và tài khoản");
                    println!("  --keep-data       Giữ lại thư mục dữ liệu cấu hình mà không cần hỏi lại");
                    println!("  --help, -h        Xem hướng dẫn lệnh gỡ cài đặt");
                    Some(Ok(()))
                } else {
                    let purge = args.iter().skip(2).any(|arg| arg == "--purge" || arg == "-p");
                    let keep_data = args.iter().skip(2).any(|arg| arg == "--keep-data" || arg == "--no-purge");
                    Some(Self::uninstall(purge, keep_data))
                }
            }
            "update" | "upgrade" => Some(Self::update_binary()),
            "version" | "-v" | "--version" => {
                println!("aam v1.0.3 (Agent Account Manager)");
                Some(Ok(()))
            }
            "help" | "--help" | "-h" => {
                Self::print_help();
                Some(Ok(()))
            }
            unknown => {
                println!("Lệnh không hợp lệ: '{}'\n", unknown);
                Self::print_help();
                Some(Ok(()))
            }
        }
    }

    async fn handle_agent_command(agent: Agent, args: &[String]) -> Result<()> {
        if args.is_empty() {
            return Self::handle_list(Some(agent), &[]).await;
        }

        let subcmd = args[0].to_lowercase();
        let sub_args = &args[1..];
        match subcmd.as_str() {
            "accounts" | "list" | "ls" => Self::handle_list(Some(agent), sub_args).await,
            "switch" => Self::handle_switch(Some(agent), sub_args).await,
            "refresh" => Self::handle_refresh(Some(agent), sub_args).await,
            "auto-select" | "select" => Self::handle_auto_select(Some(agent), sub_args).await,
            "settings" => Self::handle_settings(Some(agent), sub_args).await,
            "delete" | "rm" => Self::handle_delete(Some(agent), sub_args).await,
            "add" => Self::handle_add(Some(agent), sub_args).await,
            "import" => Self::handle_import(Some(agent), sub_args).await,
            "login" => Self::handle_login(Some(agent), sub_args).await,
            "preference" | "pref" => {
                if agent == Agent::Antigravity {
                    Self::handle_preference(sub_args).await
                } else {
                    println!("Lưu ý: Cấu hình ưu tiên mô hình (preference) chỉ áp dụng cho Antigravity.");
                    println!("Dùng lệnh 'aam agy preference' để cấu hình.");
                    Ok(())
                }
            }
            "reset" => {
                if agent == Agent::Antigravity {
                    Self::handle_reset().await
                } else {
                    println!("Lưu ý: Đặt lại thời gian chờ (reset cooldowns) chỉ áp dụng cho Antigravity.");
                    Ok(())
                }
            }
            "help" | "-h" | "--help" => {
                Self::print_agent_help(agent);
                Ok(())
            }
            unknown => {
                println!("Lệnh không hợp lệ cho {}: '{}'\n", agent.name(), unknown);
                Self::print_agent_help(agent);
                Ok(())
            }
        }
    }

    fn print_agent_help(agent: Agent) {
        let name = agent.name();
        let alias = match agent {
            Agent::Antigravity => "agy",
            Agent::Codex => "codex",
            Agent::Claude => "claude",
        };
        println!("Quản lý tài khoản {} (aam {})", name, alias);
        println!();
        println!("Cách sử dụng:");
        println!("  aam {} <lệnh_con> [tùy chọn]", alias);
        println!();
        println!("Các lệnh con khả dụng:");
        println!("  list, ls              Xem danh sách tài khoản (--active, --json)");
        println!("  switch <# | ID>       Chuyển tài khoản đang hoạt động cho {}", name);
        println!("  refresh               Làm mới dữ liệu hạn ngạch ngay lập tức");
        println!("  auto-select           Tự động chọn tài khoản tối ưu nhất");
        println!("  settings              Xem hoặc cấu hình tự động chọn (--enable/--disable)");
        match agent {
            Agent::Antigravity => {
                println!("  login                 Đăng nhập tài khoản Google OAuth qua trình duyệt");
                println!("  add                   Thêm tài khoản thủ công (--email, --access-token...)");
                println!("  preference            Xem hoặc đổi cấu hình ưu tiên (auto | gemini | claude_gpt)");
                println!("  reset                 Đặt lại thời gian chờ (cooldowns)");
            }
            Agent::Codex => {
                println!("  login                 Đăng nhập thiết bị qua trình duyệt (Device OAuth)");
                println!("  import                Nhập tài khoản từ Codex CLI cục bộ (--email)");
                println!("  add                   Thêm tài khoản thủ công (--email, --token, --id-token)");
            }
            Agent::Claude => {
                println!("  import                Nhập tài khoản từ Claude Code CLI cục bộ (--email)");
                println!("  add                   Thêm tài khoản thủ công (--email, --token)");
            }
        }
        println!("  delete <# | ID>       Xóa tài khoản khỏi danh sách (-y để bỏ qua xác nhận)");
        println!("  help                  Xem hướng dẫn này");
        println!();
    }

    pub fn open_dashboard_url() -> Result<()> {
        Self::open_dashboard()
    }

    pub fn open_dashboard() -> Result<()> {
        let port = crate::config::Config::default().port;
        if !Self::is_relay_service_running(port) {
            anyhow::ensure!(
                !Self::is_port_in_use(port),
                "Cổng {} đang bị một dịch vụ không xác định chiếm dụng",
                port
            );
            println!("[aam] Dịch vụ chưa chạy, đang tự động khởi động nền...");
            Self::start_service()?;
            std::thread::sleep(std::time::Duration::from_millis(600));
        }
        anyhow::ensure!(
            Self::is_relay_service_running(port),
            "Không thể xác minh dịch vụ Agent Relay trên cổng {}",
            port
        );

        let bootstrap_token = Self::create_browser_bootstrap(port)?;
        let url = format!("http://127.0.0.1:{port}/#bootstrap={bootstrap_token}");
        println!("[aam] Đang mở bảng điều khiển tại http://127.0.0.1:{port}");

        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("xdg-open")
                .arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }

        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("open")
                .arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }

        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("cmd")
                .args(["/c", "start", "", &url])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }

        Ok(())
    }

    fn get_installed_bin_path() -> PathBuf {
        #[cfg(target_os = "windows")]
        {
            dirs::data_local_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("agent-relay")
                .join("bin")
                .join("agent-relay.exe")
        }
        #[cfg(not(target_os = "windows"))]
        {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local")
                .join("bin")
                .join("agent-relay")
        }
    }

    fn get_service_file_path() -> Option<PathBuf> {
        #[cfg(target_os = "linux")]
        {
            let home = dirs::home_dir()?;
            Some(
                home.join(".config")
                    .join("systemd")
                    .join("user")
                    .join("agent-relay.service"),
            )
        }
        #[cfg(not(target_os = "linux"))]
        {
            None
        }
    }

    fn is_port_in_use(port: u16) -> bool {
        std::net::TcpListener::bind(format!("127.0.0.1:{}", port)).is_err()
    }

    fn is_relay_service_running(port: u16) -> bool {
        let Ok(response) = Self::relay_http_request(port, "GET", "/api/health") else {
            return false;
        };
        let lower = response.to_ascii_lowercase();
        response.starts_with("HTTP/1.1 200")
            && response.contains("application/json")
            && (lower.contains("x-agent-relay: 1") || lower.contains("x-antigravity-relay: 1"))
    }

    fn create_browser_bootstrap(port: u16) -> Result<String> {
        let response = Self::relay_http_request(port, "POST", "/api/session/bootstrap")?;
        Self::parse_browser_bootstrap_response(&response)
    }

    fn parse_browser_bootstrap_response(response: &str) -> Result<String> {
        anyhow::ensure!(
            response.starts_with("HTTP/1.1 200"),
            "Daemon từ chối tạo phiên trình duyệt"
        );
        let body = response
            .split_once("\r\n\r\n")
            .map(|(_, body)| body)
            .context("Phản hồi tạo phiên trình duyệt không hợp lệ")?;
        let json: serde_json::Value = serde_json::from_str(body)?;
        let token = json["bootstrap_token"]
            .as_str()
            .context("Phản hồi thiếu bootstrap token")?;
        anyhow::ensure!(
            !token.is_empty()
                && token.len() <= 128
                && token
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'),
            "Bootstrap token không hợp lệ"
        );
        Ok(token.to_string())
    }

    fn relay_http_request(port: u16, method: &str, path: &str) -> Result<String> {
        let address = SocketAddr::from(([127, 0, 0, 1], port));
        let mut stream =
            TcpStream::connect_timeout(&address, std::time::Duration::from_millis(800))?;
        let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(500)));
        let _ = stream.set_write_timeout(Some(std::time::Duration::from_millis(500)));
        let key = crate::config::Config::default().master_key;
        anyhow::ensure!(
            key.bytes().all(|byte| byte.is_ascii_graphic()),
            "Master key chứa ký tự không hợp lệ"
        );
        anyhow::ensure!(
            matches!(method, "GET" | "POST")
                && path.starts_with('/')
                && !path.contains(['\r', '\n']),
            "Yêu cầu nội bộ không hợp lệ"
        );
        let request = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAuthorization: Bearer {key}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
        );
        stream.write_all(request.as_bytes())?;

        let mut response = Vec::new();
        stream.take(64 * 1024).read_to_end(&mut response)?;
        Ok(String::from_utf8(response).context("Phản hồi daemon không phải UTF-8")?)
    }

    fn is_systemd_service_active() -> bool {
        #[cfg(target_os = "linux")]
        {
            let is_active = |service: &str| {
                Command::new("systemctl")
                    .args(["--user", "is-active", service])
                    .stdin(Stdio::null())
                    .stderr(Stdio::null())
                    .output()
                    .map(|out| String::from_utf8_lossy(&out.stdout).trim() == "active")
                    .unwrap_or(false)
            };
            is_active("agent-relay.service") || is_active("antigravity-relay.service")
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    fn is_systemd_service_enabled() -> bool {
        #[cfg(target_os = "linux")]
        {
            let is_enabled = |service: &str| {
                Command::new("systemctl")
                    .args(["--user", "is-enabled", service])
                    .stdin(Stdio::null())
                    .stderr(Stdio::null())
                    .output()
                    .map(|out| String::from_utf8_lossy(&out.stdout).trim() == "enabled")
                    .unwrap_or(false)
            };
            is_enabled("agent-relay.service") || is_enabled("antigravity-relay.service")
        }
        #[cfg(not(target_os = "linux"))]
        {
            false
        }
    }

    pub fn start_service() -> Result<()> {
        let port = crate::config::Config::default().port;
        if Self::is_port_in_use(port) {
            anyhow::ensure!(
                Self::is_relay_service_running(port),
                "Cổng {} đang bị một dịch vụ không xác định chiếm dụng",
                port
            );
            println!("[aam] Dịch vụ Agent Relay đang chạy tại http://127.0.0.1:{port}");
            return Ok(());
        }

        let started_via_systemd = {
            #[cfg(target_os = "linux")]
            {
                if Self::is_systemd_service_enabled() {
                    println!("[aam] Đang khởi chạy dịch vụ qua systemd...");
                    let service = if Command::new("systemctl")
                        .args(["--user", "is-enabled", "agent-relay.service"])
                        .stdin(Stdio::null())
                        .stderr(Stdio::null())
                        .output()
                        .map(|out| String::from_utf8_lossy(&out.stdout).trim() == "enabled")
                        .unwrap_or(false)
                    {
                        "agent-relay.service"
                    } else {
                        "antigravity-relay.service"
                    };
                    let _ = Command::new("systemctl")
                        .args(["--user", "start", service])
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .status();
                    true
                } else {
                    false
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                false
            }
        };

        if !started_via_systemd {
            let exe = std::env::current_exe()?;
            println!("[aam] Đang khởi chạy tiến trình nền...");
            let mut cmd = Command::new(exe);
            cmd.arg("run")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null());
            #[cfg(unix)]
            {
                use std::os::unix::process::CommandExt;
                cmd.process_group(0);
            }
            cmd.spawn()
                .context("Không thể khởi chạy tiến trình nền")?;
        }

        std::thread::sleep(std::time::Duration::from_millis(600));

        if Self::is_relay_service_running(port) {
            println!("[aam] Dịch vụ đã khởi chạy thành công tại http://127.0.0.1:{port}");
        } else if Self::is_port_in_use(port) {
            anyhow::bail!(
                "Cổng {} đang phản hồi nhưng không phải Agent Relay",
                port
            );
        } else {
            println!(
                "[aam] Đã gửi lệnh khởi chạy. Vui lòng kiểm tra lại bằng lệnh 'aam status'."
            );
        }

        Ok(())
    }

    fn stop_service() -> Result<()> {
        println!("[aam] Đang dừng dịch vụ Agent Relay...");
        let port = crate::config::Config::default().port;
        let was_relay_running = Self::is_relay_service_running(port);

        // Stop systemd units if on Linux
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("systemctl")
                .args(["--user", "stop", "agent-relay.service"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let _ = Command::new("systemctl")
                .args(["--user", "stop", "antigravity-relay.service"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
        }

        // Stop only a previously authenticated relay process left outside systemd.
        if was_relay_running && Self::is_relay_service_running(port) {
            #[cfg(unix)]
            {
                let _ = Command::new("fuser")
                    .args(["-k", &format!("{port}/tcp")])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            #[cfg(target_os = "windows")]
            {
                let _ = Command::new("taskkill")
                    .args(["/F", "/IM", "agent-relay.exe"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
                let _ = Command::new("taskkill")
                    .args(["/F", "/IM", "antigravity-relay.exe"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(300));
        println!("[aam] Đã dừng dịch vụ Agent Relay.");
        Ok(())
    }

    fn restart_service() -> Result<()> {
        Self::stop_service()?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        Self::start_service()?;
        Ok(())
    }

    fn status_service() -> Result<()> {
        println!("=====================================================");
        println!("   Trạng thái dịch vụ Agent Relay (aam)");
        println!("=====================================================");

        let port = crate::config::Config::default().port;
        let is_running = Self::is_relay_service_running(port);
        let has_port_conflict = !is_running && Self::is_port_in_use(port);
        let is_sysd_active = Self::is_systemd_service_active();
        let is_sysd_enabled = Self::is_systemd_service_enabled();

        if is_running {
            println!("* Trạng thái:           Đang hoạt động (Running)");
            println!("* Địa chỉ máy chủ:      http://127.0.0.1:{port}");
            if is_sysd_active {
                println!("* Trình quản lý:        systemd (user service)");
            } else {
                println!("* Trình quản lý:        background daemon process");
            }
        } else {
            println!("* Trạng thái:           Đã dừng (Stopped)");
            if has_port_conflict {
                println!("* Cảnh báo:             Cổng {port} đang do dịch vụ khác sử dụng");
            }
        }

        println!(
            "* Tự khởi động (boot):  {}",
            if is_sysd_enabled {
                "Đã bật (Auto-start on boot)"
            } else {
                "Đang tắt"
            }
        );
        println!("=====================================================");

        if is_running {
            println!("Giao diện quản lý: http://127.0.0.1:{port}");
            println!("Dịch vụ đang tự động quét hạn ngạch và đồng bộ tài khoản tối ưu cho các agent.");
        } else {
            println!("Dùng 'aam' hoặc 'aam start' để chạy dịch vụ và mở giao diện.");
            println!("Dùng 'aam autostart' để tự động chạy liên tục kể cả khi restart máy.");
        }

        Ok(())
    }

    fn enable_autostart() -> Result<()> {
        #[cfg(not(target_os = "linux"))]
        {
            anyhow::bail!("Tính năng tự khởi động qua systemd hiện chỉ hỗ trợ trên Linux.");
        }
        #[cfg(target_os = "linux")]
        {
            let service_path = match Self::get_service_file_path() {
                Some(p) => p,
                None => return Err(anyhow::anyhow!("Không tìm thấy thư mục home")),
            };

            if let Some(parent) = service_path.parent() {
                fs::create_dir_all(parent)?;
            }

            // Ensure binary is installed in ~/.local/bin/agent-relay
            let bin_path = Self::get_installed_bin_path();
            if !bin_path.exists() {
                let current_exe = std::env::current_exe()?;
                let binary = fs::read(&current_exe)?;
                crate::storage::secure_file::atomic_write(&bin_path, &binary, 0o755)?;
            }

            let bin_arg = Self::quote_systemd_exec_path(&bin_path)?;

            let service_content = format!(
                "[Unit]\n\
                Description=Agent Relay Daemon (Account Manager for AI Coding Agents)\n\
                After=network.target\n\n\
                [Service]\n\
                Type=simple\n\
                ExecStart={} run\n\
                Restart=always\n\
                RestartSec=3\n\
                Environment=RUST_LOG=info\n\n\
                [Install]\n\
                WantedBy=default.target\n",
                bin_arg
            );

            crate::storage::secure_file::atomic_write(
                &service_path,
                service_content.as_bytes(),
                0o600,
            )?;
            println!("[aam] Đã tạo service file tại {:?}", service_path);

            // Stop/disable legacy service if running
            let _ = Command::new("systemctl")
                .args(["--user", "disable", "--now", "antigravity-relay.service"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

            // Reload systemd & enable
            let _ = Command::new("systemctl")
                .args(["--user", "daemon-reload"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

            let status = Command::new("systemctl")
                .args(["--user", "enable", "--now", "agent-relay.service"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;

            if status.success() {
                println!("[aam] Đã kích hoạt chế độ tự động chạy cùng hệ thống (Auto-start on boot).");
                println!("       - Tự động chạy nền liên tục kể cả khi khởi động lại máy tính.");
                println!("       - Tự động hồi phục và bật lại sau 3 giây nếu bị dừng.");
                println!("       - Dùng lệnh 'aam stop' hoặc 'aam disable' khi muốn dừng.");
            } else {
                println!("[aam] Có lỗi khi kích hoạt systemd service.");
            }

            Ok(())
        }
    }

    fn quote_systemd_exec_path(path: &PathBuf) -> Result<String> {
        let value = path
            .to_str()
            .context("Đường dẫn binary không phải UTF-8 hợp lệ")?;
        anyhow::ensure!(
            !value.chars().any(char::is_control),
            "Đường dẫn binary chứa ký tự điều khiển không an toàn"
        );
        let escaped = value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('%', "%%")
            .replace('$', "$$");
        Ok(format!("\"{}\"", escaped))
    }

    fn disable_autostart() -> Result<()> {
        #[cfg(not(target_os = "linux"))]
        {
            anyhow::bail!("Tính năng tự khởi động qua systemd hiện chỉ hỗ trợ trên Linux.");
        }
        #[cfg(target_os = "linux")]
        {
            println!("[aam] Đang tắt chế độ tự động chạy cùng hệ thống...");
            let _ = Command::new("systemctl")
                .args(["--user", "disable", "--now", "agent-relay.service"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();
            let _ = Command::new("systemctl")
                .args(["--user", "disable", "--now", "antigravity-relay.service"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

            if let Some(service_path) = Self::get_service_file_path() {
                if service_path.exists() {
                    let _ = fs::remove_file(&service_path);
                }
                let legacy_path = service_path
                    .parent()
                    .map(|p| p.join("antigravity-relay.service"));
                if let Some(lp) = legacy_path {
                    if lp.exists() {
                        let _ = fs::remove_file(lp);
                    }
                }
            }

            let _ = Command::new("systemctl")
                .args(["--user", "daemon-reload"])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status();

            println!("[aam] Đã tắt chế độ tự khởi động cùng hệ thống.");
            Ok(())
        }
    }

    fn install_binary() -> Result<()> {
        let current_exe = std::env::current_exe()?;
        let target_bin = Self::get_installed_bin_path();

        if let Some(parent) = target_bin.parent() {
            fs::create_dir_all(parent)?;
        }

        if current_exe != target_bin {
            let binary = fs::read(&current_exe)?;
            crate::storage::secure_file::atomic_write(&target_bin, &binary, 0o755)?;
        }

        // Create short symlink 'aam' on Unix and clean up legacy 'agyr' symlink if present
        #[cfg(unix)]
        if let Some(parent) = target_bin.parent() {
            let symlink_path = parent.join("aam");
            let _ = fs::remove_file(&symlink_path);
            let legacy_symlink = parent.join("agyr");
            let _ = fs::remove_file(&legacy_symlink);
            use std::os::unix::fs::symlink;
            let _ = symlink(&target_bin, &symlink_path);
        }

        #[cfg(target_os = "windows")]
        if let Some(parent) = target_bin.parent() {
            let aam_exe = parent.join("aam.exe");
            if aam_exe != current_exe {
                let _ = fs::copy(&target_bin, &aam_exe);
            }
        }

        println!(
            "[aam] Đã cài đặt lệnh 'aam' và 'agent-relay' vào {:?}",
            target_bin
        );
        println!("       Bạn có thể dùng lệnh 'aam' ở bất kỳ đâu trong terminal.");
        Ok(())
    }

    fn reinstall() -> Result<()> {
        println!("=====================================================");
        println!("   Cài đặt lại Agent Relay Manager (aam)");
        println!("=====================================================");
        let port = crate::config::Config::default().port;
        let was_running = Self::is_relay_service_running(port);
        let was_sysd_enabled = Self::is_systemd_service_enabled();

        if was_running {
            let _ = Self::stop_service();
        }

        Self::install_binary()?;

        if was_sysd_enabled {
            println!("[aam] Đang cập nhật lại dịch vụ tự khởi động...");
            Self::enable_autostart()?;
        } else if was_running {
            println!("[aam] Đang khởi động lại dịch vụ...");
            Self::start_service()?;
        }

        println!("Cài đặt lại hoàn tất!");
        Ok(())
    }

    fn uninstall(mut purge: bool, keep_data: bool) -> Result<()> {
        use std::io::IsTerminal;

        println!("=====================================================");
        println!("   Gỡ cài đặt Agent Relay Manager (aam)");
        println!("=====================================================");

        let config = crate::config::Config::default();
        let data_dir = config.data_dir;
        let legacy_dir = dirs::home_dir().map(|h| h.join(".antigravity-relay"));

        if !purge && !keep_data && std::io::stdin().is_terminal() {
            print!(
                "Bạn có muốn xóa toàn bộ dữ liệu cấu hình và tài khoản (tại {}) không? [y/N]: ",
                data_dir.display()
            );
            let _ = std::io::stdout().flush();
            let mut input = String::new();
            if std::io::stdin().read_line(&mut input).is_ok() {
                let trimmed = input.trim().to_lowercase();
                if trimmed == "y" || trimmed == "yes" {
                    purge = true;
                }
            }
        }

        // 1. Dừng dịch vụ nếu đang hoạt động
        println!("[aam] Đang dừng dịch vụ nếu đang hoạt động...");
        let _ = Self::stop_service();

        // 2. Tắt chế độ tự khởi động cùng hệ thống
        #[cfg(target_os = "linux")]
        {
            println!("[aam] Đang tắt và xóa dịch vụ khởi động systemd...");
            let _ = Self::disable_autostart();
        }

        // 3. Xóa các tệp thực thi và liên kết
        let target_bin = Self::get_installed_bin_path();
        if target_bin.exists() {
            if let Err(e) = fs::remove_file(&target_bin) {
                eprintln!("[cảnh báo] Không thể xóa {}: {}", target_bin.display(), e);
            } else {
                println!("[aam] Đã xóa tệp thực thi: {}", target_bin.display());
            }
        }

        if let Some(parent) = target_bin.parent() {
            #[cfg(unix)]
            {
                let symlink_path = parent.join("aam");
                if symlink_path.exists() || fs::symlink_metadata(&symlink_path).is_ok() {
                    let _ = fs::remove_file(&symlink_path);
                    println!("[aam] Đã xóa liên kết: {}", symlink_path.display());
                }
                let legacy_symlink = parent.join("agyr");
                if legacy_symlink.exists() || fs::symlink_metadata(&legacy_symlink).is_ok() {
                    let _ = fs::remove_file(&legacy_symlink);
                }
                let legacy_bin = parent.join("antigravity-relay");
                if legacy_bin.exists() {
                    let _ = fs::remove_file(&legacy_bin);
                }
            }
            #[cfg(target_os = "windows")]
            {
                let aam_exe = parent.join("aam.exe");
                if aam_exe.exists() {
                    let _ = fs::remove_file(&aam_exe);
                    println!("[aam] Đã xóa tệp thực thi: {}", aam_exe.display());
                }
                let legacy_exe = parent.join("antigravity-relay.exe");
                if legacy_exe.exists() {
                    let _ = fs::remove_file(&legacy_exe);
                }
            }
        }

        // 4. Xóa dữ liệu cấu hình nếu được yêu cầu
        if purge {
            if data_dir.exists() {
                if let Err(e) = fs::remove_dir_all(&data_dir) {
                    eprintln!(
                        "[cảnh báo] Không thể xóa thư mục dữ liệu {}: {}",
                        data_dir.display(),
                        e
                    );
                } else {
                    println!(
                        "[aam] Đã xóa toàn bộ thư mục dữ liệu cấu hình tại: {}",
                        data_dir.display()
                    );
                }
            }
            if let Some(ref leg) = legacy_dir {
                if leg.exists() {
                    let _ = fs::remove_dir_all(leg);
                }
            }
        } else if data_dir.exists() {
            println!(
                "[aam] Đã giữ lại thư mục cấu hình và dữ liệu tài khoản tại: {}",
                data_dir.display()
            );
            println!(
                "       (Để xóa sạch hoàn toàn, bạn có thể xóa thủ công thư mục trên hoặc dùng lệnh 'aam uninstall --purge')"
            );
        }

        println!("=====================================================");
        println!("Gỡ cài đặt hoàn tất!");
        Ok(())
    }

    fn update_binary() -> Result<()> {
        println!("[aam] Đang kiểm tra và tải bản cập nhật mới nhất từ GitHub...");

        let port = crate::config::Config::default().port;
        let was_running = Self::is_relay_service_running(port);

        Self::download_verified_release()?;
        println!("[aam] Cập nhật phiên bản mới nhất thành công!");
        if was_running {
            println!("[aam] Đang khởi động lại dịch vụ với phiên bản mới...");
            Self::restart_service()?;
        }

        Ok(())
    }

    fn download_verified_release() -> Result<()> {
        let is_windows = cfg!(target_os = "windows");
        let os = match std::env::consts::OS {
            "macos" => "darwin",
            value => value,
        };
        let arch = match std::env::consts::ARCH {
            "x86_64" => "x86_64",
            "aarch64" => "aarch64",
            value => value,
        };
        let asset_name = if is_windows {
            format!("agent-relay-{}-{}.zip", os, arch)
        } else {
            format!("agent-relay-{}-{}.tar.gz", os, arch)
        };
        let asset_url = format!(
            "https://github.com/SonNX24042005/agents-account-manager/releases/latest/download/{}",
            asset_name
        );
        let checksum_url = format!("{}.sha256", asset_url);
        let temp_dir = std::env::temp_dir().join(format!("aam-update-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&temp_dir).context("Không thể tạo thư mục cập nhật tạm")?;

        let result = (|| -> Result<()> {
            let archive = temp_dir.join(&asset_name);
            let checksum_file = temp_dir.join(format!("{}.sha256", asset_name));
            if Self::download_file(&asset_url, &archive).is_err() {
                // Fallback to previous repository release URL during transition
                let fallback_url = format!(
                    "https://github.com/SonNX24042005/agents-account-manager/releases/latest/download/{}",
                    asset_name
                );
                Self::download_file(&fallback_url, &archive)?;
                Self::download_file(&format!("{}.sha256", fallback_url), &checksum_file)?;
            } else {
                Self::download_file(&checksum_url, &checksum_file)?;
            }

            let checksum_text = fs::read_to_string(&checksum_file)
                .context("Không thể đọc checksum của bản phát hành")?;
            let expected = Self::parse_sha256(&checksum_text)?;
            let actual = Self::sha256_file(&archive)?;
            anyhow::ensure!(
                actual.eq_ignore_ascii_case(&expected),
                "Checksum SHA-256 của bản cập nhật không khớp"
            );

            let extracted_bin_name = if is_windows {
                "agent-relay.exe"
            } else {
                "agent-relay"
            };

            if is_windows {
                let status = Command::new("tar")
                    .args(["-xf"])
                    .arg(&archive)
                    .arg("-C")
                    .arg(&temp_dir)
                    .status();
                let success = match status {
                    Ok(s) => s.success(),
                    Err(_) => {
                        let ps_cmd = format!(
                            "Expand-Archive -Path '{}' -DestinationPath '{}' -Force",
                            archive.display(),
                            temp_dir.display()
                        );
                        Command::new("powershell")
                            .args(["-NoProfile", "-Command", &ps_cmd])
                            .status()
                            .map(|s| s.success())
                            .unwrap_or(false)
                    }
                };
                anyhow::ensure!(success, "Không thể giải nén bản cập nhật zip");
            } else {
                let list_output = Command::new("tar")
                    .args(["-tzf"])
                    .arg(&archive)
                    .output()
                    .context("Không thể kiểm tra nội dung gói cập nhật")?;
                anyhow::ensure!(
                    list_output.status.success(),
                    "Gói cập nhật không phải tar.gz hợp lệ"
                );
                let listing = String::from_utf8_lossy(&list_output.stdout);
                let entries: Vec<&str> = listing
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .collect();
                anyhow::ensure!(
                    entries.len() == 1
                        && matches!(
                            entries[0],
                            "agent-relay" | "./agent-relay" | "antigravity-relay" | "./antigravity-relay"
                        ),
                    "Gói cập nhật chứa đường dẫn không mong đợi"
                );

                let status = Command::new("tar")
                    .args(["-xzf"])
                    .arg(&archive)
                    .arg("-C")
                    .arg(&temp_dir)
                    .status()
                    .context("Không thể giải nén bản cập nhật")?;
                anyhow::ensure!(status.success(), "Không thể giải nén bản cập nhật");
            }

            let extracted = temp_dir.join(extracted_bin_name);
            let metadata = fs::symlink_metadata(&extracted)
                .with_context(|| format!("Gói cập nhật thiếu file {}", extracted_bin_name))?;
            anyhow::ensure!(
                metadata.file_type().is_file(),
                "Binary cập nhật không phải file thường"
            );
            let binary = fs::read(&extracted).context("Không thể đọc binary cập nhật")?;
            crate::storage::secure_file::atomic_write(
                &Self::get_installed_bin_path(),
                &binary,
                0o755,
            )?;
            Ok(())
        })();

        let _ = fs::remove_dir_all(&temp_dir);
        result
    }

    fn download_file(url: &str, destination: &PathBuf) -> Result<()> {
        let status = Command::new("curl")
            .args([
                "--proto",
                "=https",
                "--tlsv1.2",
                "-fsSL",
                "--retry",
                "3",
                "--max-time",
                "120",
            ])
            .arg(url)
            .arg("-o")
            .arg(destination)
            .status();

        let ok = match status {
            Ok(s) => s.success(),
            Err(_) => false,
        };

        if !ok {
            #[cfg(target_os = "windows")]
            {
                let ps_cmd = format!(
                    "Invoke-WebRequest -Uri '{}' -OutFile '{}'",
                    url,
                    destination.display()
                );
                let fallback = Command::new("powershell")
                    .args(["-NoProfile", "-Command", &ps_cmd])
                    .status()
                    .map(|s| s.success())
                    .unwrap_or(false);
                anyhow::ensure!(fallback, "Không thể tải {}", url);
                return Ok(());
            }
            #[cfg(not(target_os = "windows"))]
            {
                anyhow::bail!("Tải bản cập nhật thất bại: {}", url);
            }
        }
        Ok(())
    }

    fn parse_sha256(value: &str) -> Result<String> {
        let checksum = value
            .split_whitespace()
            .next()
            .ok_or_else(|| anyhow::anyhow!("File checksum trống"))?;
        anyhow::ensure!(
            checksum.len() == 64 && checksum.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "Checksum SHA-256 không hợp lệ"
        );
        Ok(checksum.to_ascii_lowercase())
    }

    fn sha256_file(path: &PathBuf) -> Result<String> {
        let mut file = fs::File::open(path).context("Không thể mở gói cập nhật")?;
        let mut hasher = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(format!("{:x}", hasher.finalize()))
    }

    async fn handle_overview() -> Result<()> {
        let port = crate::config::Config::default().port;
        let is_running = Self::is_relay_service_running(port);

        println!("========================================================================");
        println!("   Tổng quan dịch vụ và tài khoản (Agent Account Manager)");
        println!("========================================================================");

        if is_running {
            println!("• Trạng thái dịch vụ:   Đang hoạt động (http://127.0.0.1:{port})");
        } else {
            println!("• Trạng thái dịch vụ:   Đã dừng (gõ 'aam start' để khởi chạy)");
        }

        let is_sysd_enabled = Self::is_systemd_service_enabled();
        println!(
            "• Tự khởi động (boot):  {}",
            if is_sysd_enabled {
                "Đã bật (systemd)"
            } else {
                "Đang tắt"
            }
        );
        println!();

        if !is_running {
            println!("Gợi ý: Dùng 'aam start' để khởi chạy dịch vụ hoặc 'aam tui' để vào giao diện.");
            return Ok(());
        }

        let client = ApiClient::new();
        let (anti_accounts, codex_accounts, claude_accounts, flags) = tokio::join!(
            client.list_accounts_for_agent(Agent::Antigravity),
            client.list_accounts_for_agent(Agent::Codex),
            client.list_accounts_for_agent(Agent::Claude),
            client.get_agent_settings(),
        );

        let anti_accs = anti_accounts.unwrap_or_default();
        let codex_accs = codex_accounts.unwrap_or_default();
        let claude_accs = claude_accounts.unwrap_or_default();
        let flags = flags.unwrap_or(crate::client::SelectionFlagsDto {
            antigravity: true,
            codex: false,
            claude: false,
        });

        println!("{:<13} {:<7} {:<36} {:<16} {:<12}", "Agent", "Số TK", "Tài khoản đang dùng", "Hạn ngạch", "Tự động chọn");
        println!("{:-<13} {:-<7} {:-<36} {:-<16} {:-<12}", "", "", "", "", "");

        // Antigravity
        {
            let count = anti_accs.len().to_string();
            let active = anti_accs.iter().find(|a| a.is_active);
            let active_str = if let Some(a) = active {
                Self::truncate_str(&a.email, 35)
            } else if anti_accs.is_empty() {
                "(Chưa có tài khoản)".to_string()
            } else {
                "(Chưa chọn)".to_string()
            };
            let quota_str = if let Some(a) = active {
                if let Some(q) = a.quota_percentage {
                    if q < 100.0 {
                        if let Some(cd) = a.primary_reset_countdown() {
                            format!("{:.0}% ({})", q, cd)
                        } else {
                            format!("{:.0}%", q)
                        }
                    } else {
                        format!("{:.0}%", q)
                    }
                } else {
                    "--".to_string()
                }
            } else {
                "--".to_string()
            };
            let auto_str = if flags.antigravity { "Bật" } else { "Tắt" };
            println!("{:<13} {:<7} {:<36} {:<16} {:<12}", "Antigravity", count, active_str, quota_str, auto_str);
        }

        // Codex
        {
            let count = codex_accs.len().to_string();
            let active = codex_accs.iter().find(|a| a.is_active);
            let active_str = if let Some(a) = active {
                Self::truncate_str(&a.email, 35)
            } else if codex_accs.is_empty() {
                "(Chưa có tài khoản)".to_string()
            } else {
                "(Chưa chọn)".to_string()
            };
            let quota_str = if let Some(a) = active {
                if let Some(q) = a.quota_percentage {
                    if q < 100.0 {
                        if let Some(cd) = a.primary_reset_countdown() {
                            format!("{:.0}% ({})", q, cd)
                        } else {
                            format!("{:.0}%", q)
                        }
                    } else {
                        format!("{:.0}%", q)
                    }
                } else {
                    "--".to_string()
                }
            } else {
                "--".to_string()
            };
            let auto_str = if flags.codex { "Bật" } else { "Tắt" };
            println!("{:<13} {:<7} {:<36} {:<16} {:<12}", "Codex", count, active_str, quota_str, auto_str);
        }

        // Claude
        {
            let count = claude_accs.len().to_string();
            let active = claude_accs.iter().find(|a| a.is_active);
            let active_str = if let Some(a) = active {
                Self::truncate_str(&a.email, 35)
            } else if claude_accs.is_empty() {
                "(Chưa có tài khoản)".to_string()
            } else {
                "(Chưa chọn)".to_string()
            };
            let quota_str = if let Some(a) = active {
                if let Some(q) = a.quota_percentage {
                    if q < 100.0 {
                        if let Some(cd) = a.primary_reset_countdown() {
                            format!("{:.0}% ({})", q, cd)
                        } else {
                            format!("{:.0}%", q)
                        }
                    } else {
                        format!("{:.0}%", q)
                    }
                } else {
                    "--".to_string()
                }
            } else {
                "--".to_string()
            };
            let auto_str = if flags.claude { "Bật" } else { "Tắt" };
            println!("{:<13} {:<7} {:<36} {:<16} {:<12}", "Claude", count, active_str, quota_str, auto_str);
        }

        println!();
        println!("Quản lý theo từng agent:");
        println!("  • aam agy <lệnh>      Quản lý Antigravity (list, switch, login, add, preference...)");
        println!("  • aam codex <lệnh>    Quản lý Codex (list, switch, login, import, add...)");
        println!("  • aam claude <lệnh>   Quản lý Claude (list, switch, import, add...)");
        println!();
        println!("Lệnh chung:");
        println!("  • aam                 Xem bảng danh sách tài khoản toàn bộ agent (hoặc 'aam list')");
        println!("  • aam tui             Mở giao diện terminal tương tác toàn màn hình");
        println!("  • aam web             Mở bảng điều khiển web trên trình duyệt");
        println!("  • aam help            Xem danh sách đầy đủ các lệnh");
        Ok(())
    }

    async fn handle_list(scoped_agent: Option<Agent>, args: &[String]) -> Result<()> {
        let mut agent_filter = scoped_agent;
        let mut json_output = false;
        let mut active_only = false;

        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-a" | "--agent" => {
                    if let Some(val) = iter.next() {
                        agent_filter = Some(Self::parse_agent(val)?);
                    } else {
                        bail!("Thiếu tên agent sau tùy chọn -a / --agent");
                    }
                }
                "--json" => json_output = true,
                "--active" => active_only = true,
                "-h" | "--help" => {
                    if let Some(agent) = scoped_agent {
                        let alias = match agent {
                            Agent::Antigravity => "agy",
                            Agent::Codex => "codex",
                            Agent::Claude => "claude",
                        };
                        println!("Cách sử dụng: aam {} list [tùy chọn]", alias);
                    } else {
                        println!("Cách sử dụng: aam list [tùy chọn]");
                    }
                    println!();
                    println!("Tùy chọn:");
                    if scoped_agent.is_none() {
                        println!("  -a, --agent <agent>   Lọc theo agent (antigravity, codex, claude)");
                    }
                    println!("  --active              Chỉ hiển thị tài khoản đang hoạt động");
                    println!("  --json                Xuất dữ liệu định dạng json");
                    println!("  -h, --help            Xem hướng dẫn lệnh list");
                    return Ok(());
                }
                unknown => bail!("Tùy chọn không hợp lệ: '{}'", unknown),
            }
        }

        let client = ApiClient::new();
        if let Some(agent) = agent_filter {
            let mut accounts = client.list_accounts_for_agent(agent).await?;
            if active_only {
                accounts.retain(|a| a.is_active);
            }
            if json_output {
                println!("{}", serde_json::to_string_pretty(&accounts)?);
                return Ok(());
            }
            Self::print_agent_account_table(agent, &accounts);
            println!();
            let active_count = accounts.iter().filter(|a| a.is_active).count();
            let alias = match agent {
                Agent::Antigravity => "agy",
                Agent::Codex => "codex",
                Agent::Claude => "claude",
            };
            println!("Tổng cộng: {} tài khoản ({} đang hoạt động).", accounts.len(), active_count);
            println!("Gợi ý: Dùng 'aam {} switch <# hoặc email>' để chuyển tài khoản.", alias);
        } else {
            let mut all_accounts = client.list_all_accounts().await?;
            if active_only {
                all_accounts.retain(|a| a.is_active);
            }
            if json_output {
                println!("{}", serde_json::to_string_pretty(&all_accounts)?);
                return Ok(());
            }

            let agents = [Agent::Antigravity, Agent::Codex, Agent::Claude];
            for (i, agent) in agents.iter().enumerate() {
                if i > 0 {
                    println!();
                }
                let accs: Vec<UnifiedAccountDto> = all_accounts
                    .iter()
                    .filter(|a| a.agent == *agent)
                    .cloned()
                    .collect();
                Self::print_agent_account_table(*agent, &accs);
            }

            let total_active = all_accounts.iter().filter(|a| a.is_active).count();
            println!();
            println!("Tổng cộng: {} tài khoản trên toàn bộ agent ({} đang hoạt động).", all_accounts.len(), total_active);
            println!("Gợi ý:");
            println!("  • Chuyển tài khoản theo số thứ tự của agent: 'aam agy switch <#>' hoặc 'aam codex switch <#>'");
            println!("  • Chuyển tài khoản theo số thứ tự hoặc email: 'aam switch <# | email>'");
            println!("  • Xem và thao tác trực quan: 'aam tui'");
        }

        Ok(())
    }

    fn print_agent_account_table(agent: Agent, accounts: &[UnifiedAccountDto]) {
        let active_count = accounts.iter().filter(|a| a.is_active).count();

        if accounts.is_empty() {
            println!("=== {} (0 tài khoản) ===", agent.name());
            match agent {
                Agent::Antigravity => println!("(Chưa có tài khoản nào · Dùng 'aam agy login' hoặc 'aam agy add' để thêm)"),
                Agent::Codex => println!("(Chưa có tài khoản nào · Dùng 'aam codex login' hoặc 'aam codex import' để thêm)"),
                Agent::Claude => println!("(Chưa có tài khoản nào · Dùng 'aam claude import' để thêm)"),
            }
            return;
        }

        let detail_width = if agent == Agent::Antigravity { 62 } else { 46 };

        println!("=== {} ({} tài khoản · {} đang hoạt động) ===", agent.name(), accounts.len(), active_count);
        println!("{:<4} {:<12} {:<36} {}", "#", "Trạng thái", "Email", "Chi tiết");
        println!("{:-<4} {:-<12} {:-<36} {:-<width$}", "", "", "", "", width = detail_width);

        for (idx, acc) in accounts.iter().enumerate() {
            let num = format!("[{:>2}]", idx + 1);
            let status_str = if acc.is_active {
                "* Đang dùng"
            } else if acc.error.is_some() {
                "! Lỗi"
            } else if acc.status == "Hết hạn ngạch" || acc.quota_percentage == Some(0.0) {
                "x Hết quota"
            } else if acc.status == "Chờ cập nhật" {
                "~ Chờ"
            } else {
                "- Sẵn sàng"
            };

            let detail_str = if acc.quota_groups.len() > 1 {
                let mut group_parts = Vec::new();
                for group in &acc.quota_groups {
                    let grp_rank = if group.name.to_lowercase().contains("gemini") {
                        0
                    } else if group.name.to_lowercase().contains("claude") {
                        1
                    } else {
                        2
                    };
                    let grp_label = if group.name.to_lowercase().contains("gemini") {
                        "Gem"
                    } else if group.name.to_lowercase().contains("claude") {
                        "Claude"
                    } else {
                        &group.name
                    };

                    let mut sorted_buckets = group.buckets.clone();
                    sorted_buckets.sort_by_key(|b| {
                        if b.is_5h() {
                            0
                        } else if b.is_weekly() {
                            1
                        } else {
                            2
                        }
                    });

                    let bucket_texts: Vec<String> = sorted_buckets
                        .iter()
                        .map(|b| {
                            let pct = b.effective_percentage();
                            let reset_suffix = if pct < 100.0 {
                                b.reset_countdown()
                                    .map(|cd| format!(" ({})", cd))
                                    .unwrap_or_default()
                            } else {
                                String::new()
                            };
                            format!("{:.0}%{}", pct, reset_suffix)
                        })
                        .collect();

                    if !bucket_texts.is_empty() {
                        group_parts.push((grp_rank, format!("{}: {}", grp_label, bucket_texts.join(" · "))));
                    }
                }

                group_parts.sort_by_key(|(rank, _)| *rank);
                let group_texts: Vec<String> = group_parts.into_iter().map(|(_, text)| text).collect();
                if group_texts.is_empty() {
                    acc.status.clone()
                } else {
                    group_texts.join(" | ")
                }
            } else {
                let mut detail_parts = Vec::new();
                for group in &acc.quota_groups {
                    for bucket in &group.buckets {
                        let pct = bucket.effective_percentage();
                        let win_rank = if bucket.is_5h() {
                            0
                        } else if bucket.is_weekly() {
                            1
                        } else {
                            2
                        };

                        let win = if bucket.is_5h() {
                            "5h"
                        } else if bucket.is_weekly() {
                            "Tuần"
                        } else {
                            &bucket.window
                        };

                        let reset_suffix = if pct < 100.0 {
                            bucket
                                .reset_countdown()
                                .map(|cd| format!(" ({})", cd))
                                .unwrap_or_default()
                        } else {
                            String::new()
                        };
                        detail_parts.push((win_rank, format!("{}: {:.0}%{}", win, pct, reset_suffix)));
                    }
                }

                detail_parts.sort_by_key(|(rank, _)| *rank);
                let detail_texts: Vec<String> = detail_parts.into_iter().map(|(_, text)| text).collect();
                if detail_texts.is_empty() {
                    acc.status.clone()
                } else {
                    detail_texts.join(" · ")
                }
            };

            let truncated_detail = Self::truncate_str(&detail_str, detail_width - 1);
            let email_display = Self::truncate_str(&acc.email, 35);

            println!("{:<4} {:<12} {:<36} {}", num, status_str, email_display, truncated_detail);
        }
    }

    async fn handle_switch(scoped_agent: Option<Agent>, args: &[String]) -> Result<()> {
        let mut target = None;
        let mut agent_filter = scoped_agent;
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-a" | "--agent" => {
                    if let Some(val) = iter.next() {
                        agent_filter = Some(Self::parse_agent(val)?);
                    } else {
                        bail!("Thiếu tên agent sau tùy chọn -a / --agent");
                    }
                }
                "-h" | "--help" => {
                    let prefix = match scoped_agent {
                        Some(Agent::Antigravity) => "aam agy",
                        Some(Agent::Codex) => "aam codex",
                        Some(Agent::Claude) => "aam claude",
                        None => "aam",
                    };
                    println!("Cách sử dụng: {} switch <# | ID | email> [tùy chọn]", prefix);
                    println!();
                    println!("Tùy chọn:");
                    if scoped_agent.is_none() {
                        println!("  -a, --agent <agent>   Chỉ định agent (antigravity, codex, claude)");
                    }
                    println!("  -h, --help            Xem hướng dẫn lệnh switch");
                    return Ok(());
                }
                val if !val.starts_with('-') && target.is_none() => {
                    target = Some(val.to_string());
                }
                unknown => bail!("Tùy chọn không hợp lệ: '{}'", unknown),
            }
        }

        let Some(target) = target else {
            let prefix = match scoped_agent {
                Some(Agent::Antigravity) => "aam agy",
                Some(Agent::Codex) => "aam codex",
                Some(Agent::Claude) => "aam claude",
                None => "aam",
            };
            println!("Cách sử dụng: {} switch <# | ID | email>", prefix);
            println!();
            println!("Ví dụ:");
            println!("  {} switch 1", prefix);
            println!("  {} switch user@example.com", prefix);
            bail!("Thiếu định danh tài khoản cần chuyển");
        };

        let client = ApiClient::new();
        let accounts = if let Some(agent) = agent_filter {
            client.list_accounts_for_agent(agent).await?
        } else {
            client.list_all_accounts().await?
        };

        let matched = Self::find_account(&accounts, &target)?;
        client.switch_account(matched.agent, &matched.id).await?;
        println!("✓ Đã chuyển sang tài khoản {} ({}) thành công.", matched.email, matched.agent.name());
        Ok(())
    }

    async fn handle_refresh(scoped_agent: Option<Agent>, args: &[String]) -> Result<()> {
        let mut agent_filter = scoped_agent;
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-a" | "--agent" => {
                    if let Some(val) = iter.next() {
                        agent_filter = Some(Self::parse_agent(val)?);
                    } else {
                        bail!("Thiếu tên agent sau tùy chọn -a / --agent");
                    }
                }
                "-h" | "--help" => {
                    let prefix = match scoped_agent {
                        Some(Agent::Antigravity) => "aam agy",
                        Some(Agent::Codex) => "aam codex",
                        Some(Agent::Claude) => "aam claude",
                        None => "aam",
                    };
                    println!("Cách sử dụng: {} refresh [tùy chọn]", prefix);
                    println!();
                    println!("Tùy chọn:");
                    if scoped_agent.is_none() {
                        println!("  -a, --agent <agent>   Chỉ định agent cần làm mới (antigravity, codex, claude)");
                    }
                    return Ok(());
                }
                unknown => bail!("Tùy chọn không hợp lệ: '{}'", unknown),
            }
        }

        let client = ApiClient::new();
        client.refresh_quota(agent_filter).await?;
        if let Some(agent) = agent_filter {
            println!("✓ Đã gửi yêu cầu làm mới hạn ngạch cho {}.", agent.name());
        } else {
            println!("✓ Đã gửi yêu cầu làm mới hạn ngạch cho toàn bộ agent.");
        }
        Ok(())
    }

    async fn handle_preference(args: &[String]) -> Result<()> {
        let mut new_pref = None;
        let mut iter = args.iter();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    println!("Cách sử dụng: aam agy preference [auto | gemini | claude_gpt]");
                    println!();
                    println!("Tùy chọn giá trị:");
                    println!("  auto        Tự động chọn tài khoản theo mô hình vừa dùng gần nhất");
                    println!("  gemini      Luôn ưu tiên tài khoản có hạn ngạch Gemini cao nhất");
                    println!("  claude_gpt  Luôn ưu tiên tài khoản có hạn ngạch Claude & GPT cao nhất");
                    return Ok(());
                }
                val if !val.starts_with('-') && new_pref.is_none() => {
                    new_pref = Some(val);
                }
                unknown => bail!("Tùy chọn không hợp lệ: '{}'", unknown),
            }
        }

        let client = ApiClient::new();
        if let Some(pref) = new_pref {
            let updated = client.set_preference(pref).await?;
            let name = match updated.preference.as_str() {
                "auto" => "Tự động (theo mô hình vừa dùng)",
                "gemini" => "Luôn ưu tiên Gemini",
                "claude_gpt" => "Luôn ưu tiên Claude & GPT",
                other => other,
            };
            println!("✓ Đã cập nhật ưu tiên định tuyến thành: {}", name);
        } else {
            let state = client.get_preference().await?;
            let name = match state.preference.as_str() {
                "auto" => "Tự động (theo mô hình vừa dùng)",
                "gemini" => "Luôn ưu tiên Gemini",
                "claude_gpt" => "Luôn ưu tiên Claude & GPT",
                other => other,
            };
            println!("Cấu hình định tuyến mô hình (Antigravity):");
            println!("  • Chế độ ưu tiên:     {}", name);
            println!("  • Mô hình phát hiện:  {}", state.detected_category);
            println!("  • Nguồn phát hiện:    {}", state.last_detected_source);
            println!();
            println!("Thay đổi cấu hình: aam agy preference [auto | gemini | claude_gpt]");
        }
        Ok(())
    }

    async fn handle_auto_select(scoped_agent: Option<Agent>, args: &[String]) -> Result<()> {
        let mut agent = scoped_agent.unwrap_or(Agent::Antigravity);
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-a" | "--agent" => {
                    if let Some(val) = iter.next() {
                        agent = Self::parse_agent(val)?;
                    } else {
                        bail!("Thiếu tên agent sau tùy chọn -a / --agent");
                    }
                }
                "-h" | "--help" => {
                    let prefix = match scoped_agent {
                        Some(Agent::Antigravity) => "aam agy",
                        Some(Agent::Codex) => "aam codex",
                        Some(Agent::Claude) => "aam claude",
                        None => "aam",
                    };
                    println!("Cách sử dụng: {} auto-select [tùy chọn]", prefix);
                    println!();
                    println!("Tùy chọn:");
                    if scoped_agent.is_none() {
                        println!("  -a, --agent <agent>   Chỉ định agent (mặc định: antigravity)");
                    }
                    return Ok(());
                }
                unknown => bail!("Tùy chọn không hợp lệ: '{}'", unknown),
            }
        }

        let client = ApiClient::new();
        let msg = client.auto_select(agent).await?;
        println!("✓ {}", msg);
        Ok(())
    }

    async fn handle_delete(scoped_agent: Option<Agent>, args: &[String]) -> Result<()> {
        let mut target = None;
        let mut agent_filter = scoped_agent;
        let mut auto_confirm = false;
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-a" | "--agent" => {
                    if let Some(val) = iter.next() {
                        agent_filter = Some(Self::parse_agent(val)?);
                    } else {
                        bail!("Thiếu tên agent sau tùy chọn -a / --agent");
                    }
                }
                "-y" | "--yes" => auto_confirm = true,
                "-h" | "--help" => {
                    let prefix = match scoped_agent {
                        Some(Agent::Antigravity) => "aam agy",
                        Some(Agent::Codex) => "aam codex",
                        Some(Agent::Claude) => "aam claude",
                        None => "aam",
                    };
                    println!("Cách sử dụng: {} delete <# | ID | email> [tùy chọn]", prefix);
                    println!();
                    println!("Tùy chọn:");
                    if scoped_agent.is_none() {
                        println!("  -a, --agent <agent>   Chỉ định agent");
                    }
                    println!("  -y, --yes             Bỏ qua xác nhận xóa");
                    return Ok(());
                }
                val if !val.starts_with('-') && target.is_none() => {
                    target = Some(val.to_string());
                }
                unknown => bail!("Tùy chọn không hợp lệ: '{}'", unknown),
            }
        }

        let Some(target) = target else {
            let prefix = match scoped_agent {
                Some(Agent::Antigravity) => "aam agy",
                Some(Agent::Codex) => "aam codex",
                Some(Agent::Claude) => "aam claude",
                None => "aam",
            };
            bail!("Thiếu định danh tài khoản cần xóa. Cách dùng: {} delete <# | ID | email>", prefix);
        };

        let client = ApiClient::new();
        let accounts = if let Some(agent) = agent_filter {
            client.list_accounts_for_agent(agent).await?
        } else {
            client.list_all_accounts().await?
        };

        let matched = Self::find_account(&accounts, &target)?;

        if !auto_confirm {
            print!("Bạn có chắc chắn muốn xóa tài khoản {} ({})? [y/N]: ", matched.email, matched.agent.name());
            std::io::stdout().flush()?;
            let mut input = String::new();
            std::io::stdin().read_line(&mut input)?;
            let trimmed = input.trim().to_lowercase();
            if trimmed != "y" && trimmed != "yes" {
                println!("Đã hủy thao tác xóa tài khoản.");
                return Ok(());
            }
        }

        client.delete_account(matched.agent, &matched.id).await?;
        println!("✓ Đã xóa tài khoản {} ({}) thành công.", matched.email, matched.agent.name());
        Ok(())
    }

    async fn handle_settings(scoped_agent: Option<Agent>, args: &[String]) -> Result<()> {
        let mut agent_filter = scoped_agent;
        let mut toggle = None;
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-a" | "--agent" => {
                    if let Some(val) = iter.next() {
                        agent_filter = Some(Self::parse_agent(val)?);
                    } else {
                        bail!("Thiếu tên agent sau tùy chọn -a / --agent");
                    }
                }
                "--enable" => toggle = Some(true),
                "--disable" => toggle = Some(false),
                "-h" | "--help" => {
                    let prefix = match scoped_agent {
                        Some(Agent::Antigravity) => "aam agy",
                        Some(Agent::Codex) => "aam codex",
                        Some(Agent::Claude) => "aam claude",
                        None => "aam",
                    };
                    println!("Cách sử dụng: {} settings [tùy chọn]", prefix);
                    println!();
                    println!("Tùy chọn:");
                    if scoped_agent.is_none() {
                        println!("  -a, --agent <agent>   Chỉ định agent (antigravity, codex, claude)");
                    }
                    println!("  --enable              Bật tự động chọn tài khoản cho agent");
                    println!("  --disable             Tắt tự động chọn tài khoản cho agent");
                    return Ok(());
                }
                unknown => bail!("Tùy chọn không hợp lệ: '{}'", unknown),
            }
        }

        let client = ApiClient::new();
        if let Some(enabled) = toggle {
            let agent = agent_filter.unwrap_or(Agent::Antigravity);
            client.set_agent_setting(agent, enabled).await?;
            let state_str = if enabled { "bật" } else { "tắt" };
            println!("✓ Đã {} chế độ tự động chọn tài khoản cho {}.", state_str, agent.name());
        } else if let Some(agent) = agent_filter {
            let flags = client.get_agent_settings().await?;
            let is_enabled = flags.is_enabled(agent);
            println!("Cài đặt tự động chọn tài khoản cho {}:", agent.name());
            println!("  • Trạng thái: {}", if is_enabled { "Bật" } else { "Tắt" });
            println!();
            let alias = match agent {
                Agent::Antigravity => "agy",
                Agent::Codex => "codex",
                Agent::Claude => "claude",
            };
            println!("Thay đổi cài đặt: aam {} settings [--enable | --disable]", alias);
        } else {
            let flags = client.get_agent_settings().await?;
            println!("Cài đặt tự động chọn tài khoản (auto-selection):");
            println!("  • Antigravity:  {}", if flags.antigravity { "Bật" } else { "Tắt" });
            println!("  • Codex:        {}", if flags.codex { "Bật" } else { "Tắt" });
            println!("  • Claude:       {}", if flags.claude { "Bật" } else { "Tắt" });
            println!();
            println!("Thay đổi cấu hình: aam <agent> settings [--enable | --disable]");
        }
        Ok(())
    }

    async fn handle_add(scoped_agent: Option<Agent>, args: &[String]) -> Result<()> {
        let mut agent = scoped_agent.unwrap_or(Agent::Antigravity);
        let mut email = None;
        let mut access_token = None;
        let mut refresh_token = None;
        let mut id_token = None;
        let mut expires_in = None;
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-a" | "--agent" => {
                    if let Some(val) = iter.next() {
                        agent = Self::parse_agent(val)?;
                    }
                }
                "--email" => email = iter.next().cloned(),
                "--access-token" | "--token" => access_token = iter.next().cloned(),
                "--refresh-token" => refresh_token = iter.next().cloned(),
                "--id-token" => id_token = iter.next().cloned(),
                "--expires-in" => {
                    if let Some(val) = iter.next() {
                        expires_in = val.parse::<i64>().ok();
                    }
                }
                "-h" | "--help" => {
                    let prefix = match scoped_agent {
                        Some(Agent::Antigravity) => "aam agy",
                        Some(Agent::Codex) => "aam codex",
                        Some(Agent::Claude) => "aam claude",
                        None => "aam",
                    };
                    match agent {
                        Agent::Antigravity => {
                            println!("Cách sử dụng: {} add --email <email> --access-token <token> [tùy chọn]", prefix);
                            println!();
                            println!("Tùy chọn:");
                            println!("  --email <email>             Email tài khoản Google");
                            println!("  --access-token <token>      Access token");
                            println!("  --refresh-token <token>     Refresh token");
                            println!("  --expires-in <giây>         Thời gian hiệu lực của token");
                        }
                        Agent::Codex => {
                            println!("Cách sử dụng: {} add --email <email> --token <token> [tùy chọn]", prefix);
                            println!();
                            println!("Tùy chọn:");
                            println!("  --email <email>             Email tài khoản Codex");
                            println!("  --token <token>             Access token hoặc API key");
                            println!("  --id-token <token>          ID token (tùy chọn)");
                        }
                        Agent::Claude => {
                            println!("Cách sử dụng: {} add --email <email> --token <token>", prefix);
                            println!();
                            println!("Tùy chọn:");
                            println!("  --email <email>             Email hoặc định danh tài khoản");
                            println!("  --token <token>             Session token của Claude Code");
                        }
                    }
                    return Ok(());
                }
                unknown => bail!("Tùy chọn không hợp lệ: '{}'", unknown),
            }
        }

        let Some(email) = email else {
            bail!("Thiếu email tài khoản (--email)");
        };

        let client = ApiClient::new();
        match agent {
            Agent::Antigravity => {
                let Some(token) = access_token else {
                    bail!("Thiếu access token (--access-token)");
                };
                client
                    .add_antigravity_account(
                        &email,
                        &token,
                        refresh_token.as_deref(),
                        expires_in,
                    )
                    .await?;
                println!("✓ Đã thêm tài khoản Antigravity {} thành công.", email);
            }
            Agent::Codex => {
                let Some(token) = access_token else {
                    bail!("Thiếu token (--token)");
                };
                let mut creds_map = serde_json::Map::new();
                creds_map.insert("token".to_string(), serde_json::json!(token));
                if let Some(id_tok) = id_token {
                    creds_map.insert("id_token".to_string(), serde_json::json!(id_tok));
                }
                client.import_native_account(Agent::Codex, &email, Some(serde_json::Value::Object(creds_map))).await?;
                println!("✓ Đã thêm tài khoản Codex {} thành công.", email);
            }
            Agent::Claude => {
                let Some(token) = access_token else {
                    bail!("Thiếu token (--token)");
                };
                let mut creds_map = serde_json::Map::new();
                creds_map.insert("token".to_string(), serde_json::json!(token));
                client.import_native_account(Agent::Claude, &email, Some(serde_json::Value::Object(creds_map))).await?;
                println!("✓ Đã thêm tài khoản Claude {} thành công.", email);
            }
        }
        Ok(())
    }

    async fn handle_import(scoped_agent: Option<Agent>, args: &[String]) -> Result<()> {
        let mut agent = scoped_agent;
        let mut email = None;
        let mut token = None;
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-a" | "--agent" => {
                    if let Some(val) = iter.next() {
                        let a = Self::parse_agent(val)?;
                        if a == Agent::Antigravity {
                            bail!("Lệnh import chỉ dành cho Codex và Claude. Đối với Antigravity, vui lòng dùng 'aam agy add' hoặc 'aam agy login'.");
                        }
                        agent = Some(a);
                    }
                }
                "--email" => email = iter.next().cloned(),
                "--token" => token = iter.next().cloned(),
                "-h" | "--help" => {
                    let prefix = match scoped_agent {
                        Some(Agent::Codex) => "aam codex",
                        Some(Agent::Claude) => "aam claude",
                        _ => "aam",
                    };
                    println!("Cách sử dụng: {} import [--email <email>] [--token <token>]", prefix);
                    println!();
                    println!("Ghi chú: Nếu không truyền --token, hệ thống sẽ tự động nhập phiên đăng nhập hiện tại từ CLI cục bộ.");
                    return Ok(());
                }
                unknown => bail!("Tùy chọn không hợp lệ: '{}'", unknown),
            }
        }

        let Some(agent) = agent else {
            bail!("Thiếu agent cần import (-a codex hoặc -a claude)");
        };

        if agent == Agent::Antigravity {
            bail!("Lệnh import chỉ dành cho Codex và Claude. Đối với Antigravity, vui lòng dùng 'aam agy add' hoặc 'aam agy login'.");
        }

        let email = match email {
            Some(e) => e,
            None => format!("{}-local", agent.name().to_lowercase()),
        };

        let creds = token.map(|t| serde_json::json!({ "token": t }));
        let client = ApiClient::new();
        client.import_native_account(agent, &email, creds).await?;
        println!("✓ Đã nhập tài khoản {} cho {} thành công.", email, agent.name());
        Ok(())
    }

    async fn handle_login(scoped_agent: Option<Agent>, args: &[String]) -> Result<()> {
        let mut agent = scoped_agent.unwrap_or(Agent::Antigravity);
        let mut iter = args.iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-a" | "--agent" => {
                    if let Some(val) = iter.next() {
                        agent = Self::parse_agent(val)?;
                    }
                }
                "-h" | "--help" => {
                    let prefix = match scoped_agent {
                        Some(Agent::Antigravity) => "aam agy",
                        Some(Agent::Codex) => "aam codex",
                        Some(Agent::Claude) => "aam claude",
                        None => "aam",
                    };
                    println!("Cách sử dụng: {} login [tùy chọn]", prefix);
                    println!();
                    println!("Tùy chọn:");
                    if scoped_agent.is_none() {
                        println!("  -a, --agent <agent>   Chỉ định agent (antigravity, codex)");
                    }
                    return Ok(());
                }
                unknown => bail!("Tùy chọn không hợp lệ: '{}'", unknown),
            }
        }

        let client = ApiClient::new();
        match agent {
            Agent::Antigravity => {
                println!("[aam] Đang khởi tạo phiên xác thực Google OAuth...");
                let auth_url = client.start_oauth_flow().await?;
                println!("Vui lòng mở liên kết sau trên trình duyệt để hoàn tất đăng nhập:\n");
                println!("  {}\n", auth_url);
                let _ = Self::open_url(&auth_url);
                println!("Sau khi cấp quyền thành công, tài khoản sẽ tự động được lưu vào aam.");
            }
            Agent::Codex => {
                println!("[aam] Đang khởi tạo phiên xác thực Codex...");
                let status = client.start_codex_login().await?;
                if let Some(auth_url) = status.auth_url {
                    println!("Vui lòng mở liên kết sau để xác thực:\n  {}\n", auth_url);
                    let _ = Self::open_url(&auth_url);
                }
                println!("Đang chờ xác nhận từ trình duyệt (nhấn Ctrl+C để hủy)...");
                for _ in 0..120 {
                    tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
                    if let Ok(Some(current)) = client.get_codex_login_status().await {
                        if current.status == "completed" {
                            println!("\n✓ Đăng nhập Codex thành công!");
                            return Ok(());
                        } else if current.status == "failed" {
                            bail!("\n✗ Đăng nhập Codex thất bại: {}", current.message);
                        }
                    }
                }
                bail!("\nHết thời gian chờ đăng nhập Codex (timeout)");
            }
            Agent::Claude => {
                println!("Claude không hỗ trợ đăng nhập OAuth trực tiếp qua CLI.");
                println!("Vui lòng đăng nhập qua CLI của Claude hoặc dùng lệnh 'aam claude import'.");
            }
        }
        Ok(())
    }

    async fn handle_reset() -> Result<()> {
        let client = ApiClient::new();
        client.reset_cooldowns().await?;
        println!("✓ Đã đặt lại thời gian chờ cho các tài khoản Antigravity.");
        Ok(())
    }

    pub fn parse_agent(val: &str) -> Result<Agent> {
        match val.to_lowercase().as_str() {
            "antigravity" | "anti" | "ag" | "agy" => Ok(Agent::Antigravity),
            "codex" => Ok(Agent::Codex),
            "claude" => Ok(Agent::Claude),
            _ => bail!("Tên agent không hợp lệ: '{}'. Chỉ chấp nhận: antigravity (agy), codex, claude", val),
        }
    }

    fn find_account<'a>(accounts: &'a [UnifiedAccountDto], target: &str) -> Result<&'a UnifiedAccountDto> {
        if let Ok(idx) = target.parse::<usize>() {
            if idx >= 1 && idx <= accounts.len() {
                return Ok(&accounts[idx - 1]);
            }
        }

        let target_lower = target.to_lowercase();
        if let Some(acc) = accounts.iter().find(|a| a.id == target) {
            return Ok(acc);
        }
        if let Some(acc) = accounts.iter().find(|a| a.email.to_lowercase() == target_lower) {
            return Ok(acc);
        }
        let id_matches: Vec<_> = accounts.iter().filter(|a| a.id.starts_with(target)).collect();
        if id_matches.len() == 1 {
            return Ok(id_matches[0]);
        }
        let email_matches: Vec<_> = accounts.iter().filter(|a| a.email.to_lowercase().contains(&target_lower)).collect();
        if email_matches.len() == 1 {
            return Ok(email_matches[0]);
        }

        if id_matches.len() > 1 || email_matches.len() > 1 {
            bail!("Có nhiều tài khoản khớp với '{}'. Vui lòng dùng số thứ tự (#) hoặc ID đầy đủ.", target);
        }

        bail!("Không tìm thấy tài khoản phù hợp với '{}'. Gõ 'aam list' để xem danh sách.", target);
    }

    pub fn open_url(url: &str) -> Result<()> {
        #[cfg(target_os = "linux")]
        {
            let _ = Command::new("xdg-open")
                .arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("open")
                .arg(url)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        #[cfg(target_os = "windows")]
        {
            let _ = Command::new("cmd")
                .args(["/C", "start", "", url])
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn();
        }
        Ok(())
    }

    pub fn truncate_str(s: &str, max_chars: usize) -> String {
        let char_count = s.chars().count();
        if char_count > max_chars {
            let truncated: String = s.chars().take(max_chars.saturating_sub(3)).collect();
            format!("{}...", truncated)
        } else {
            s.to_string()
        }
    }

    fn print_help() {
        println!("Agent Account Manager CLI (aam)");
        println!();
        println!("Cách sử dụng:");
        println!("  aam [lệnh_chung] [tùy_chọn]");
        println!("  aam <agent> <lệnh_con> [tùy_chọn]");
        println!();
        println!("Quản lý theo từng agent:");
        println!("  aam agy [lệnh]        Quản lý tài khoản Antigravity (viết tắt: agy, antigravity, anti, ag)");
        println!("  aam codex [lệnh]      Quản lý tài khoản OpenAI Codex CLI");
        println!("  aam claude [lệnh]     Quản lý tài khoản Claude Code CLI");
        println!();
        println!("Các lệnh con cho từng agent (ví dụ: aam codex list, aam agy switch 1):");
        println!("  list, ls              Xem danh sách tài khoản của agent (--active, --json)");
        println!("  switch <# | ID>       Chuyển tài khoản đang hoạt động của agent");
        println!("  refresh               Làm mới hạn ngạch của agent");
        println!("  auto-select           Tự động chọn tài khoản tối ưu cho agent");
        println!("  settings              Xem hoặc cấu hình tự động chọn (--enable/--disable)");
        println!("  login                 Đăng nhập tài khoản mới (agy: Google OAuth, codex: Device OAuth)");
        println!("  import                Nhập tài khoản từ CLI cục bộ (codex, claude)");
        println!("  add                   Thêm tài khoản thủ công bằng token");
        println!("  delete <# | ID>       Xóa tài khoản của agent (-y)");
        println!("  preference            Xem hoặc đổi cấu hình ưu tiên mô hình (chỉ dành cho Antigravity)");
        println!("  reset                 Đặt lại thời gian chờ (cooldowns) cho Antigravity");
        println!();
        println!("Kiểm tra và thao tác chung (toàn bộ agent):");
        println!("  aam                   Xem bảng danh sách tài khoản toàn bộ agent (tương đương 'aam list')");
        println!("  aam check             Kiểm tra tổng quan trạng thái dịch vụ và tài khoản các agent");
        println!("  aam list              Xem bảng danh sách tài khoản của toàn bộ agent");
        println!("  aam switch <# | ID>   Chuyển tài khoản đang hoạt động");
        println!("  aam refresh           Làm mới hạn ngạch cho toàn bộ agent");
        println!("  aam tui               Mở giao diện terminal tương tác toàn màn hình");
        println!("  aam web               Mở bảng điều khiển web trên trình duyệt (hoặc 'aam open')");
        println!();
        println!("Quản lý dịch vụ hệ thống:");
        println!("  aam start             Khởi chạy dịch vụ chạy ngầm");
        println!("  aam stop              Dừng dịch vụ đang chạy");
        println!("  aam restart           Khởi động lại dịch vụ");
        println!("  aam status            Xem trạng thái hoạt động của dịch vụ");
        println!("  aam autostart         Bật tự động chạy liên tục cùng hệ thống (systemd)");
        println!("  aam disable           Tắt chế độ tự khởi động cùng máy");
        println!("  aam update            Cập nhật aam lên phiên bản mới nhất từ GitHub");
        println!("  aam version           Xem phiên bản hiện tại");
        println!();
    }
}

#[cfg(test)]
mod tests {
    use super::Cli;
    use std::path::PathBuf;

    #[test]
    fn accepts_only_well_formed_sha256() {
        let valid = "a".repeat(64);
        assert_eq!(
            Cli::parse_sha256(&format!("{}  release.tar.gz", valid)).unwrap(),
            valid
        );
        assert!(Cli::parse_sha256("not-a-checksum").is_err());
    }

    #[test]
    fn quotes_systemd_exec_paths() {
        let quoted = Cli::quote_systemd_exec_path(&PathBuf::from("/tmp/a b/%x$y")).unwrap();
        assert_eq!(quoted, "\"/tmp/a b/%%x$$y\"");
        assert!(Cli::quote_systemd_exec_path(&PathBuf::from("/tmp/a\nInjected=true")).is_err());
    }

    #[test]
    fn parses_browser_bootstrap_response() {
        let response = "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\n\r\n{\"bootstrap_token\":\"safe_token-123\"}";
        assert_eq!(
            Cli::parse_browser_bootstrap_response(response).unwrap(),
            "safe_token-123"
        );
        assert!(
            Cli::parse_browser_bootstrap_response("HTTP/1.1 401 Unauthorized\r\n\r\n").is_err()
        );
    }

    #[test]
    fn parses_uninstall_flags_correctly() {
        let purge_args = vec!["aam".to_string(), "uninstall".to_string(), "--purge".to_string()];
        let is_purge = purge_args.iter().skip(2).any(|arg| arg == "--purge" || arg == "-p");
        assert!(is_purge);

        let short_purge_args = vec!["aam".to_string(), "uninstall".to_string(), "-p".to_string()];
        let is_short_purge = short_purge_args.iter().skip(2).any(|arg| arg == "--purge" || arg == "-p");
        assert!(is_short_purge);

        let keep_args = vec!["aam".to_string(), "uninstall".to_string(), "--keep-data".to_string()];
        let is_keep = keep_args.iter().skip(2).any(|arg| arg == "--keep-data" || arg == "--no-purge");
        assert!(is_keep);
    }

    #[test]
    fn parses_agent_names_correctly() {
        use crate::proxy::selection::Agent;
        assert_eq!(Cli::parse_agent("antigravity").unwrap(), Agent::Antigravity);
        assert_eq!(Cli::parse_agent("agy").unwrap(), Agent::Antigravity);
        assert_eq!(Cli::parse_agent("anti").unwrap(), Agent::Antigravity);
        assert_eq!(Cli::parse_agent("ag").unwrap(), Agent::Antigravity);
        assert_eq!(Cli::parse_agent("codex").unwrap(), Agent::Codex);
        assert_eq!(Cli::parse_agent("claude").unwrap(), Agent::Claude);
        assert!(Cli::parse_agent("unknown").is_err());
    }

    #[test]
    fn finds_account_by_index_id_or_email() {
        use crate::client::UnifiedAccountDto;
        use crate::proxy::selection::Agent;

        let accounts = vec![
            UnifiedAccountDto {
                id: "11112222-3333-4444-5555-666677778888".to_string(),
                agent: Agent::Antigravity,
                email: "alpha@gmail.com".to_string(),
                is_active: true,
                quota_percentage: Some(90.0),
                quota_groups: vec![],
                checked_at: None,
                status: "Sẵn sàng".to_string(),
                error: None,
            },
            UnifiedAccountDto {
                id: "aaaa2222-3333-4444-5555-666677778888".to_string(),
                agent: Agent::Codex,
                email: "beta@company.com".to_string(),
                is_active: false,
                quota_percentage: Some(75.0),
                quota_groups: vec![],
                checked_at: None,
                status: "Sẵn sàng".to_string(),
                error: None,
            },
        ];

        // 1. By index (#)
        assert_eq!(Cli::find_account(&accounts, "1").unwrap().email, "alpha@gmail.com");
        assert_eq!(Cli::find_account(&accounts, "2").unwrap().email, "beta@company.com");

        // 2. By exact email
        assert_eq!(Cli::find_account(&accounts, "alpha@gmail.com").unwrap().id, "11112222-3333-4444-5555-666677778888");

        // 3. By email substring
        assert_eq!(Cli::find_account(&accounts, "beta").unwrap().email, "beta@company.com");

        // 4. By ID prefix
        assert_eq!(Cli::find_account(&accounts, "11112222").unwrap().email, "alpha@gmail.com");
        assert_eq!(Cli::find_account(&accounts, "aaaa2222").unwrap().email, "beta@company.com");

        // 5. Not found
        assert!(Cli::find_account(&accounts, "nonexistent").is_err());
        assert!(Cli::find_account(&accounts, "99").is_err());
    }

    #[tokio::test]
    async fn routes_agent_commands_correctly() {
        // Agent help routes
        let res = Cli::handle_args(&["aam".to_string(), "agy".to_string(), "help".to_string()]).await;
        assert!(matches!(res, Some(Ok(()))));

        let res = Cli::handle_args(&["aam".to_string(), "codex".to_string(), "help".to_string()]).await;
        assert!(matches!(res, Some(Ok(()))));

        let res = Cli::handle_args(&["aam".to_string(), "claude".to_string(), "help".to_string()]).await;
        assert!(matches!(res, Some(Ok(()))));

        // General help and version routes
        let res = Cli::handle_args(&["aam".to_string(), "help".to_string()]).await;
        assert!(matches!(res, Some(Ok(()))));

        let res = Cli::handle_args(&["aam".to_string(), "version".to_string()]).await;
        assert!(matches!(res, Some(Ok(()))));
    }
}
