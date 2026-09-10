pub fn get_admin_ui_html() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="vi" class="dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0, maximum-scale=5.0">
  <title>Quản lý tài khoản - Agent Account Manager</title>
  <style nonce="{{CSP_NONCE}}">
    {{ADMIN_CSS}}
    :root {
      --bg-page: #f8fafc;
      --bg-header: rgba(255, 255, 255, 0.88);
      --bg-card: #ffffff;
      --bg-card-subtle: #f8fafc;
      --bg-card-active: #eff6ff;
      --bg-well: #f1f5f9;
      --bg-input: #ffffff;
      --text-primary: #0f172a;
      --text-secondary: #475569;
      --text-muted: #64748b;
      --border-subtle: #e2e8f0;
      --border-strong: #cbd5e1;
      --border-active: rgba(59, 130, 246, 0.45);
      --card-shadow: 0 1px 3px rgba(0, 0, 0, 0.04), 0 1px 2px rgba(0, 0, 0, 0.02);
      --card-shadow-active: 0 4px 20px -2px rgba(59, 130, 246, 0.12);
      --card-shadow-hover: 0 6px 20px rgba(0, 0, 0, 0.06);
      --modal-bg: #ffffff;
      --modal-overlay: rgba(15, 23, 42, 0.55);
      --progress-track: #e2e8f0;
      --seg-bg: #f1f5f9;
      --seg-btn-active: #ffffff;
      --seg-btn-active-text: #0f172a;
      --seg-btn-text: #64748b;
      --seg-shadow: 0 1px 2px rgba(0, 0, 0, 0.08);
    }

    html.dark {
      --bg-page: #0b0c10;
      --bg-header: rgba(16, 18, 26, 0.88);
      --bg-card: rgba(24, 24, 27, 0.65);
      --bg-card-subtle: rgba(9, 9, 11, 0.6);
      --bg-card-active: #121626;
      --bg-well: rgba(9, 9, 11, 0.6);
      --bg-input: #09090b;
      --text-primary: #f4f4f5;
      --text-secondary: #a1a1aa;
      --text-muted: #71717a;
      --border-subtle: rgba(39, 39, 42, 0.8);
      --border-strong: #3f3f46;
      --border-active: rgba(59, 130, 246, 0.5);
      --card-shadow: 0 4px 20px -2px rgba(0, 0, 0, 0.4);
      --card-shadow-active: 0 0 24px rgba(59, 130, 246, 0.08);
      --card-shadow-hover: 0 8px 24px -4px rgba(0, 0, 0, 0.6);
      --modal-bg: #18181b;
      --modal-overlay: rgba(0, 0, 0, 0.75);
      --progress-track: rgb(24 24 27);
      --seg-bg: rgba(24, 24, 27, 0.9);
      --seg-btn-active: #27272a;
      --seg-btn-active-text: #f4f4f5;
      --seg-btn-text: #a1a1aa;
      --seg-shadow: 0 1px 3px rgba(0, 0, 0, 0.3);
    }

    body {
      font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
      background-color: var(--bg-page) !important;
      color: var(--text-primary);
      transition: background-color 0.2s ease, color 0.2s ease;
    }
    header {
      background-color: var(--bg-header) !important;
      border-color: var(--border-subtle) !important;
    }
    .font-mono { font-family: 'JetBrains Mono', ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace; }
    [data-lucide] { display: inline-flex; align-items: center; justify-content: center; font-style: normal; }
    [data-lucide="refresh-cw"]::before { content: '↻'; }
    [data-lucide="plus"]::before { content: '+'; }
    [data-lucide="chevron-down"]::before { content: '⌄'; }
    [data-lucide="globe"]::before { content: '◎'; }
    [data-lucide="key"]::before { content: '⚿'; }
    [data-lucide="x"]::before { content: '×'; }
    [data-lucide="trash-2"]::before { content: '⌫'; }
    [data-lucide="arrow-right-left"]::before { content: '⇄'; }
    [data-lucide="check"]::before { content: '✓'; }
    [data-lucide="user-x"]::before { content: '∅'; }

    .agent-tabs {
      display: flex;
      gap: 6px;
      margin-bottom: 20px;
      border-bottom: 1px solid var(--border-subtle);
      overflow-x: auto;
      -webkit-overflow-scrolling: touch;
      scrollbar-width: none;
    }
    .agent-tabs::-webkit-scrollbar { display: none; }
    .agent-tab {
      padding: 10px 18px;
      color: var(--text-muted);
      font-size: 13px;
      font-weight: 500;
      border-bottom: 2px solid transparent;
      border-top: none;
      border-left: none;
      border-right: none;
      background: transparent;
      cursor: pointer;
      transition: all 0.15s ease;
      white-space: nowrap;
    }
    .agent-tab:hover {
      color: var(--text-primary);
    }
    .agent-tab[aria-selected="true"] {
      color: #2563eb;
      border-color: #2563eb;
      font-weight: 600;
    }
    html.dark .agent-tab[aria-selected="true"] {
      color: #93c5fd;
      border-color: #60a5fa;
    }

    .agent-controls {
      display: flex;
      align-items: center;
      flex-wrap: wrap;
      gap: 12px;
      padding: 12px 16px;
      margin-bottom: 18px;
      background: var(--bg-card);
      border: 1px solid var(--border-subtle);
      border-radius: 12px;
      box-shadow: var(--card-shadow);
      transition: background-color 0.2s ease, border-color 0.2s ease;
    }
    .agent-controls input[type="checkbox"] {
      accent-color: #2563eb;
      width: 17px;
      height: 17px;
      cursor: pointer;
    }
    html.dark .agent-controls input[type="checkbox"] {
      accent-color: #60a5fa;
    }
    .agent-note {
      font-size: 12px;
      color: var(--text-muted);
      line-height: 1.6;
      margin-bottom: 18px;
    }
    .agent-note.agent-error-alert {
      color: #ef4444;
      background: rgba(239, 68, 68, 0.1);
      border: 1px solid rgba(239, 68, 68, 0.25);
      padding: 10px 14px;
      border-radius: 8px;
    }
    button:disabled { opacity: .5; cursor: not-allowed !important; }
    [hidden] { display: none !important; }

    .quota-progress {
      appearance: none;
      display: block;
      width: 100%;
      height: 0.35rem;
      overflow: hidden;
      border: 1px solid var(--border-subtle);
      border-radius: 9999px;
      background: var(--progress-track);
      transition: all 0.2s ease;
    }
    .quota-progress::-webkit-progress-bar { background: var(--progress-track); }
    .quota-progress::-webkit-progress-value { background: linear-gradient(to right, #2563eb, #38bdf8); border-radius: 9999px; }
    .quota-progress.low::-webkit-progress-value { background: #f59e0b; }
    .quota-progress.exhausted::-webkit-progress-value { background: #ef4444; }
    .quota-progress::-moz-progress-bar { background: linear-gradient(to right, #2563eb, #38bdf8); border-radius: 9999px; }
    .quota-progress.low::-moz-progress-bar { background: #f59e0b; }
    .quota-progress.exhausted::-moz-progress-bar { background: #ef4444; }

    .theme-segmented-group {
      display: inline-flex;
      align-items: center;
      padding: 3px;
      background: var(--seg-bg);
      border: 1px solid var(--border-subtle);
      border-radius: 9px;
      gap: 2px;
      transition: background-color 0.2s ease, border-color 0.2s ease;
    }
    .theme-seg-btn {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 5px;
      padding: 4px 8px;
      font-size: 11px;
      font-weight: 500;
      color: var(--seg-btn-text);
      border-radius: 6px;
      border: none;
      background: transparent;
      cursor: pointer;
      transition: all 0.15s ease;
      line-height: 1;
    }
    .theme-seg-btn:hover {
      color: var(--text-primary);
    }
    .theme-seg-btn.active {
      background: var(--seg-btn-active);
      color: var(--seg-btn-active-text);
      box-shadow: var(--seg-shadow);
      font-weight: 600;
    }

    .account-card {
      transition: transform 0.15s ease, box-shadow 0.15s ease, border-color 0.15s ease;
    }
    .account-card:hover {
      transform: translateY(-1px);
    }
    .stat-card {
      transition: transform 0.15s ease, box-shadow 0.15s ease, border-color 0.15s ease;
    }
    .stat-card:hover {
      transform: translateY(-1px);
    }

    #add-menu {
      width: 360px !important;
      max-width: calc(100vw - 2rem) !important;
    }
    .badge-pill {
      display: inline-flex;
      align-items: center;
      white-space: nowrap;
      flex-shrink: 0;
      font-size: 10px;
      font-weight: 500;
      line-height: 1;
      padding: 2.5px 7px;
      border-radius: 9999px;
      background-color: rgba(59, 130, 246, 0.15);
      color: #60a5fa;
      border: 1px solid rgba(59, 130, 246, 0.3);
    }
    .app-logo-badge {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      width: 32px;
      height: 32px;
      border-radius: 9px;
      background: linear-gradient(135deg, rgba(37, 99, 235, 0.16), rgba(59, 130, 246, 0.08));
      border: 1px solid rgba(59, 130, 246, 0.35);
      color: #60a5fa;
      transition: all 0.2s ease;
    }
    .app-logo-badge:hover {
      border-color: rgba(59, 130, 246, 0.55);
      box-shadow: 0 0 12px rgba(59, 130, 246, 0.2);
    }
    .app-logo-badge svg {
      width: 18px;
      height: 18px;
    }

    /* Light theme styling adaptations */
    html:not(.dark) body {
      background-color: var(--bg-page) !important;
      color: var(--text-secondary) !important;
    }
    html:not(.dark) header {
      background-color: var(--bg-header) !important;
      border-color: var(--border-subtle) !important;
    }
    html:not(.dark) .text-zinc-100 { color: #0f172a !important; }
    html:not(.dark) .text-zinc-200 { color: #1e293b !important; }
    html:not(.dark) .text-zinc-300 { color: #334155 !important; }
    html:not(.dark) .text-zinc-400 { color: #64748b !important; }
    html:not(.dark) .text-zinc-500 { color: #94a3b8 !important; }

    html:not(.dark) .bg-zinc-900,
    html:not(.dark) .bg-zinc-900\/50,
    html:not(.dark) .bg-zinc-900\/60,
    html:not(.dark) .bg-zinc-900\/90 {
      background-color: var(--bg-card) !important;
      border-color: var(--border-subtle) !important;
    }
    html:not(.dark) .bg-zinc-950,
    html:not(.dark) .bg-zinc-950\/60,
    html:not(.dark) .bg-zinc-950\/80 {
      background-color: var(--bg-card-subtle) !important;
    }
    html:not(.dark) .bg-zinc-800 {
      background-color: var(--bg-well) !important;
    }
    html:not(.dark) .border-zinc-800,
    html:not(.dark) .border-zinc-800\/60,
    html:not(.dark) .border-zinc-800\/80 {
      border-color: var(--border-subtle) !important;
    }
    html:not(.dark) .border-zinc-700,
    html:not(.dark) .border-zinc-700\/60 {
      border-color: var(--border-strong) !important;
    }
    html:not(.dark) .hover\:bg-zinc-800:hover,
    html:not(.dark) .hover\:bg-zinc-800\/80:hover {
      background-color: #f1f5f9 !important;
    }
    html:not(.dark) .hover\:bg-zinc-700:hover {
      background-color: #e2e8f0 !important;
    }
    html:not(.dark) .hover\:border-zinc-700:hover {
      border-color: #cbd5e1 !important;
    }
    html:not(.dark) .hover\:text-zinc-200:hover {
      color: #0f172a !important;
    }
    html:not(.dark) .group:hover .group-hover\:text-zinc-200,
    html:not(.dark) .group:hover .group-hover\:text-white {
      color: #0f172a !important;
    }
    html:not(.dark) .group:hover .group-hover\:bg-zinc-700 {
      background-color: #e2e8f0 !important;
    }

    html:not(.dark) .bg-\[\#121626\] {
      background-color: var(--bg-card-active) !important;
      border-color: rgba(59, 130, 246, 0.4) !important;
      box-shadow: 0 4px 20px -2px rgba(59, 130, 246, 0.12) !important;
    }
    html:not(.dark) .bg-blue-950\/80 {
      background-color: #dbeafe !important;
      color: #1d4ed8 !important;
      border-color: rgba(59, 130, 246, 0.3) !important;
    }
    html:not(.dark) .text-blue-300,
    html:not(.dark) .text-blue-400 {
      color: #2563eb !important;
    }
    html:not(.dark) .bg-blue-400 {
      background-color: #2563eb !important;
    }
    html:not(.dark) .bg-blue-500\/10,
    html:not(.dark) .bg-blue-600\/10 {
      background-color: rgba(37, 99, 235, 0.08) !important;
    }
    html:not(.dark) .border-blue-500\/20,
    html:not(.dark) .border-blue-500\/30,
    html:not(.dark) .border-blue-500\/40 {
      border-color: rgba(37, 99, 235, 0.25) !important;
    }

    html:not(.dark) input:not([type="checkbox"]):not([type="file"]),
    html:not(.dark) select,
    html:not(.dark) textarea {
      background-color: var(--bg-input) !important;
      border-color: var(--border-strong) !important;
      color: var(--text-primary) !important;
    }
    html:not(.dark) input:focus,
    html:not(.dark) select:focus,
    html:not(.dark) textarea:focus {
      border-color: #3b82f6 !important;
      box-shadow: 0 0 0 3px rgba(59, 130, 246, 0.15) !important;
    }

    html:not(.dark) #native-modal > div,
    html:not(.dark) #codex-login-modal > div,
    html:not(.dark) #add-modal > div {
      background-color: var(--modal-bg) !important;
      border-color: var(--border-subtle) !important;
      box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.1), 0 8px 10px -6px rgba(0, 0, 0, 0.1) !important;
    }
    html:not(.dark) #native-modal,
    html:not(.dark) #codex-login-modal,
    html:not(.dark) #add-modal {
      background-color: var(--modal-overlay) !important;
    }
    html:not(.dark) #native-cancel,
    html:not(.dark) #codex-login-cancel-btn,
    html:not(.dark) #cancel-add-btn {
      background-color: #f1f5f9 !important;
      color: #334155 !important;
      border: 1px solid #cbd5e1;
    }
    html:not(.dark) #native-cancel:hover,
    html:not(.dark) #codex-login-cancel-btn:hover,
    html:not(.dark) #cancel-add-btn:hover {
      background-color: #e2e8f0 !important;
      color: #0f172a !important;
    }

    html:not(.dark) #add-menu {
      background-color: #ffffff !important;
      border-color: var(--border-subtle) !important;
      box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.1), 0 8px 10px -6px rgba(0, 0, 0, 0.05) !important;
    }
    html:not(.dark) .badge-pill {
      background-color: #eff6ff !important;
      color: #2563eb !important;
      border-color: #bfdbfe !important;
    }
    html:not(.dark) .app-logo-badge {
      background: linear-gradient(135deg, #eff6ff, #dbeafe) !important;
      border-color: #bfdbfe !important;
      color: #2563eb !important;
      box-shadow: 0 1px 3px rgba(37, 99, 235, 0.08) !important;
    }
    html:not(.dark) .app-logo-badge .logo-back-card {
      stroke: #3b82f6 !important;
      stroke-opacity: 0.5 !important;
    }
    html:not(.dark) .switch-account-btn {
      background-color: #f1f5f9 !important;
      color: #1e293b !important;
      border-color: #cbd5e1 !important;
    }
    html:not(.dark) .switch-account-btn:hover:not(:disabled) {
      background-color: #e2e8f0 !important;
      color: #0f172a !important;
      border-color: #94a3b8 !important;
    }
    html:not(.dark) .bg-zinc-900\/20 {
      background-color: #f1f5f9 !important;
      border-color: #cbd5e1 !important;
    }
  </style>
</head>
<body class="bg-[#0b0c10] text-zinc-200 min-h-screen antialiased selection:bg-blue-600/30">
  <!-- Header -->
  <header class="border-b border-zinc-800/80 bg-[#10121a]/80 backdrop-blur sticky top-0 z-40">
    <div class="max-w-6xl mx-auto px-4 sm:px-6 h-14 flex items-center justify-between gap-2 sm:gap-3">
      <div class="flex items-center gap-2.5 sm:gap-3 min-w-0">
        <div class="app-logo-badge flex-shrink-0" title="Agent Account Manager">
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
            <path d="M7 4h11.5A2.5 2.5 0 0 1 21 6.5V16" stroke-opacity="0.6" class="logo-back-card"/>
            <rect x="3" y="7" width="14" height="13" rx="2.8"/>
            <path d="M6.8 13.5a3.2 3.2 0 0 1 3.2-3.2 3.5 3.5 0 0 1 2.4 1l1.1 1.1"/>
            <path d="M13.5 10v2.4h-2.4"/>
            <path d="M13.2 13.5a3.2 3.2 0 0 1-3.2 3.2 3.5 3.5 0 0 1-2.4-1l-1.1-1.1"/>
            <path d="M6.5 17v-2.4h2.4"/>
          </svg>
        </div>
        <div class="min-w-0">
          <h1 class="text-sm font-semibold text-zinc-100 truncate">Quản lý tài khoản</h1>
        </div>
      </div>
      <div class="flex items-center gap-2 sm:gap-3 flex-shrink-0">
        <div class="hidden md:flex items-center gap-2 px-2.5 py-1 rounded-md text-xs font-mono bg-zinc-900/90 border border-zinc-800 text-zinc-300">
          <span class="w-1.5 h-1.5 rounded-full bg-blue-500"></span>
          <span id="relay-address">Đang kết nối...</span>
        </div>
        <div id="theme-selector" class="theme-segmented-group" role="radiogroup" aria-label="Giao diện">
          <button type="button" id="theme-btn-system" class="theme-seg-btn active" data-theme-val="system" title="Theo hệ thống" aria-checked="true">
            <svg class="w-3.5 h-3.5 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <rect x="2" y="3" width="20" height="14" rx="2"/>
              <line x1="8" y1="21" x2="16" y2="21"/>
              <line x1="12" y1="17" x2="12" y2="21"/>
            </svg>
            <span class="theme-label hidden sm:inline">Hệ thống</span>
          </button>
          <button type="button" id="theme-btn-light" class="theme-seg-btn" data-theme-val="light" title="Giao diện sáng" aria-checked="false">
            <svg class="w-3.5 h-3.5 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="12" cy="12" r="4"/>
              <path d="M12 2v2 M12 20v2 M4.93 4.93l1.41 1.41 M17.66 17.66l1.41 1.41 M2 12h2 M20 12h2 M6.34 17.66l-1.41 1.41 M19.07 4.93l-1.41 1.41"/>
            </svg>
            <span class="theme-label hidden sm:inline">Sáng</span>
          </button>
          <button type="button" id="theme-btn-dark" class="theme-seg-btn" data-theme-val="dark" title="Giao diện tối" aria-checked="false">
            <svg class="w-3.5 h-3.5 flex-shrink-0" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 3a6 6 0 0 0 9 9 9 9 0 1 1-9-9Z"/>
            </svg>
            <span class="theme-label hidden sm:inline">Tối</span>
          </button>
        </div>
        <button id="refresh-btn" title="Làm mới" class="p-1.5 hover:bg-zinc-800 rounded-md text-zinc-400 hover:text-zinc-200 transition flex items-center justify-center">
          <i data-lucide="refresh-cw" class="w-4 h-4 text-zinc-400"></i>
        </button>
      </div>
    </div>
  </header>

  <!-- Main Content -->
  <main class="max-w-6xl mx-auto px-4 sm:px-6 py-6 sm:py-8">
    <nav class="agent-tabs" role="tablist" aria-label="Agent">
      <button id="tab-antigravity" class="agent-tab" role="tab" aria-selected="true" aria-controls="agent-panel" data-agent="antigravity">Antigravity</button>
      <button id="tab-codex" class="agent-tab" role="tab" aria-selected="false" aria-controls="agent-panel" tabindex="-1" data-agent="codex">Codex</button>
      <button id="tab-claude" class="agent-tab" role="tab" aria-selected="false" aria-controls="agent-panel" tabindex="-1" data-agent="claude">Claude</button>
    </nav>
    <section id="agent-panel" role="tabpanel" aria-labelledby="tab-antigravity">
    <div class="agent-controls">
      <label class="flex items-center gap-2.5 text-xs cursor-pointer select-none" for="auto-select-toggle">
        <input type="checkbox" id="auto-select-toggle" role="switch" disabled>
        <span class="font-medium text-zinc-200">Tự động chọn tài khoản có nhiều quota nhất</span>
      </label>
      <span id="auto-select-status" class="text-xs text-zinc-400">Đang tải cấu hình...</span>
    </div>
    <p id="agent-note" class="agent-note"></p>
    <p id="agent-error" role="alert" class="agent-note agent-error-alert" hidden></p>
    <!-- Action & summary bar -->
    <div class="flex flex-col sm:flex-row sm:items-center justify-between gap-4 mb-6">
      <div>
        <h2 class="text-base font-semibold text-zinc-100">Danh sách tài khoản</h2>
        <p class="text-xs text-zinc-400 mt-0.5">Quản lý tài khoản và hạn ngạch riêng cho từng agent</p>
      </div>

      <!-- Add Account Dropdown Menu -->
      <div class="relative">
        <button id="add-account-btn" class="px-3.5 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold shadow-sm transition flex items-center gap-1.5 cursor-pointer flex-shrink-0">
          <i data-lucide="plus" class="w-3.5 h-3.5"></i>
          Thêm tài khoản
          <i data-lucide="chevron-down" class="w-3.5 h-3.5 ml-0.5 text-blue-200"></i>
        </button>

        <!-- Dropdown Menu -->
        <div id="add-menu" class="absolute right-0 mt-2 bg-zinc-900 border border-zinc-800 rounded-xl shadow-2xl p-1.5 z-50 hidden">
          <div class="px-2.5 py-1.5 text-[10px] font-medium text-zinc-500 tracking-wider">
            Chọn phương thức thêm
          </div>
          
          <!-- Option 1: Browser OAuth (Google / Codex / Claude) -->
          <button id="oauth-option-btn" class="w-full text-left p-2.5 rounded-lg hover:bg-zinc-800/80 transition flex items-start gap-3 group cursor-pointer">
            <div class="w-7 h-7 rounded-md bg-blue-500/10 border border-blue-500/20 text-blue-400 flex items-center justify-center flex-shrink-0 mt-0.5 group-hover:bg-blue-500 group-hover:text-white transition">
              <i data-lucide="globe" class="w-4 h-4"></i>
            </div>
            <div class="flex-1 min-w-0">
              <div class="flex items-center gap-2">
                <span id="oauth-option-title" class="text-xs font-semibold text-zinc-200 group-hover:text-white">Đăng nhập Google</span>
                <span id="oauth-option-badge" class="badge-pill">Khuyên dùng</span>
              </div>
              <div id="oauth-option-desc" class="text-[10px] text-zinc-400 mt-1 leading-snug">
                Tự động xác thực qua trình duyệt và tự động làm mới token
              </div>
            </div>
          </button>

          <!-- Option 2: Direct Token / Manual File Import -->
          <button id="direct-option-btn" class="w-full text-left p-2.5 rounded-lg hover:bg-zinc-800/80 transition flex items-start gap-3 group cursor-pointer">
            <div class="w-7 h-7 rounded-md bg-zinc-800 border border-zinc-700/60 text-zinc-400 flex items-center justify-center flex-shrink-0 mt-0.5 group-hover:bg-zinc-700 group-hover:text-zinc-200 transition">
              <i data-lucide="key" class="w-4 h-4"></i>
            </div>
            <div class="flex-1 min-w-0">
              <div id="direct-option-title" class="text-xs font-semibold text-zinc-200 group-hover:text-white">
                Nhập token thủ công
              </div>
              <div id="direct-option-desc" class="text-[10px] text-zinc-400 mt-1 leading-snug">
                Dán access token hoặc refresh token trực tiếp
              </div>
            </div>
          </button>
        </div>
      </div>
    </div>

    <!-- Quick Stats & Intelligent Routing -->
    <div class="grid grid-cols-1 sm:grid-cols-3 gap-3 mb-6">
      <div class="stat-card p-3.5 sm:p-4 rounded-xl bg-zinc-900/50 border border-zinc-800/80 flex flex-col justify-between">
        <span class="text-xs text-zinc-400">Tổng số tài khoản</span>
        <div class="text-xl font-semibold text-zinc-100 mt-1" id="stat-total">0</div>
      </div>

      <div class="stat-card p-3.5 sm:p-4 rounded-xl bg-zinc-900/50 border border-zinc-800/80 flex flex-col justify-between">
        <span class="text-xs text-zinc-400">Tài khoản đang dùng</span>
        <div class="text-xl font-semibold text-blue-400 mt-1 truncate" id="stat-active">Chưa có</div>
      </div>

      <!-- Model routing preference -->
      <div id="antigravity-preference" class="stat-card p-3.5 sm:p-4 rounded-xl bg-zinc-900/50 border border-zinc-800/80 flex flex-col justify-between">
        <div class="flex items-center justify-between gap-2">
          <span class="text-xs text-zinc-400">Chế độ tự động chọn</span>
          <select id="pref-select" class="bg-zinc-950 border border-zinc-800 text-[11px] text-zinc-200 rounded-md px-2 py-0.5 focus:outline-none focus:border-blue-500">
            <option value="auto">Tự động (theo mô hình vừa dùng)</option>
            <option value="gemini">Luôn ưu tiên Gemini</option>
            <option value="claude_gpt">Luôn ưu tiên Claude & GPT</option>
          </select>
        </div>
        <div class="mt-2 flex items-center justify-between">
          <div class="flex items-center gap-1.5 text-xs font-medium text-zinc-200">
            <span class="w-1.5 h-1.5 rounded-full bg-blue-500"></span>
            <span id="detected-model-label">Đang phát hiện...</span>
          </div>
          <span id="detected-source-tag" class="text-[10px] text-zinc-500 truncate max-w-[150px]" title=""></span>
        </div>
      </div>
    </div>

    <!-- Accounts Grid -->
    <div id="accounts-grid" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
      <div class="col-span-full py-12 text-center text-zinc-500 text-sm">
        Đang tải dữ liệu...
      </div>
    </div>
    </section>
  </main>

  <div id="native-modal" role="dialog" aria-modal="true" aria-labelledby="native-title" class="fixed inset-0 bg-zinc-950/80 backdrop-blur-sm z-50 flex items-center justify-center p-4" hidden>
    <div class="bg-zinc-900 border border-zinc-800 rounded-2xl p-5 sm:p-6 w-full max-w-md shadow-2xl max-h-[90vh] overflow-y-auto">
      <h3 id="native-title" class="text-sm font-semibold text-zinc-100">Thêm tài khoản</h3>
      <p id="native-help" class="agent-note"></p>
      <label for="native-email" class="block text-xs text-zinc-400 mb-1">Email hoặc tên tài khoản</label>
      <input id="native-email" maxlength="254" autocomplete="off" class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm sm:text-xs text-zinc-200">
      <label for="native-file" class="block text-xs text-zinc-400 mt-3 mb-1">Tệp đăng nhập JSON (tùy chọn)</label>
      <input id="native-file" type="file" accept=".json,application/json" class="text-xs text-zinc-400 w-full">
      <p class="agent-note">Để trống tệp để nhập phiên đăng nhập hiện tại trên máy chạy dịch vụ. Lặp lại sau khi đăng nhập tài khoản khác để thêm vào danh sách.</p>
      <p id="native-error" role="alert" class="text-xs text-zinc-400"></p>
      <div class="flex items-center justify-between mt-5">
        <button id="native-cancel" class="px-3 py-1.5 rounded-lg bg-zinc-800 text-xs cursor-pointer">Đóng</button>
        <button id="native-save" class="px-3 py-1.5 rounded-lg bg-blue-600 text-white text-xs cursor-pointer">Nhập tài khoản</button>
      </div>
    </div>
  </div>

  <!-- Modal: Codex browser login -->
  <div id="codex-login-modal" role="dialog" aria-modal="true" aria-labelledby="codex-login-title" class="fixed inset-0 bg-zinc-950/80 backdrop-blur-sm z-50 flex items-center justify-center p-4" hidden>
    <div class="bg-zinc-900 border border-zinc-800 rounded-2xl p-5 sm:p-6 w-full max-w-md shadow-2xl max-h-[90vh] overflow-y-auto">
      <div class="flex items-center justify-between mb-3">
        <h3 id="codex-login-title" class="text-sm font-semibold text-zinc-100">Đăng nhập Codex qua trình duyệt</h3>
        <button id="codex-login-close-btn" class="text-zinc-400 hover:text-zinc-200 cursor-pointer">
          <i data-lucide="x" class="w-4 h-4"></i>
        </button>
      </div>

      <div class="space-y-3.5 my-4">
        <p class="text-xs text-zinc-300 leading-relaxed">
          Trang xác thực OpenAI / ChatGPT sẽ mở trong trình duyệt của bạn để bạn chọn tài khoản và cấp quyền.
        </p>

        <div id="codex-login-link-container" class="p-3 bg-zinc-950 border border-zinc-800 rounded-lg text-xs" hidden>
          <span class="text-zinc-400 block mb-1">Nếu trình duyệt không tự mở, hãy bấm liên kết bên dưới:</span>
          <a id="codex-login-link" href="javascript:void(0)" target="_blank" rel="noopener noreferrer" class="text-blue-400 hover:underline break-all font-mono text-[11px] inline-flex items-center gap-1">
            <span>Mở trang đăng nhập OpenAI</span>
            <span class="text-xs">↗</span>
          </a>
        </div>

        <div id="codex-login-status-box" class="flex items-center gap-3 p-3 bg-blue-950/20 border border-blue-800/30 rounded-lg text-xs text-blue-300">
          <span id="codex-login-spinner" class="inline-block w-4 h-4 border-2 border-blue-400 border-t-transparent rounded-full animate-spin"></span>
          <span id="codex-login-status-text">Đang tạo liên kết đăng nhập...</span>
        </div>

        <p id="codex-login-error" role="alert" class="text-xs text-red-400 bg-red-950/30 border border-red-800/40 p-2.5 rounded-lg" hidden></p>
      </div>

      <div class="flex items-center justify-end gap-2 mt-5 pt-3 border-t border-zinc-800">
        <button id="codex-login-cancel-btn" class="px-3.5 py-1.5 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-xs font-medium text-zinc-300 transition cursor-pointer">
          Hủy
        </button>
      </div>
    </div>
  </div>

  <!-- Modal: Add direct token -->
  <div id="add-modal" class="fixed inset-0 bg-zinc-950/80 backdrop-blur-sm z-50 flex items-center justify-center hidden p-4">
    <div class="bg-zinc-900 border border-zinc-800 rounded-2xl p-5 sm:p-6 w-full max-w-md shadow-2xl max-h-[90vh] overflow-y-auto">
      <div class="flex items-center justify-between mb-4">
        <div>
          <h3 class="text-sm font-semibold text-zinc-100">Nhập token thủ công</h3>
          <p class="text-[11px] text-zinc-400 mt-0.5">Dán thông tin token của tài khoản Google</p>
        </div>
        <button id="close-add-modal-btn" class="text-zinc-400 hover:text-zinc-200">
          <i data-lucide="x" class="w-4 h-4"></i>
        </button>
      </div>
      <div class="space-y-3.5">
        <div>
          <label class="block text-xs font-medium text-zinc-400 mb-1">Email Google</label>
          <input id="input-email" type="email" placeholder="user@gmail.com" class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm sm:text-xs focus:outline-none focus:border-blue-500 text-zinc-200">
        </div>
        <div>
          <label class="block text-xs font-medium text-zinc-400 mb-1">Access token</label>
          <textarea id="input-access-token" rows="3" placeholder="ya29.a0..." class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm sm:text-xs focus:outline-none focus:border-blue-500 font-mono text-zinc-200"></textarea>
        </div>
        <div>
          <label class="block text-xs font-medium text-zinc-400 mb-1">Refresh token (tùy chọn)</label>
          <input id="input-refresh-token" type="text" placeholder="1//04..." class="w-full bg-zinc-950 border border-zinc-800 rounded-lg px-3 py-2 text-sm sm:text-xs focus:outline-none focus:border-blue-500 font-mono text-zinc-200">
        </div>
      </div>
      <div class="flex items-center justify-between mt-5 pt-3 border-t border-zinc-800">
        <button id="modal-oauth-btn" class="text-xs text-blue-400 hover:text-blue-300 transition cursor-pointer">
          Hoặc đăng nhập Google
        </button>
        <div class="flex items-center gap-2">
          <button id="cancel-add-btn" class="px-3 py-1.5 rounded-lg bg-zinc-800 hover:bg-zinc-700 text-xs font-medium text-zinc-300 cursor-pointer">Đóng</button>
          <button id="save-add-btn" class="px-3.5 py-1.5 rounded-lg bg-blue-600 hover:bg-blue-500 text-white text-xs font-semibold shadow-sm transition cursor-pointer">Lưu tài khoản</button>
        </div>
      </div>
    </div>
  </div>

  <script nonce="{{CSP_NONCE}}">
    let SESSION_READY = false;
    let selectedAgent = 'antigravity';
    let selectionFlags = null;
    let accountsRequest = 0;
    let nativeModalAgent = null;
    let settingsBusy = false;
    let settingsRequest = 0;
    const agentNames = { antigravity: 'Antigravity', codex: 'Codex', claude: 'Claude' };

    // Theme Management
    const THEME_KEY = 'aam_theme_preference';

    function getStoredTheme() {
      try {
        if (typeof localStorage !== 'undefined') {
          return localStorage.getItem(THEME_KEY) || 'system';
        }
      } catch (e) {}
      return 'system';
    }

    function setStoredTheme(theme) {
      try {
        if (typeof localStorage !== 'undefined') {
          localStorage.setItem(THEME_KEY, theme);
        }
      } catch (e) {}
    }

    function isSystemDark() {
      if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
        try {
          return window.matchMedia('(prefers-color-scheme: dark)').matches;
        } catch (e) {}
      }
      return true;
    }

    function applyTheme(pref) {
      const isDark = pref === 'dark' || (pref === 'system' && isSystemDark());
      const root = document.documentElement;
      if (root) {
        if (isDark) {
          root.classList.add('dark');
          root.classList.remove('light');
        } else {
          root.classList.remove('dark');
          root.classList.add('light');
        }
      }

      ['system', 'light', 'dark'].forEach(t => {
        const btn = document.getElementById('theme-btn-' + t);
        if (btn) {
          if (t === pref) {
            btn.classList.add('active');
            btn.setAttribute('aria-checked', 'true');
          } else {
            btn.classList.remove('active');
            btn.setAttribute('aria-checked', 'false');
          }
        }
      });
    }

    function initTheme() {
      const pref = getStoredTheme();
      applyTheme(pref);

      if (typeof window !== 'undefined' && typeof window.matchMedia === 'function') {
        try {
          const media = window.matchMedia('(prefers-color-scheme: dark)');
          if (media && typeof media.addEventListener === 'function') {
            media.addEventListener('change', () => {
              if (getStoredTheme() === 'system') {
                applyTheme('system');
              }
            });
          }
        } catch (e) {}
      }

      ['system', 'light', 'dark'].forEach(t => {
        const btn = document.getElementById('theme-btn-' + t);
        if (btn) {
          btn.addEventListener('click', () => {
            setStoredTheme(t);
            applyTheme(t);
          });
        }
      });
    }

    initTheme();

    function showError(message) {
      const node = document.getElementById('agent-error');
      node.textContent = message || '';
      node.hidden = !message;
    }

    function updateSelectionControl() {
      const toggle = document.getElementById('auto-select-toggle');
      toggle.disabled = !selectionFlags || settingsBusy;
      toggle.checked = !!selectionFlags?.[selectedAgent];
      document.getElementById('auto-select-status').textContent = !selectionFlags ? 'Đang tải cấu hình...' :
        (toggle.checked ? 'Đang bật · tắt để chọn thủ công' : 'Đang tắt · giữ tài khoản bạn chọn');
    }

    async function fetchSelectionSettings() {
      if (settingsBusy) return;
      const requestId = ++settingsRequest;
      try {
        const response = await apiFetch('/api/agents/settings');
        if (!response.ok) throw new Error('Không tải được cấu hình tự động chọn');
        const flags = await response.json();
        if (!settingsBusy && requestId === settingsRequest) { selectionFlags = flags; updateSelectionControl(); }
      } catch (e) { showError(e.message); }
    }

    async function changeAutoSelection() {
      const agent = selectedAgent;
      const enabled = document.getElementById('auto-select-toggle').checked;
      settingsBusy = true;
      settingsRequest++;
      updateSelectionControl();
      try {
        const response = await apiFetch('/api/agents/settings', {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ agent, enabled })
        });
        const data = await response.json();
        if (!response.ok) throw new Error(data.error || 'Không lưu được cấu hình');
        selectionFlags = { ...selectionFlags, [agent]: enabled };
        showError('');
      } catch (e) { showError(e.message); }
      finally { settingsBusy = false; updateSelectionControl(); fetchAccounts(); }
    }

    function selectAgent(agent) {
      selectedAgent = agent;
      document.querySelectorAll('[data-agent]').forEach(tab => {
        tab.setAttribute('aria-selected', String(tab.dataset.agent === agent));
        tab.tabIndex = tab.dataset.agent === agent ? 0 : -1;
      });
      document.getElementById('agent-panel').setAttribute('aria-labelledby', 'tab-' + agent);
      document.getElementById('antigravity-preference').hidden = agent !== 'antigravity';
      document.getElementById('add-menu').classList.add('hidden');
      document.getElementById('add-modal').classList.add('hidden');
      document.getElementById('agent-note').textContent = agent === 'antigravity'
        ? 'Quota Google/Antigravity được chọn theo nhóm mô hình bên dưới.'
        : (agent === 'codex'
          ? 'Đăng nhập trực tiếp qua trình duyệt (OpenAI OAuth) hoặc nhập phiên từ tệp/CLI. Quota được đọc tự động qua Codex CLI.'
          : 'Đăng nhập bằng Claude Code rồi nhập tài khoản. Hỗ trợ tệp đăng nhập trên Linux và Windows. Quota được kiểm tra mỗi 5 phút; phiên hết hạn cần đăng nhập và nhập lại.');
      if (agent !== 'antigravity') document.getElementById('agent-note').textContent += ' Sau khi chuyển tài khoản, mở lại phiên agent để dùng đăng nhập mới. Quota chưa có hoặc quá cũ không được tự chọn.';
      document.getElementById('accounts-grid').textContent = 'Đang tải dữ liệu...';
      document.getElementById('stat-total').textContent = '—';
      document.getElementById('stat-active').textContent = '—';
      showError('');
      updateSelectionControl();
      updateAddMenuForAgent(agent);
      fetchAccounts();
      if (agent === 'antigravity') fetchPreference();
    }

    function updateAddMenuForAgent(agent) {
      const oauthTitle = document.getElementById('oauth-option-title');
      const oauthDesc = document.getElementById('oauth-option-desc');
      const oauthBadge = document.getElementById('oauth-option-badge');
      const directBtn = document.getElementById('direct-option-btn');
      const directTitle = document.getElementById('direct-option-title');
      const directDesc = document.getElementById('direct-option-desc');

      if (!oauthTitle) return;

      if (agent === 'antigravity') {
        oauthTitle.textContent = 'Đăng nhập Google';
        oauthBadge.hidden = false;
        oauthDesc.textContent = 'Tự động xác thực qua trình duyệt và tự động làm mới token';
        directBtn.hidden = false;
        directTitle.textContent = 'Nhập token thủ công';
        directDesc.textContent = 'Dán access token hoặc refresh token trực tiếp';
      } else if (agent === 'codex') {
        oauthTitle.textContent = 'Đăng nhập ChatGPT / Codex';
        oauthBadge.hidden = false;
        oauthDesc.textContent = 'Mở liên kết trong trình duyệt để đăng nhập, tự động lưu tài khoản';
        directBtn.hidden = false;
        directTitle.textContent = 'Nhập phiên từ tệp / máy';
        directDesc.textContent = 'Nhập phiên hiện tại trong ~/.codex hoặc chọn tệp auth.json';
      } else if (agent === 'claude') {
        oauthTitle.textContent = 'Nhập phiên Claude Code';
        oauthBadge.hidden = true;
        oauthDesc.textContent = 'Đăng nhập bằng claude auth login rồi nhập phiên từ máy hoặc tệp JSON';
        directBtn.hidden = true;
      }
    }

    let codexLoginPollTimer = null;
    let activeCodexLoginId = null;

    async function startCodexBrowserLogin() {
      document.getElementById('add-menu').classList.add('hidden');
      const modal = document.getElementById('codex-login-modal');
      const linkContainer = document.getElementById('codex-login-link-container');
      const link = document.getElementById('codex-login-link');
      const statusBox = document.getElementById('codex-login-status-box');
      const spinner = document.getElementById('codex-login-spinner');
      const statusText = document.getElementById('codex-login-status-text');
      const errorText = document.getElementById('codex-login-error');
      const cancelBtn = document.getElementById('codex-login-cancel-btn');

      modal.hidden = false;
      linkContainer.hidden = true;
      statusBox.className = 'flex items-center gap-3 p-3 bg-blue-950/20 border border-blue-800/30 rounded-lg text-xs text-blue-300';
      spinner.hidden = false;
      errorText.hidden = true;
      statusText.textContent = 'Đang khởi tạo phiên đăng nhập Codex...';
      cancelBtn.textContent = 'Hủy';
      cancelBtn.disabled = false;

      try {
        const res = await apiFetch('/api/agents/codex/login', { method: 'POST' });
        const data = await res.json();
        if (!res.ok) {
          throw new Error(data.error || 'Không thể bắt đầu đăng nhập Codex');
        }

        activeCodexLoginId = data.id;
        if (data.auth_url) {
          link.href = data.auth_url;
          linkContainer.hidden = false;
          try {
            window.open(data.auth_url, '_blank');
          } catch (e) {
            console.warn('Popup blocked, link shown in modal', e);
          }
        }

        statusText.textContent = 'Đang chờ bạn hoàn tất đăng nhập trong trình duyệt...';
        pollCodexLoginStatus();
      } catch (err) {
        spinner.hidden = true;
        errorText.textContent = err.message;
        errorText.hidden = false;
        cancelBtn.textContent = 'Đóng';
      }
    }

    function pollCodexLoginStatus() {
      if (codexLoginPollTimer) clearInterval(codexLoginPollTimer);
      codexLoginPollTimer = setInterval(async () => {
        try {
          const res = await apiFetch('/api/agents/codex/login');
          if (!res.ok) return;
          const data = await res.json();
          if (!data) return;

          const statusBox = document.getElementById('codex-login-status-box');
          const spinner = document.getElementById('codex-login-spinner');
          const statusText = document.getElementById('codex-login-status-text');
          const errorText = document.getElementById('codex-login-error');
          const cancelBtn = document.getElementById('codex-login-cancel-btn');

          if (data.status === 'completed') {
            clearInterval(codexLoginPollTimer);
            codexLoginPollTimer = null;
            activeCodexLoginId = null;
            statusBox.className = 'flex items-center gap-3 p-3 bg-green-950/20 border border-green-800/30 rounded-lg text-xs text-green-300';
            spinner.hidden = true;
            statusText.textContent = '✓ ' + (data.message || 'Đăng nhập thành công!');
            cancelBtn.textContent = 'Xong';
            fetchAccounts();
            setTimeout(() => {
              closeCodexLoginModal();
            }, 1800);
          } else if (data.status === 'failed') {
            clearInterval(codexLoginPollTimer);
            codexLoginPollTimer = null;
            statusBox.className = 'flex items-center gap-3 p-3 bg-red-950/20 border border-red-800/30 rounded-lg text-xs text-red-300';
            spinner.hidden = true;
            statusText.textContent = 'Đăng nhập thất bại';
            errorText.textContent = data.message || 'Không thể hoàn tất đăng nhập';
            errorText.hidden = false;
            cancelBtn.textContent = 'Đóng';
          } else if (data.status === 'cancelled') {
            clearInterval(codexLoginPollTimer);
            codexLoginPollTimer = null;
            closeCodexLoginModal();
          }
        } catch (e) {
          console.error('Polling codex login status error:', e);
        }
      }, 1500);
    }

    async function cancelCodexLogin() {
      if (codexLoginPollTimer) {
        clearInterval(codexLoginPollTimer);
        codexLoginPollTimer = null;
      }
      if (activeCodexLoginId) {
        const idToCancel = activeCodexLoginId;
        activeCodexLoginId = null;
        try {
          await apiFetch('/api/agents/codex/login/cancel', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ id: idToCancel })
          });
        } catch (e) {
          console.warn('Failed to cancel codex login:', e);
        }
      }
      closeCodexLoginModal();
    }

    function closeCodexLoginModal() {
      if (codexLoginPollTimer) {
        clearInterval(codexLoginPollTimer);
        codexLoginPollTimer = null;
      }
      activeCodexLoginId = null;
      document.getElementById('codex-login-modal').hidden = true;
      document.getElementById('add-account-btn').focus();
    }

    function openNativeModal() {
      nativeModalAgent = selectedAgent;
      document.getElementById('native-title').textContent = 'Nhập tài khoản ' + agentNames[nativeModalAgent];
      document.getElementById('native-help').textContent = nativeModalAgent === 'codex'
        ? 'Nhập phiên hiện tại từ ~/.codex/auth.json hoặc chọn tệp auth.json.'
        : 'Đăng nhập bằng claude auth login rồi nhập từ ~/.claude/.credentials.json hoặc chọn tệp .credentials.json.';
      document.getElementById('native-error').textContent = '';
      document.getElementById('native-modal').hidden = false;
      document.getElementById('native-email').focus();
    }

    function closeNativeModal() {
      document.getElementById('native-modal').hidden = true;
      document.getElementById('native-email').value = '';
      document.getElementById('native-file').value = '';
      document.getElementById('add-account-btn').focus();
    }

    async function importNativeAccount() {
      const button = document.getElementById('native-save');
      button.disabled = true;
      try {
        const file = document.getElementById('native-file').files[0];
        if (file && file.size > 256 * 1024) throw new Error('Tệp đăng nhập tối đa 256 KiB');
        const email = document.getElementById('native-email').value.trim();
        if (!email) throw new Error('Vui lòng nhập email hoặc tên tài khoản');
        const credentials = file ? JSON.parse(await file.text()) : null;
        const response = await apiFetch('/api/agents/import', {
          method: 'POST', headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ agent: nativeModalAgent, email, credentials })
        });
        const data = await response.json();
        if (!response.ok) throw new Error(data.error || 'Không nhập được tài khoản');
        closeNativeModal();
        fetchAccounts();
      } catch (e) { document.getElementById('native-error').textContent = e.message; }
      finally { button.disabled = false; }
    }

    async function apiFetch(url, options) {
      const request = options ? { ...options } : {};
      request.credentials = 'same-origin';
      const response = await window.fetch(url, request);
      if (response.status === 401) {
        SESSION_READY = false;
        showSessionHelp();
        throw new Error('Phiên quản trị đã hết hạn. Hãy chạy lại lệnh aam.');
      }
      return response;
    }

    async function establishBrowserSession() {
      const fragment = new URLSearchParams(window.location.hash.slice(1));
      const bootstrapToken = fragment.get('bootstrap');
      if (bootstrapToken) {
        window.history.replaceState(null, '', window.location.pathname + window.location.search);
        const exchange = await window.fetch('/api/session/exchange', {
          method: 'POST',
          credentials: 'same-origin',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ bootstrap_token: bootstrapToken })
        });
        if (!exchange.ok) {
          return false;
        }
      }

      const health = await window.fetch('/api/health', { credentials: 'same-origin' });
      return health.ok;
    }

    function showSessionHelp() {
      document.getElementById('accounts-grid').innerHTML = `
        <div class="col-span-full py-16 text-center rounded-xl border border-dashed border-zinc-800 bg-zinc-900/20">
          <p class="text-zinc-300 text-sm font-medium">Cần mở phiên quản trị an toàn</p>
          <p class="text-xs text-zinc-500 mt-1">Chạy <code class="font-mono text-blue-400">aam</code> trong terminal để mở lại tự động.</p>
        </div>
      `;
    }

    function escapeHtml(value) {
      return String(value ?? '').replace(/[&<>"']/g, (character) => ({
        '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
      })[character]);
    }

    function toggleAddMenu() {
      const menu = document.getElementById('add-menu');
      menu.classList.toggle('hidden');
    }

    function handleOptionOAuth() {
      document.getElementById('add-menu').classList.add('hidden');
      if (selectedAgent === 'antigravity') {
        startOAuthLogin();
      } else if (selectedAgent === 'codex') {
        startCodexBrowserLogin();
      } else if (selectedAgent === 'claude') {
        openNativeModal();
      }
    }

    function handleOptionDirectToken() {
      document.getElementById('add-menu').classList.add('hidden');
      if (selectedAgent === 'antigravity') {
        openDirectAddModal();
      } else {
        openNativeModal();
      }
    }

    // Close dropdown on outside click
    document.addEventListener('click', (e) => {
      const btn = document.getElementById('add-account-btn');
      const menu = document.getElementById('add-menu');
      if (btn && menu && !btn.contains(e.target) && !menu.contains(e.target)) {
        menu.classList.add('hidden');
      }
    });

    async function fetchPreference() {
      if (selectedAgent !== 'antigravity') return;
      try {
        const res = await apiFetch('/api/preference');
        const data = await res.json();
        const select = document.getElementById('pref-select');
        if (select) {
          select.value = data.preference;
        }

        const label = document.getElementById('detected-model-label');
        if (label) {
          if (data.preference === 'gemini') {
            label.innerText = 'Ưu tiên Gemini';
          } else if (data.preference === 'claude_gpt') {
            label.innerText = 'Ưu tiên Claude & GPT';
          } else {
            label.innerText = data.detected_category === 'claude_gpt' ? 'Đang dùng: Claude & GPT' : 'Đang dùng: Gemini Models';
          }
        }

        const tag = document.getElementById('detected-source-tag');
        if (tag && data.last_detected_source) {
          tag.innerText = data.last_detected_source;
          tag.title = data.last_detected_source;
        }
      } catch (e) {
        console.error('Failed to fetch preference:', e);
      }
    }

    async function changePreference(prefVal) {
      try {
        const res = await apiFetch('/api/preference', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ preference: prefVal })
        });
        if (res.ok) {
          fetchPreference();
          fetchAccounts();
        }
      } catch (e) {
        alert('Lỗi cập nhật cấu hình: ' + e);
      }
    }

    async function fetchAccounts() {
      const agent = selectedAgent;
      const requestId = ++accountsRequest;
      try {
        const res = await apiFetch(agent === 'antigravity' ? '/api/accounts' : '/api/agents/accounts?agent=' + agent);
        const accounts = await res.json();
        if (agent !== selectedAgent || requestId !== accountsRequest) return;
        if (!res.ok) throw new Error(accounts.error || 'Không tải được tài khoản');
        renderAccounts(accounts);
      } catch (err) {
        if (agent === selectedAgent && requestId === accountsRequest) showError(err.message);
      }
    }

    function renderAccounts(accounts) {
      document.getElementById('stat-total').innerText = accounts.length;
      const activeAcc = accounts.find(a => a.is_active);
      document.getElementById('stat-active').innerText = activeAcc ? activeAcc.email : 'Chưa chọn';

      const container = document.getElementById('accounts-grid');
      if (accounts.length === 0) {
        container.innerHTML = `
          <div class="col-span-full py-16 text-center rounded-xl border border-dashed border-zinc-800 bg-zinc-900/20">
            <i data-lucide="user-x" class="w-8 h-8 text-zinc-600 mx-auto mb-2"></i>
            <p class="text-zinc-400 text-sm font-medium">Chưa có tài khoản nào</p>
            <p class="text-xs text-zinc-500 mt-0.5">Bấm nút "Thêm tài khoản" ở trên để bắt đầu thêm tài khoản.</p>
          </div>
        `;
        return;
      }

      container.innerHTML = accounts.map(acc => {
        const isActive = acc.is_active;
        const nativeQuotaUnavailable = selectedAgent !== 'antigravity' && (acc.quota_stale || acc.quota_percentage == null);
        const quotaStatusLabel = selectedAgent !== 'antigravity' && ({
          ready: 'Sẵn sàng', exhausted: 'Hết hạn ngạch', stale: 'Chờ quota mới',
          unknown: 'Chưa có quota', error: 'Lỗi đọc quota'
        })[acc.quota_status];
        const safeEmail = escapeHtml(acc.email);
        const safeId = escapeHtml(acc.id);
        const safeInitials = escapeHtml(acc.email.substring(0, 2).toUpperCase());

        return `
          <div class="account-card p-4 rounded-xl transition flex flex-col justify-between ${
            isActive
              ? 'bg-[#121626] border border-blue-500/50 shadow-[0_0_24px_rgba(59,130,246,0.06)]'
              : 'bg-zinc-900/60 border border-zinc-800/80 hover:border-zinc-700'
          }">
            <div>
              <div class="flex items-start justify-between gap-2 mb-3.5">
                <div class="flex items-center gap-2.5 min-w-0">
                  <div class="w-8 h-8 rounded-full flex items-center justify-center text-xs font-semibold flex-shrink-0 ${
                    isActive
                      ? 'bg-blue-950/80 text-blue-300 border border-blue-500/40'
                      : 'bg-zinc-800 text-zinc-300 border border-zinc-700/60'
                  }">
                    ${safeInitials}
                  </div>
                  <div class="min-w-0">
                    <h3 class="font-medium text-xs text-zinc-100 truncate" title="${safeEmail}">${safeEmail}</h3>
                    <p class="text-[10px] text-zinc-500 font-mono mt-0.5 truncate">id: ${escapeHtml(acc.id.substring(0, 8))}</p>
                  </div>
                </div>
                <div class="flex items-center gap-2 flex-shrink-0">
                  ${
                    isActive
                      ? `<span class="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full text-[10px] font-medium bg-blue-500/10 text-blue-400 border border-blue-500/20">
                          <span class="w-1.5 h-1.5 rounded-full bg-blue-400"></span>
                          Đang dùng
                        </span>`
                      : `<span class="inline-flex items-center gap-1.5 text-[10px] font-medium text-zinc-400 px-1 py-0.5">
                          <span class="w-1.5 h-1.5 rounded-full bg-zinc-600"></span>
                          ${quotaStatusLabel || (nativeQuotaUnavailable ? 'Cần kiểm tra quota' : 'Sẵn sàng')}
                        </span>`
                  }
                  <button data-account-id="${safeId}" data-account-email="${safeEmail}" title="Xóa tài khoản" class="delete-account-btn p-1.5 hover:bg-zinc-800 text-zinc-500 hover:text-red-400 rounded-md transition flex items-center justify-center cursor-pointer">
                    <i data-lucide="trash-2" class="w-3.5 h-3.5"></i>
                  </button>
                </div>
              </div>

              ${acc.error ? `<p class="text-xs text-zinc-400">${escapeHtml(acc.error)}</p>` : ''}
              ${acc.quota_message ? `<p class="text-xs text-zinc-400">${escapeHtml(acc.quota_message)}</p>` : ''}
              ${acc.checked_at ? `<p class="text-[10px] text-zinc-500">Cập nhật: ${escapeHtml(new Date(acc.checked_at).toLocaleString('vi-VN'))}</p>` : ''}
              <!-- Quota breakdown -->
              <div class="space-y-2.5 mt-3 pt-3 border-t border-zinc-800/60">
                ${(acc.quota_groups && acc.quota_groups.length > 0) ? acc.quota_groups.map(g => `
                  <div class="space-y-1.5">
                    <div class="text-[11px] font-medium text-zinc-300 flex items-center gap-1">
                      <span>${escapeHtml(g.name)}</span>
                    </div>
                    <div class="space-y-1.5 bg-zinc-950/60 p-2 rounded-lg border border-zinc-800/60">
                      ${g.buckets.map(b => {
                        let pct = Number.isFinite(b.remaining_percentage)
                          ? Math.max(0, Math.min(100, Math.round(b.remaining_percentage)))
                          : 0;
                        const resetInfo = getResetDisplay(b.reset_time, b.window);
                        const stale = resetInfo.isExpired || (selectedAgent !== 'antigravity' && acc.quota_stale);

                        const winUpper = String(b.window || '').toUpperCase();
                        const isWeekly = winUpper.includes('WEEK') || winUpper.includes('7D') || winUpper.includes('SEVEN');
                        const is5h = winUpper.includes('5H') || winUpper.includes('FIVE');
                        const winTitle = is5h ? 'Hạn ngạch 5 giờ' : (isWeekly ? 'Hạn ngạch tuần' : escapeHtml(b.window));
                        const isExhausted = pct === 0 && !stale;
                        const isLow = pct < 20;
                        return `
                        <div>
                          <div class="flex justify-between text-[10px] text-zinc-400 mb-0.5">
                            <span class="${isWeekly && isExhausted ? 'text-red-400 font-medium' : ''}">
                              ${winTitle}${isWeekly && isExhausted ? ' (Hết hạn ngạch tuần)' : ''}
                            </span>
                            <span class="font-mono ${stale ? 'text-zinc-500' : (isExhausted ? 'text-red-400' : (isLow ? 'text-amber-400' : 'text-blue-400'))} font-medium">${pct}%${stale ? ' (cũ)' : ''}</span>
                          </div>
                          <progress class="quota-progress ${isExhausted ? 'exhausted' : (isLow ? 'low' : '')}" max="100" value="${pct}">${pct}%</progress>
                          ${resetInfo.text ? `
                            <div class="text-[9px] text-zinc-500 mt-0.5 font-mono">
                              ${escapeHtml(resetInfo.isExpired ? 'Đã đến giờ đặt lại · chờ quota mới' : resetInfo.text)}
                            </div>
                          ` : ''}
                        </div>
                      `}).join('')}
                    </div>
                  </div>
                `).join('') : (selectedAgent !== 'antigravity' || acc.quota_percentage == null) ? '<p class="text-xs text-zinc-400">Chưa có dữ liệu quota</p>' : `
                  <div class="bg-zinc-950/60 p-2 rounded-lg border border-zinc-800/60">
                    <div class="flex justify-between text-[10px] text-zinc-400 mb-0.5">
                      <span>Hạn ngạch khả dụng</span>
                      <span class="font-mono font-medium text-blue-400">${Number.isFinite(acc.quota_percentage) ? Math.max(0, Math.min(100, Math.round(acc.quota_percentage))) : 0}%</span>
                    </div>
                    <progress class="quota-progress" max="100" value="${Number.isFinite(acc.quota_percentage) ? Math.max(0, Math.min(100, Math.round(acc.quota_percentage))) : 0}"></progress>
                  </div>
                `}
              </div>
            </div>

            <!-- Action Button -->
            <div class="mt-4 pt-3 border-t border-zinc-800/60">
              ${isActive ? `
                <div class="w-full py-1.5 px-2 text-center text-xs font-medium rounded-lg bg-blue-500/10 text-blue-400 border border-blue-500/20 flex items-center justify-center gap-1.5">
                  <i data-lucide="check" class="w-3.5 h-3.5"></i> Đang hoạt động
                </div>
              ` : `
                <button data-account-id="${safeId}" ${!selectionFlags || selectionFlags[selectedAgent] ? 'disabled title="Tắt tự động chọn để chuyển thủ công"' : ''} class="switch-account-btn w-full py-1.5 px-2 text-center text-xs font-medium rounded-lg bg-zinc-800 hover:bg-zinc-700 text-zinc-200 border border-zinc-700 transition duration-150 flex items-center justify-center gap-1.5 cursor-pointer">
                  <i data-lucide="arrow-right-left" class="w-3.5 h-3.5 text-zinc-400"></i> Chuyển sang tài khoản này
                </button>
              `}
            </div>
          </div>
        `;
      }).join('');
    }

    async function switchAccount(accountId) {
      try {
        const res = await apiFetch(selectedAgent === 'antigravity' ? '/api/accounts/switch' : '/api/agents/switch', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ account_id: accountId, agent: selectedAgent })
        });
        const data = await res.json();
        if (res.ok) {
          fetchAccounts();
        } else {
          alert('Lỗi: ' + data.error);
        }
      } catch (e) {
        alert('Lỗi kết nối: ' + e.message);
      }
    }

    async function deleteAccount(accountId, email) {
      if (!confirm(`Bạn có chắc chắn muốn xóa tài khoản "${email}" không?`)) {
        return;
      }
      try {
        const res = await apiFetch(selectedAgent === 'antigravity' ? '/api/accounts/delete' : '/api/agents/delete', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ account_id: accountId, agent: selectedAgent })
        });
        const data = await res.json();
        if (res.ok) {
          fetchAccounts();
        } else {
          alert('Lỗi: ' + (data.error || 'Không thể xóa tài khoản'));
        }
      } catch (e) {
        alert('Lỗi kết nối: ' + e.message);
      }
    }

    function getResetDisplay(resetTimeStr, window) {
      const winUpper = String(window || '').toUpperCase();
      const is5h = winUpper.includes('5H') || winUpper.includes('FIVE');
      const isWeekly = winUpper.includes('WEEK') || winUpper.includes('7D') || winUpper.includes('SEVEN');

      if (!resetTimeStr) {
        return { text: is5h ? 'Chu kỳ: 5 giờ (đầy đủ)' : (isWeekly ? 'Chu kỳ: 7 ngày (đầy đủ)' : ''), isExpired: false };
      }
      try {
        const d = new Date(resetTimeStr);
        const now = new Date();
        const diffMs = d - now;
        
        const hours = d.getHours().toString().padStart(2, '0');
        const mins = d.getMinutes().toString().padStart(2, '0');
        const day = d.getDate().toString().padStart(2, '0');
        const month = (d.getMonth() + 1).toString().padStart(2, '0');

        if (diffMs <= 0) {
          return {
            text: 'Đã đến giờ đặt lại · chờ quota mới',
            isExpired: true
          };
        }

        const diffMinutesTotal = Math.floor(diffMs / (1000 * 60));
        const diffHours = Math.floor(diffMinutesTotal / 60);
        const diffMins = diffMinutesTotal % 60;
        const diffDays = Math.floor(diffHours / 24);

        let timeStr = '';
        if (diffDays > 0) {
          timeStr = `Reset: ${hours}:${mins} (${day}/${month}, còn ${diffDays}d ${diffHours % 24}h)`;
        } else if (diffHours > 0) {
          timeStr = `Reset: ${hours}:${mins} (còn ${diffHours}h ${diffMins}m)`;
        } else {
          timeStr = `Reset: ${hours}:${mins} (còn ${diffMins}m)`;
        }

        return { text: timeStr, isExpired: false };
      } catch (e) {
        return { text: '', isExpired: false };
      }
    }

    async function startOAuthLogin() {
      try {
        const res = await apiFetch('/api/accounts/oauth/start');
        const data = await res.json();
        if (data.auth_url) {
          window.open(data.auth_url, '_blank');
        }
      } catch (err) {
        alert('Lỗi khi lấy URL OAuth: ' + err);
      }
    }

    function openDirectAddModal() {
      document.getElementById('add-modal').classList.remove('hidden');
    }

    function closeDirectAddModal() {
      document.getElementById('add-modal').classList.add('hidden');
    }

    async function submitDirectAdd() {
      const email = document.getElementById('input-email').value;
      const access_token = document.getElementById('input-access-token').value;
      const refresh_token = document.getElementById('input-refresh-token').value;

      if (!email || !access_token) {
        alert('Vui lòng nhập Email và Access token');
        return;
      }

      try {
        const res = await apiFetch('/api/accounts/add', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ email, access_token, refresh_token })
        });
        if (res.ok) {
          closeDirectAddModal();
          fetchAccounts();
        } else {
          alert('Lỗi khi thêm tài khoản');
        }
      } catch (err) {
        alert('Lỗi: ' + err);
      }
    }

    document.getElementById('refresh-btn').addEventListener('click', async () => {
      try {
        const response = await apiFetch('/api/agents/refresh', {
          method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ agent: selectedAgent })
        });
        if (!response.ok) throw new Error('Không làm mới được quota');
        showError('Đã yêu cầu làm mới. Agent có thời gian chờ giữa các lần đọc quota.');
        fetchAccounts();
        fetchPreference();
      } catch (e) { showError(e.message); }
    });
    document.getElementById('auto-select-toggle').addEventListener('change', changeAutoSelection);
    document.getElementById('native-cancel').addEventListener('click', closeNativeModal);
    document.getElementById('native-save').addEventListener('click', importNativeAccount);
    document.querySelectorAll('[data-agent]').forEach(tab => tab.addEventListener('click', () => selectAgent(tab.dataset.agent)));
    document.querySelector('.agent-tabs').addEventListener('keydown', event => {
      const tabs = Array.from(document.querySelectorAll('[data-agent]'));
      let index = tabs.indexOf(document.activeElement);
      if (index < 0 || !['ArrowLeft', 'ArrowRight', 'Home', 'End'].includes(event.key)) return;
      event.preventDefault();
      index = event.key === 'Home' ? 0 : event.key === 'End' ? tabs.length - 1 : (index + (event.key === 'ArrowRight' ? 1 : -1) + tabs.length) % tabs.length;
      tabs[index].focus(); tabs[index].click();
    });
    document.addEventListener('keydown', event => {
      if (event.key === 'Escape') { closeNativeModal(); closeDirectAddModal(); cancelCodexLogin(); }
    });
    document.getElementById('add-account-btn').addEventListener('click', toggleAddMenu);
    document.getElementById('oauth-option-btn').addEventListener('click', handleOptionOAuth);
    document.getElementById('direct-option-btn').addEventListener('click', handleOptionDirectToken);
    document.getElementById('codex-login-close-btn').addEventListener('click', cancelCodexLogin);
    document.getElementById('codex-login-cancel-btn').addEventListener('click', cancelCodexLogin);
    document.getElementById('pref-select').addEventListener('change', (event) => changePreference(event.target.value));
    document.getElementById('close-add-modal-btn').addEventListener('click', closeDirectAddModal);
    document.getElementById('cancel-add-btn').addEventListener('click', closeDirectAddModal);
    document.getElementById('save-add-btn').addEventListener('click', submitDirectAdd);
    document.getElementById('modal-oauth-btn').addEventListener('click', () => {
      closeDirectAddModal();
      startOAuthLogin();
    });
    document.getElementById('accounts-grid').addEventListener('click', (event) => {
      const deleteButton = event.target.closest('.delete-account-btn');
      if (deleteButton) {
        deleteAccount(deleteButton.dataset.accountId, deleteButton.dataset.accountEmail);
        return;
      }
      const switchButton = event.target.closest('.switch-account-btn');
      if (switchButton) {
        switchAccount(switchButton.dataset.accountId);
      }
    });

    document.getElementById('relay-address').innerText = window.location.host;

    async function initializeDashboard() {
      try {
        SESSION_READY = await establishBrowserSession();
      } catch (error) {
        console.error('Failed to establish browser session:', error);
        SESSION_READY = false;
      }
      if (!SESSION_READY) {
        showSessionHelp();
        return;
      }
      await fetchSelectionSettings();
      selectAgent(selectedAgent);
    }

    initializeDashboard();
    setInterval(() => {
      if (SESSION_READY) {
        fetchSelectionSettings();
        fetchAccounts();
        fetchPreference();
      }
    }, 5000);
  </script>
</body>
</html>
"#
}

#[cfg(test)]
mod tests {
    use super::get_admin_ui_html;

    #[test]
    fn admin_ui_contains_no_embedded_secret_or_remote_script() {
        let html = get_admin_ui_html();
        assert!(!html.contains("MASTER_KEY"));
        assert!(!html.contains("<script src="));
        assert!(!html.contains("onclick="));
        assert!(!html.contains("onchange="));
        assert!(!html.contains("sessionStorage"));
        assert!(!html.contains("window.prompt"));
        assert!(html.contains("/api/session/exchange"));
    }
}
