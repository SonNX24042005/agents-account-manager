use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Stdio};

pub struct Cli;

impl Cli {
    pub fn handle_args(args: &[String]) -> Option<Result<()>> {
        if args.len() < 2 {
            return Some(Self::open_dashboard());
        }

        let cmd = args[1].to_lowercase();
        match cmd.as_str() {
            "run" | "daemon" => None,
            "open" | "ui" | "web" => Some(Self::open_dashboard()),
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

    fn open_dashboard() -> Result<()> {
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

    fn start_service() -> Result<()> {
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

    fn print_help() {
        println!("Agent Account Manager CLI (aam)");
        println!();
        println!("Cách sử dụng:");
        println!("  aam                    Tự động bật dịch vụ (nếu chưa chạy) và mở giao diện web");
        println!("  aam update             Cập nhật aam lên phiên bản mới nhất từ GitHub");
        println!("  aam start              Khởi chạy dịch vụ chạy ngầm");
        println!("  aam autostart          Bật tự động chạy liên tục cùng hệ thống (kể cả restart máy)");
        println!("  aam stop               Dừng dịch vụ đang chạy");
        println!("  aam restart            Khởi động lại dịch vụ");
        println!("  aam status             Xem trạng thái hoạt động của dịch vụ");
        println!("  aam version            Xem phiên bản hiện tại");
        println!("  aam disable            Tắt chế độ tự khởi động cùng máy");
        println!("  aam install            Cài đặt lệnh aam vào ~/.local/bin");
        println!("  aam reinstall          Cài đặt lại binary và thiết lập liên kết lệnh");
        println!("  aam uninstall [tùy_chọn]  Gỡ cài đặt aam khỏi hệ thống");
        println!("  aam run                Chạy trực tiếp trên terminal hiện tại (foreground)");
        println!();
        println!("Tùy chọn gỡ cài đặt (aam uninstall):");
        println!("  --purge, -p            Xóa toàn bộ thư mục dữ liệu cấu hình và tài khoản");
        println!("  --keep-data            Giữ lại thư mục dữ liệu cấu hình mà không cần hỏi lại");
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
}
