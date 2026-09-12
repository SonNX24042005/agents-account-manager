use crate::client::{ApiClient, ModelRoutingStateDto, SelectionFlagsDto, UnifiedAccountDto};
use crate::proxy::selection::Agent;
use anyhow::Result;
use crossterm::{
    event::{Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use futures_util::StreamExt;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Gauge, Paragraph, Row, Table, TableState, Tabs, Wrap},
    Frame, Terminal,
};
use std::io::stdout;
use std::time::{Duration, Instant};

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

#[derive(Clone)]
pub enum ActiveModal {
    DeleteConfirm(UnifiedAccountDto),
    AddMenu,
    CodexLoginProgress {
        id: String,
        auth_url: Option<String>,
        status: String,
        message: String,
    },
    ManualInput {
        active_field: usize, // 0 = email, 1 = token
        email: String,
        token: String,
        error_msg: Option<String>,
    },
}

pub struct TuiApp {
    client: ApiClient,
    current_agent: Agent,
    accounts: Vec<UnifiedAccountDto>,
    table_state: TableState,
    preference: Option<ModelRoutingStateDto>,
    settings: Option<SelectionFlagsDto>,
    status_msg: Option<(String, Instant, bool)>, // (text, time, is_error)
    active_modal: Option<ActiveModal>,
    should_quit: bool,
}

impl TuiApp {
    pub fn new(client: ApiClient) -> Self {
        let mut table_state = TableState::default();
        table_state.select(Some(0));
        Self {
            client,
            current_agent: Agent::Antigravity,
            accounts: Vec::new(),
            table_state,
            preference: None,
            settings: None,
            status_msg: Some((
                "Sử dụng mũi tên để chọn, Enter để chuyển, phím + hoặc n để thêm tài khoản mới".to_string(),
                Instant::now(),
                false,
            )),
            active_modal: None,
            should_quit: false,
        }
    }

    pub fn set_status(&mut self, text: impl Into<String>, is_error: bool) {
        self.status_msg = Some((text.into(), Instant::now(), is_error));
    }

    pub async fn fetch_data(&mut self) {
        if let Ok(accounts) = self.client.list_accounts_for_agent(self.current_agent).await {
            self.accounts = accounts;
            if self.accounts.is_empty() {
                self.table_state.select(None);
            } else {
                let curr = self.table_state.selected().unwrap_or(0);
                if curr >= self.accounts.len() {
                    self.table_state.select(Some(self.accounts.len() - 1));
                } else if self.table_state.selected().is_none() {
                    self.table_state.select(Some(0));
                }
            }
        }

        if self.current_agent == Agent::Antigravity {
            if let Ok(pref) = self.client.get_preference().await {
                self.preference = Some(pref);
            }
        }

        if let Ok(settings) = self.client.get_agent_settings().await {
            self.settings = Some(settings);
        }
    }

    fn select_next(&mut self) {
        if self.accounts.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.accounts.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn select_prev(&mut self) {
        if self.accounts.is_empty() {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.accounts.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub async fn switch_tab(&mut self, agent: Agent) {
        if self.current_agent != agent {
            self.current_agent = agent;
            self.table_state.select(Some(0));
            self.fetch_data().await;
            self.set_status(format!("Đã chuyển sang xem agent {}", agent.name()), false);
        }
    }

    pub async fn switch_selected_account(&mut self) {
        let Some(idx) = self.table_state.selected() else { return; };
        let Some(acc) = self.accounts.get(idx).cloned() else { return; };

        self.set_status(format!("Đang chuyển sang tài khoản {}...", acc.email), false);
        match self.client.switch_account(self.current_agent, &acc.id).await {
            Ok(()) => {
                self.set_status(format!("✓ Đã chuyển sang tài khoản {}", acc.email), false);
                self.fetch_data().await;
            }
            Err(e) => {
                self.set_status(format!("✗ Chuyển tài khoản thất bại: {}", e), true);
            }
        }
    }

    pub async fn refresh_selected_quota(&mut self) {
        self.set_status("Đang gửi yêu cầu làm mới hạn ngạch...", false);
        match self.client.refresh_quota(Some(self.current_agent)).await {
            Ok(()) => {
                tokio::time::sleep(Duration::from_millis(800)).await;
                self.fetch_data().await;
                self.set_status("✓ Đã làm mới dữ liệu hạn ngạch", false);
            }
            Err(e) => {
                self.set_status(format!("✗ Làm mới thất bại: {}", e), true);
            }
        }
    }

    pub async fn cycle_preference(&mut self) {
        if self.current_agent != Agent::Antigravity {
            self.set_status("Cấu hình ưu tiên mô hình chỉ áp dụng cho Antigravity", false);
            return;
        }

        let current_pref = self
            .preference
            .as_ref()
            .map(|p| p.preference.as_str())
            .unwrap_or("auto");

        let next_pref = match current_pref {
            "auto" => "gemini",
            "gemini" => "claude_gpt",
            _ => "auto",
        };

        match self.client.set_preference(next_pref).await {
            Ok(pref) => {
                let name = match pref.preference.as_str() {
                    "auto" => "Tự động (theo mô hình vừa dùng)",
                    "gemini" => "Luôn ưu tiên Gemini",
                    "claude_gpt" => "Luôn ưu tiên Claude & GPT",
                    other => other,
                }
                .to_string();
                self.preference = Some(pref);
                self.set_status(format!("✓ Đã đổi ưu tiên định tuyến thành: {}", name), false);
                self.fetch_data().await;
            }
            Err(e) => {
                self.set_status(format!("✗ Đổi ưu tiên thất bại: {}", e), true);
            }
        }
    }

    pub async fn trigger_auto_select(&mut self) {
        self.set_status("Đang tự động chọn tài khoản tối ưu...", false);
        match self.client.auto_select(self.current_agent).await {
            Ok(msg) => {
                self.set_status(format!("✓ {}", msg), false);
                self.fetch_data().await;
            }
            Err(e) => {
                self.set_status(format!("✗ Tự động chọn thất bại: {}", e), true);
            }
        }
    }

    pub async fn toggle_auto_selection_setting(&mut self) {
        let is_enabled = self
            .settings
            .as_ref()
            .map(|s| s.is_enabled(self.current_agent))
            .unwrap_or(false);

        let new_state = !is_enabled;
        match self.client.set_agent_setting(self.current_agent, new_state).await {
            Ok(()) => {
                let state_str = if new_state { "Bật" } else { "Tắt" };
                self.set_status(
                    format!("✓ Đã {} chế độ tự động chọn cho {}", state_str, self.current_agent.name()),
                    false,
                );
                self.fetch_data().await;
            }
            Err(e) => {
                self.set_status(format!("✗ Đổi cài đặt tự động chọn thất bại: {}", e), true);
            }
        }
    }

    pub async fn execute_delete(&mut self, acc: UnifiedAccountDto) {
        self.set_status(format!("Đang xóa tài khoản {}...", acc.email), false);
        match self.client.delete_account(self.current_agent, &acc.id).await {
            Ok(()) => {
                self.set_status(format!("✓ Đã xóa tài khoản {}", acc.email), false);
                self.fetch_data().await;
            }
            Err(e) => {
                self.set_status(format!("✗ Xóa tài khoản thất bại: {}", e), true);
            }
        }
    }
}

pub async fn run_tui() -> Result<()> {
    let client = ApiClient::new();
    client.ensure_service_ready().await?;

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = TuiApp::new(client);
    app.fetch_data().await;

    let mut event_stream = crossterm::event::EventStream::new();
    let mut refresh_interval = tokio::time::interval(Duration::from_millis(3000));
    refresh_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    while !app.should_quit {
        terminal.draw(|f| render_tui(f, &mut app))?;

        tokio::select! {
            _ = refresh_interval.tick() => {
                app.fetch_data().await;
                // Nếu đang trong tiến trình Codex login, cập nhật trạng thái
                if let Some(ActiveModal::CodexLoginProgress { .. }) = app.active_modal.clone() {
                    if let Ok(Some(st)) = app.client.get_codex_login_status().await {
                        if st.status == "completed" {
                            app.set_status("✓ Đăng nhập Codex thành công!", false);
                            app.active_modal = None;
                            app.fetch_data().await;
                        } else if st.status == "failed" {
                            app.set_status(format!("✗ Đăng nhập thất bại: {}", st.message), true);
                            app.active_modal = None;
                        }
                    }
                }
            }
            maybe_event = event_stream.next() => {
                if let Some(Ok(Event::Key(key))) = maybe_event {
                    if key.kind == KeyEventKind::Press {
                        handle_key_event(&mut app, key.code, key.modifiers).await;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_key_event(app: &mut TuiApp, key_code: KeyCode, modifiers: KeyModifiers) {
    // 1. Modal đang hiển thị
    if let Some(modal) = app.active_modal.clone() {
        match modal {
            ActiveModal::DeleteConfirm(acc) => {
                match key_code {
                    KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                        app.active_modal = None;
                        app.execute_delete(acc).await;
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        app.active_modal = None;
                        app.set_status("Đã hủy thao tác xóa tài khoản", false);
                    }
                    _ => {}
                }
                return;
            }
            ActiveModal::AddMenu => {
                match key_code {
                    KeyCode::Esc | KeyCode::Char('q') => {
                        app.active_modal = None;
                        app.set_status("Đã đóng menu thêm tài khoản", false);
                    }
                    // Antigravity options
                    KeyCode::Char('1') | KeyCode::Char('o') if app.current_agent == Agent::Antigravity => {
                        app.set_status("Đang khởi tạo phiên xác thực Google OAuth...", false);
                        match app.client.start_oauth_flow().await {
                            Ok(url) => {
                                let _ = crate::cli::Cli::open_url(&url);
                                app.set_status("✓ Đã mở trình duyệt đăng nhập Google OAuth. Đang chờ xác thực...", false);
                                app.active_modal = None;
                            }
                            Err(e) => {
                                app.set_status(format!("✗ Khởi tạo OAuth thất bại: {}", e), true);
                            }
                        }
                    }
                    KeyCode::Char('2') | KeyCode::Char('m') if app.current_agent == Agent::Antigravity => {
                        app.active_modal = Some(ActiveModal::ManualInput {
                            active_field: 0,
                            email: String::new(),
                            token: String::new(),
                            error_msg: None,
                        });
                    }
                    // Codex options
                    KeyCode::Char('1') | KeyCode::Char('o') if app.current_agent == Agent::Codex => {
                        app.set_status("Đang khởi tạo phiên đăng nhập thiết bị Codex...", false);
                        match app.client.start_codex_login().await {
                            Ok(status) => {
                                if let Some(ref url) = status.auth_url {
                                    let _ = crate::cli::Cli::open_url(url);
                                }
                                app.active_modal = Some(ActiveModal::CodexLoginProgress {
                                    id: status.id,
                                    auth_url: status.auth_url,
                                    status: status.status,
                                    message: status.message,
                                });
                            }
                            Err(e) => {
                                app.set_status(format!("✗ Khởi tạo đăng nhập Codex thất bại: {}", e), true);
                            }
                        }
                    }
                    KeyCode::Char('2') | KeyCode::Char('c') if app.current_agent == Agent::Codex => {
                        app.set_status("Đang nhập phiên Codex từ máy...", false);
                        match app.client.import_native_account(Agent::Codex, "codex-cli", None).await {
                            Ok(()) => {
                                app.set_status("✓ Đã nhập phiên Codex từ máy thành công", false);
                                app.active_modal = None;
                                app.fetch_data().await;
                            }
                            Err(e) => {
                                app.set_status(format!("✗ Nhập tài khoản Codex thất bại: {}", e), true);
                            }
                        }
                    }
                    KeyCode::Char('3') | KeyCode::Char('m') if app.current_agent == Agent::Codex => {
                        app.active_modal = Some(ActiveModal::ManualInput {
                            active_field: 0,
                            email: String::new(),
                            token: String::new(),
                            error_msg: None,
                        });
                    }
                    // Claude options
                    KeyCode::Char('1') | KeyCode::Char('c') if app.current_agent == Agent::Claude => {
                        app.set_status("Đang nhập phiên Claude Code từ máy...", false);
                        match app.client.import_native_account(Agent::Claude, "claude-cli", None).await {
                            Ok(()) => {
                                app.set_status("✓ Đã nhập phiên Claude Code từ máy thành công", false);
                                app.active_modal = None;
                                app.fetch_data().await;
                            }
                            Err(e) => {
                                app.set_status(format!("✗ Nhập tài khoản Claude thất bại: {}", e), true);
                            }
                        }
                    }
                    KeyCode::Char('2') | KeyCode::Char('m') if app.current_agent == Agent::Claude => {
                        app.active_modal = Some(ActiveModal::ManualInput {
                            active_field: 0,
                            email: String::new(),
                            token: String::new(),
                            error_msg: None,
                        });
                    }
                    _ => {}
                }
                return;
            }
            ActiveModal::CodexLoginProgress { id, .. } => {
                match key_code {
                    KeyCode::Esc | KeyCode::Char('c') | KeyCode::Char('q') => {
                        let _ = app.client.cancel_codex_login(&id).await;
                        app.active_modal = None;
                        app.set_status("Đã hủy phiên đăng nhập Codex", false);
                    }
                    _ => {}
                }
                return;
            }
            ActiveModal::ManualInput {
                mut active_field,
                mut email,
                mut token,
                ..
            } => {
                match key_code {
                    KeyCode::Esc => {
                        app.active_modal = None;
                        app.set_status("Đã hủy nhập tài khoản", false);
                    }
                    KeyCode::Tab | KeyCode::Down | KeyCode::Up => {
                        active_field = 1 - active_field;
                        app.active_modal = Some(ActiveModal::ManualInput {
                            active_field,
                            email,
                            token,
                            error_msg: None,
                        });
                    }
                    KeyCode::Backspace => {
                        if active_field == 0 {
                            email.pop();
                        } else {
                            token.pop();
                        }
                        app.active_modal = Some(ActiveModal::ManualInput {
                            active_field,
                            email,
                            token,
                            error_msg: None,
                        });
                    }
                    KeyCode::Char(c) => {
                        if active_field == 0 {
                            email.push(c);
                        } else {
                            token.push(c);
                        }
                        app.active_modal = Some(ActiveModal::ManualInput {
                            active_field,
                            email,
                            token,
                            error_msg: None,
                        });
                    }
                    KeyCode::Enter => {
                        let email_trim = email.trim().to_string();
                        let token_trim = token.trim().to_string();

                        if email_trim.is_empty() {
                            app.active_modal = Some(ActiveModal::ManualInput {
                                active_field: 0,
                                email,
                                token,
                                error_msg: Some("Vui lòng nhập email".to_string()),
                            });
                            return;
                        }
                        if token_trim.is_empty() {
                            app.active_modal = Some(ActiveModal::ManualInput {
                                active_field: 1,
                                email,
                                token,
                                error_msg: Some("Vui lòng nhập token xác thực".to_string()),
                            });
                            return;
                        }

                        let agent = app.current_agent;
                        let res = match agent {
                            Agent::Antigravity => {
                                app.client
                                    .add_antigravity_account(&email_trim, &token_trim, None, None)
                                    .await
                            }
                            Agent::Codex | Agent::Claude => {
                                app.client
                                    .import_native_account(
                                        agent,
                                        &email_trim,
                                        Some(serde_json::json!({ "token": token_trim })),
                                    )
                                    .await
                            }
                        };

                        match res {
                            Ok(()) => {
                                app.set_status(format!("✓ Đã thêm tài khoản {} ({})", email_trim, agent.name()), false);
                                app.active_modal = None;
                                app.fetch_data().await;
                            }
                            Err(e) => {
                                app.active_modal = Some(ActiveModal::ManualInput {
                                    active_field,
                                    email,
                                    token,
                                    error_msg: Some(e.to_string()),
                                });
                            }
                        }
                    }
                    _ => {}
                }
                return;
            }
        }
    }

    // 2. Không có modal, xử lý phím điều khiển chính
    match key_code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Tab => {
            let next_agent = match app.current_agent {
                Agent::Antigravity => Agent::Codex,
                Agent::Codex => Agent::Claude,
                Agent::Claude => Agent::Antigravity,
            };
            app.switch_tab(next_agent).await;
        }
        KeyCode::Char('1') => app.switch_tab(Agent::Antigravity).await,
        KeyCode::Char('2') => app.switch_tab(Agent::Codex).await,
        KeyCode::Char('3') => app.switch_tab(Agent::Claude).await,
        KeyCode::Down | KeyCode::Char('j') => app.select_next(),
        KeyCode::Up | KeyCode::Char('k') => app.select_prev(),
        KeyCode::Home => {
            if !app.accounts.is_empty() {
                app.table_state.select(Some(0));
            }
        }
        KeyCode::End => {
            if !app.accounts.is_empty() {
                app.table_state.select(Some(app.accounts.len() - 1));
            }
        }
        KeyCode::Enter | KeyCode::Char('s') => {
            app.switch_selected_account().await;
        }
        KeyCode::Char('+') | KeyCode::Char('n') => {
            app.active_modal = Some(ActiveModal::AddMenu);
        }
        KeyCode::Char('r') => {
            app.refresh_selected_quota().await;
        }
        KeyCode::Char('p') => {
            app.cycle_preference().await;
        }
        KeyCode::Char('a') => {
            app.trigger_auto_select().await;
        }
        KeyCode::Char('t') => {
            app.toggle_auto_selection_setting().await;
        }
        KeyCode::Char('d') => {
            if let Some(idx) = app.table_state.selected() {
                if let Some(acc) = app.accounts.get(idx) {
                    app.active_modal = Some(ActiveModal::DeleteConfirm(acc.clone()));
                }
            }
        }
        KeyCode::Char('w') => {
            let _ = crate::cli::Cli::open_dashboard_url();
            app.set_status("Đã mở giao diện web trên trình duyệt mặc định", false);
        }
        _ => {}
    }
}

fn render_tui(f: &mut Frame, app: &mut TuiApp) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tabs
            Constraint::Min(10),   // Main content (Table + Detail)
            Constraint::Length(3), // Status message
            Constraint::Length(2), // Help / shortcuts footer
        ])
        .split(f.area());

    render_header(f, chunks[0], app);
    render_main(f, chunks[1], app);
    render_status(f, chunks[2], app);
    render_footer(f, chunks[3]);

    if let Some(ref modal) = app.active_modal {
        match modal {
            ActiveModal::DeleteConfirm(acc) => render_delete_modal(f, acc),
            ActiveModal::AddMenu => render_add_menu_modal(f, app.current_agent),
            ActiveModal::CodexLoginProgress { auth_url, status, message, .. } => {
                render_codex_login_modal(f, auth_url.as_deref(), status, message);
            }
            ActiveModal::ManualInput { active_field, email, token, error_msg } => {
                render_manual_input_modal(f, app.current_agent, *active_field, email, token, error_msg.as_deref());
            }
        }
    }
}

fn render_header(f: &mut Frame, area: Rect, app: &TuiApp) {
    let tab_titles = vec!["[1] Antigravity", "[2] Codex", "[3] Claude"];
    let selected_tab_idx = match app.current_agent {
        Agent::Antigravity => 0,
        Agent::Codex => 1,
        Agent::Claude => 2,
    };

    let header_layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(45), Constraint::Min(20)])
        .split(area);

    let tabs = Tabs::new(tab_titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Agent "),
        )
        .select(selected_tab_idx)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );
    f.render_widget(tabs, header_layout[0]);

    let routing_text = if app.current_agent == Agent::Antigravity {
        let pref_str = app
            .preference
            .as_ref()
            .map(|p| match p.preference.as_str() {
                "auto" => "Tự động (theo mô hình vừa dùng)",
                "gemini" => "Luôn ưu tiên Gemini",
                "claude_gpt" => "Luôn ưu tiên Claude & GPT",
                other => other,
            })
            .unwrap_or("Đang tải...");
        let cat_str = app
            .preference
            .as_ref()
            .map(|p| p.detected_category.as_str())
            .unwrap_or("-");
        format!("Định tuyến: {} | Phát hiện: {}", pref_str, cat_str)
    } else {
        let is_auto = app
            .settings
            .as_ref()
            .map(|s| s.is_enabled(app.current_agent))
            .unwrap_or(false);
        format!(
            "Tự động chọn: {}",
            if is_auto { "Bật" } else { "Tắt" }
        )
    };

    let daemon_info = Paragraph::new(Line::from(vec![
        Span::styled("• Dịch vụ: ", Style::default().fg(Color::DarkGray)),
        Span::styled("Trực tuyến (cổng 8045)", Style::default().fg(Color::Green)),
        Span::raw(" | "),
        Span::styled(routing_text, Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Trạng thái hệ thống "),
    );
    f.render_widget(daemon_info, header_layout[1]);
}

fn render_main(f: &mut Frame, area: Rect, app: &mut TuiApp) {
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(area);

    render_accounts_table(f, main_chunks[0], app);
    render_account_detail(f, main_chunks[1], app);
}

fn render_accounts_table(f: &mut Frame, area: Rect, app: &mut TuiApp) {
    let header_cells = ["", "Email", "Hạn ngạch", "Trạng thái", "ID"]
        .into_iter()
        .map(|h| {
            ratatui::widgets::Cell::from(h).style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
        });
    let header = Row::new(header_cells).height(1).bottom_margin(1);

    let rows: Vec<Row> = app
        .accounts
        .iter()
        .enumerate()
        .map(|(_idx, acc)| {
            let active_marker = if acc.is_active { "* " } else { "  " };
            let quota_str = if let Some(q) = acc.quota_percentage {
                if q < 100.0 {
                    if let Some(cd) = acc.primary_reset_countdown() {
                        format!("{:.0}% ({})", q, cd)
                    } else {
                        format!("{:.0}%", q)
                    }
                } else {
                    format!("{:.0}%", q)
                }
            } else {
                "--".to_string()
            };

            let short_id = if acc.id.len() > 8 {
                &acc.id[..8]
            } else {
                &acc.id
            };

            let row_style = if acc.is_active {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else if acc.error.is_some() {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::White)
            };

            Row::new(vec![
                ratatui::widgets::Cell::from(active_marker),
                ratatui::widgets::Cell::from(acc.email.clone()),
                ratatui::widgets::Cell::from(quota_str),
                ratatui::widgets::Cell::from(acc.status.clone()),
                ratatui::widgets::Cell::from(short_id.to_string()),
            ])
            .style(row_style)
        })
        .collect();

    let table_title = format!(
        " Danh sách tài khoản {} ({}) ",
        app.current_agent.name(),
        app.accounts.len()
    );

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Percentage(38),
            Constraint::Length(16),
            Constraint::Length(15),
            Constraint::Length(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(table_title),
    )
    .row_highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .fg(Color::White)
            .add_modifier(Modifier::BOLD),
    )
    .highlight_symbol("> ");

    f.render_stateful_widget(table, area, &mut app.table_state);
}

fn render_account_detail(f: &mut Frame, area: Rect, app: &TuiApp) {
    let selected_acc = app
        .table_state
        .selected()
        .and_then(|idx| app.accounts.get(idx));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Chi tiết tài khoản ");

    let Some(acc) = selected_acc else {
        let empty_msg = Paragraph::new("Chưa có tài khoản nào. Nhấn [+] hoặc [n] để thêm tài khoản mới.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(block);
        f.render_widget(empty_msg, area);
        return;
    };

    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let detail_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4),
            Constraint::Min(6),
            Constraint::Length(3),
        ])
        .split(inner_area);

    let active_badge = if acc.is_active {
        Span::styled(" [Đang hoạt động]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
    } else {
        Span::styled(" [Chưa kích hoạt]", Style::default().fg(Color::DarkGray))
    };

    let info_text = vec![
        Line::from(vec![
            Span::styled("Email: ", Style::default().fg(Color::Cyan)),
            Span::styled(&acc.email, Style::default().add_modifier(Modifier::BOLD)),
            active_badge,
        ]),
        Line::from(vec![
            Span::styled("ID: ", Style::default().fg(Color::DarkGray)),
            Span::raw(&acc.id),
        ]),
        Line::from(vec![
            Span::styled("Kiểm tra lần cuối: ", Style::default().fg(Color::DarkGray)),
            Span::raw(
                acc.checked_at
                    .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                    .unwrap_or_else(|| "Chưa kiểm tra".to_string()),
            ),
        ]),
    ];
    let info_paragraph = Paragraph::new(info_text);
    f.render_widget(info_paragraph, detail_chunks[0]);

    let mut quota_widgets = Vec::new();
    if acc.quota_groups.is_empty() {
        if let Some(quota_val) = acc.quota_percentage {
            quota_widgets.push((
                "Hạn ngạch chung".to_string(),
                quota_val.clamp(0.0, 100.0),
                None,
            ));
        }
    } else {
        for group in &acc.quota_groups {
            for bucket in &group.buckets {
                let win_label = if bucket.is_5h() {
                    "5h"
                } else if bucket.is_weekly() {
                    "Tuần"
                } else {
                    &bucket.window
                };
                let label = format!("{}: {}", group.name, win_label);
                let pct = bucket.effective_percentage();
                quota_widgets.push((label, pct, bucket.reset_countdown()));
            }
        }
    }

    if quota_widgets.is_empty() {
        let no_quota = Paragraph::new("Chưa có thông tin hạn ngạch chi tiết")
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(no_quota, detail_chunks[1]);
    } else {
        let gauge_constraints: Vec<Constraint> = quota_widgets
            .iter()
            .map(|_| Constraint::Length(2))
            .collect();

        let gauge_areas = Layout::default()
            .direction(Direction::Vertical)
            .constraints(gauge_constraints)
            .split(detail_chunks[1]);

        for (i, (label, pct, reset_countdown)) in quota_widgets.iter().enumerate() {
            if i >= gauge_areas.len() {
                break;
            }
            let gauge_color = if *pct > 50.0 {
                Color::Green
            } else if *pct > 20.0 {
                Color::Yellow
            } else {
                Color::Red
            };

            let title_line = match (reset_countdown, *pct < 100.0) {
                (Some(ref cd), true) => format!("{} ({:.0}%) - hồi sau {}", label, pct, cd),
                _ => format!("{} ({:.0}%)", label, pct),
            };

            let gauge = Gauge::default()
                .block(Block::default().title(title_line))
                .gauge_style(Style::default().fg(gauge_color).bg(Color::DarkGray))
                .percent((*pct as u16).min(100));

            f.render_widget(gauge, gauge_areas[i]);
        }
    }

    if let Some(ref err) = acc.error {
        let err_paragraph = Paragraph::new(Line::from(vec![
            Span::styled("Ghi chú: ", Style::default().fg(Color::Yellow)),
            Span::raw(err),
        ]))
        .wrap(Wrap { trim: true });
        f.render_widget(err_paragraph, detail_chunks[2]);
    }
}

fn render_status(f: &mut Frame, area: Rect, app: &TuiApp) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(" Thông báo ");

    let (content, color) = if let Some((ref text, _, is_err)) = app.status_msg {
        let color = if is_err { Color::Red } else { Color::Green };
        (text.clone(), color)
    } else {
        ("Sẵn sàng".to_string(), Color::DarkGray)
    };

    let p = Paragraph::new(content)
        .style(Style::default().fg(color))
        .block(block);
    f.render_widget(p, area);
}

fn render_footer(f: &mut Frame, area: Rect) {
    let keys = vec![
        Span::styled("[Tab/1-3]", Style::default().fg(Color::Cyan)),
        Span::raw(" Agent | "),
        Span::styled("[↑/↓]", Style::default().fg(Color::Cyan)),
        Span::raw(" Chọn | "),
        Span::styled("[Enter/s]", Style::default().fg(Color::Cyan)),
        Span::raw(" Chuyển | "),
        Span::styled("[+/n]", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(" Thêm | "),
        Span::styled("[d]", Style::default().fg(Color::Cyan)),
        Span::raw(" Xóa | "),
        Span::styled("[r]", Style::default().fg(Color::Cyan)),
        Span::raw(" Làm mới | "),
        Span::styled("[p]", Style::default().fg(Color::Cyan)),
        Span::raw(" Ưu tiên | "),
        Span::styled("[a]", Style::default().fg(Color::Cyan)),
        Span::raw(" Tự chọn | "),
        Span::styled("[t]", Style::default().fg(Color::Cyan)),
        Span::raw(" Auto-select | "),
        Span::styled("[w]", Style::default().fg(Color::Cyan)),
        Span::raw(" Web | "),
        Span::styled("[q/Esc]", Style::default().fg(Color::Cyan)),
        Span::raw(" Thoát"),
    ];

    let footer = Paragraph::new(Line::from(keys)).alignment(Alignment::Center);
    f.render_widget(footer, area);
}

fn render_delete_modal(f: &mut Frame, acc: &UnifiedAccountDto) {
    let area = f.area();
    let popup_width = 50.min(area.width.saturating_sub(4));
    let popup_height = 8.min(area.height.saturating_sub(2));

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(Color::Red))
        .title(" Xác nhận xóa tài khoản ");

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::raw("Bạn có chắc chắn muốn xóa tài khoản "),
            Span::styled(&acc.email, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
            Span::raw("?"),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [y / Enter] ", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
            Span::raw("Xác nhận xóa   "),
            Span::styled(" [n / Esc] ", Style::default().fg(Color::DarkGray)),
            Span::raw("Hủy bỏ"),
        ]),
    ];

    let p = Paragraph::new(text)
        .alignment(Alignment::Center)
        .block(block);

    f.render_widget(p, popup_area);
}

fn render_add_menu_modal(f: &mut Frame, agent: Agent) {
    let area = f.area();
    let popup_width = 58.min(area.width.saturating_sub(4));
    let popup_height = 10.min(area.height.saturating_sub(2));

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" Thêm tài khoản mới ({}) ", agent.name()));

    let lines = match agent {
        Agent::Antigravity => vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(" [1 / o] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("Đăng nhập bằng Google OAuth (mở trình duyệt)"),
            ]),
            Line::from(vec![
                Span::styled(" [2 / m] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("Nhập thủ công bằng Access Token"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" [Esc]   ", Style::default().fg(Color::DarkGray)),
                Span::raw("Hủy bỏ"),
            ]),
        ],
        Agent::Codex => vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(" [1 / o] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("Đăng nhập bằng trình duyệt (Device OAuth)"),
            ]),
            Line::from(vec![
                Span::styled(" [2 / c] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("Nhập phiên đăng nhập từ CLI Codex trên máy"),
            ]),
            Line::from(vec![
                Span::styled(" [3 / m] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("Nhập token xác thực thủ công"),
            ]),
            Line::from(vec![
                Span::styled(" [Esc]   ", Style::default().fg(Color::DarkGray)),
                Span::raw("Hủy bỏ"),
            ]),
        ],
        Agent::Claude => vec![
            Line::from(""),
            Line::from(vec![
                Span::styled(" [1 / c] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("Nhập phiên đăng nhập từ CLI Claude Code trên máy"),
            ]),
            Line::from(vec![
                Span::styled(" [2 / m] ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::raw("Nhập session token / cookie thủ công"),
            ]),
            Line::from(""),
            Line::from(vec![
                Span::styled(" [Esc]   ", Style::default().fg(Color::DarkGray)),
                Span::raw("Hủy bỏ"),
            ]),
        ],
    };

    let p = Paragraph::new(lines).block(block);
    f.render_widget(p, popup_area);
}

fn render_codex_login_modal(f: &mut Frame, auth_url: Option<&str>, status: &str, message: &str) {
    let area = f.area();
    let popup_width = 64.min(area.width.saturating_sub(4));
    let popup_height = 10.min(area.height.saturating_sub(2));

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(Color::Green))
        .title(" Đang đăng nhập Codex ");

    let url_display = auth_url.unwrap_or("Đang mở trình duyệt...");

    let lines = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled("Trạng thái: ", Style::default().fg(Color::DarkGray)),
            Span::styled(status, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(format!(" ({})", message)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Liên kết xác thực: ", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(url_display, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [Esc / c] ", Style::default().fg(Color::Red)),
            Span::raw("Hủy bỏ phiên đăng nhập"),
        ]),
    ];

    let p = Paragraph::new(lines).block(block);
    f.render_widget(p, popup_area);
}

fn render_manual_input_modal(
    f: &mut Frame,
    agent: Agent,
    active_field: usize,
    email: &str,
    token: &str,
    error_msg: Option<&str>,
) {
    let area = f.area();
    let popup_width = 62.min(area.width.saturating_sub(4));
    let popup_height = 12.min(area.height.saturating_sub(2));

    let popup_area = Rect {
        x: (area.width.saturating_sub(popup_width)) / 2,
        y: (area.height.saturating_sub(popup_height)) / 2,
        width: popup_width,
        height: popup_height,
    };

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(Color::Cyan))
        .title(format!(" Nhập tài khoản thủ công ({}) ", agent.name()));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let form_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Email input
            Constraint::Length(3), // Token input
            Constraint::Length(2), // Error or status
            Constraint::Length(1), // Key guide
        ])
        .split(inner);

    // 1. Email box
    let email_style = if active_field == 0 {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let email_p = Paragraph::new(email).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(email_style)
            .title(" Email / Tên tài khoản "),
    );
    f.render_widget(email_p, form_layout[0]);

    // 2. Token box
    let token_style = if active_field == 1 {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let token_display = if token.len() > 50 {
        format!("{}... ({} ký tự)", &token[..40], token.len())
    } else {
        token.to_string()
    };
    let token_p = Paragraph::new(token_display).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(token_style)
            .title(" Access Token / Refresh Token "),
    );
    f.render_widget(token_p, form_layout[1]);

    // 3. Error message
    if let Some(err) = error_msg {
        let err_p = Paragraph::new(Line::from(vec![
            Span::styled("! ", Style::default().fg(Color::Red)),
            Span::styled(err, Style::default().fg(Color::Red)),
        ]));
        f.render_widget(err_p, form_layout[2]);
    }

    // 4. Instructions
    let guide = Paragraph::new(Line::from(vec![
        Span::styled("[Tab]", Style::default().fg(Color::Cyan)),
        Span::raw(" Đổi ô  "),
        Span::styled("[Enter]", Style::default().fg(Color::Cyan)),
        Span::raw(" Xác nhận thêm  "),
        Span::styled("[Esc]", Style::default().fg(Color::DarkGray)),
        Span::raw(" Hủy"),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(guide, form_layout[3]);
}
