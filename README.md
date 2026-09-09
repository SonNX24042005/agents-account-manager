# Agent account manager

<p align="left">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.80%2B-orange.svg" alt="Rust 1.80+"></a>
  <a href="#installation-and-usage"><img src="https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg" alt="Platform: Linux | macOS | Windows"></a>
  <a href="https://github.com/SonNX24042005/agents-account-manager"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" alt="PRs Welcome"></a>
</p>

[English](README.md) | [Tiếng Việt](README_VI.md)

An intelligent multi-account manager and quota coordinator designed for AI coding agents, including **Antigravity CLI (`agy`)**, **Antigravity IDE**, **Claude Code**, **Cursor**, and the broader AI coding ecosystem.

---

## Key features

- **Multi-agent ready**: Extensible architecture designed to manage accounts, refresh tokens, and coordinate quotas across multiple coding agents instead of relying on a single tool.
- **Sub-2ms 1-click account switching**: Automatically syncs credentials to OS Keyring (GNOME Keyring / Linux Secret Service) and Antigravity IDE SQLite storage (`state.vscdb`) in less than 2ms.
- **Zero environment pollution**: Runs 100% standalone without injecting environment variables or modifying shell configuration files (`.bashrc`, `.zshrc`), maintaining direct full-speed connections to model providers.
- **Real-time quota monitoring**: Tracks 5-hour and weekly rate limits across model families (Gemini, Claude, GPT) with precise reset countdowns.
- **Smart account auto-selection**: Dynamically evaluates and activates the account with the highest available quota whenever an agent performs a task.
- **Minimalist web dashboard**: Clean, distraction-free dark mode interface hosted locally at `http://127.0.0.1:8045`.

---

## Installation and usage

### 1. One-line quick install (no clone required)

Similar to tools like `rustup` or `claude code`, you can install and launch the service immediately on any machine with a single curl command:

```bash
curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/install.sh | bash
```

*(Alternatively, run `./install.sh` directly if you have already cloned the repository).*

This command downloads the appropriate pre-compiled binary, verifies its SHA256 checksum, completes installation, starts the service, and launches the dashboard. No master key entry is required during initial setup. Afterward, run `aam` at any time to open the dashboard with a secure session.

### 2. Service commands (`aam`)

- **Auto-start on boot (recommended):**
  ```bash
  aam autostart
  ```
  *The service starts automatically in the background on system boot, with auto-recovery that restarts the daemon within 3 seconds if stopped unexpectedly. To disable, run `aam stop` or `aam disable`.*

- **Start service:**
  ```bash
  aam start
  ```

- **Check service status:**
  ```bash
  aam status
  ```

- **Stop service:**
  ```bash
  aam stop
  ```

- **Update to latest version:**
  ```bash
  aam update
  ```

- **Restart service:**
  ```bash
  aam restart
  ```

Access the local web dashboard at: [http://127.0.0.1:8045](http://127.0.0.1:8045)

### 3. Transparent synchronization without shell aliases

While `aam` is running, the background daemon periodically scans available quotas and syncs the optimal credentials into your OS Keyring and agent configuration. You can use your coding agents normally **without defining shell aliases or writing complex wrapper scripts**.

---

## Managing Antigravity, Codex, and Claude

The web dashboard provides dedicated tabs for each agent. Each tab displays accounts, remaining quotas, and an automatic selection toggle. Toggle states are persisted in `agent-selection.json` within the service data directory. Upon upgrading, Antigravity retains its auto-selection setting, while Codex and Claude default to manual mode so you can import and review accounts first.

### Adding and switching accounts

- **Antigravity:** Sign in with Google OAuth or enter credentials directly in the Antigravity tab.
- **Codex:** Open the Codex tab and select **Add account**:
  - *Sign in via ChatGPT / Codex (recommended)*: Opens an OpenAI OAuth authorization link in your default browser. Once granted, credentials and quotas are saved and refreshed automatically.
  - *Import session from file/machine*: Reads existing credentials from `~/.codex/auth.json` or a chosen file. Respects `CODEX_HOME` (defaults to `~/.codex`). File-based switching requires `cli_auth_credentials_store = "file"`. Explicit `keyring`, `auto`, or `ephemeral` modes are rejected to avoid modifying system keychains unexpectedly.
- **Claude:** Authenticate using `claude auth login`, then import the session in the Claude tab. Reads `.credentials.json` from `CLAUDE_CONFIG_DIR` (defaults to `~/.claude`). File switching is currently supported on Linux and Windows.

Importing an existing session updates the entry in place without creating duplicates. Account names and emails serve display purposes only; active credentials remain stored securely. Accounts sharing an email across different agents are managed independently.

When auto-selection is enabled, the service selects the account with the highest available quota, keeping the active account in case of a tie. For Codex and Claude, scoring is based on the lowest percentage between short-term and weekly windows; for Antigravity, the selected model group quota is used. Stale (older than 10 minutes), unknown, or errored quotas are excluded from auto-selection.

After switching accounts, restart existing CLI or IDE sessions if they maintain credentials in memory.

### Quota monitoring and integration details

- **Codex**: Reads quotas through `account/rateLimits/read` via `codex app-server`. Each check runs in an isolated directory with a 25-second timeout, without creating conversations or dispatching prompts. Refreshed access tokens are saved automatically.
- **Claude**: Inspects OAuth usage endpoints utilized by Claude Code. The service enforces rate limiting with a minimum 5-minute polling interval and respects `Retry-After` headers up to 24 hours. Expired sessions require re-authentication through Claude Code before re-importing.
- **Antigravity**: Retains a 30-second refresh cycle. Codex and Claude checks run sequentially every 5 minutes per account without concurrent overlap.

Account data is stored at `native-accounts.json` with strict `0600` permissions on Unix. Before replacing session files, backup copies are created (`auth.aam.bak` or `.credentials.aam.bak`). The list API exposes only quota metadata and identifiers, never secrets or raw tokens.

---

## Project structure

```
├── agent-relay/                # Rust backend daemon source code
│   ├── src/
│   │   ├── cli.rs              # Global CLI command manager (aam)
│   │   ├── storage/            # Account storage, OS Keyring, and IDE SQLite sync
│   │   ├── proxy/              # Axum HTTP server, token management, quota router, UI
│   │   ├── oauth/              # Secure OAuth authorization flows
│   │   └── device/             # Hardware fingerprinting
│   ├── Cargo.lock
│   └── Cargo.toml
├── install.sh                  # One-line installation script
├── LICENSE                     # MIT License
├── README.md                   # English documentation
└── README_VI.md                # Vietnamese documentation
```

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
