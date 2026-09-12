# Agent account manager

<p align="left">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.80%2B-orange.svg" alt="Rust 1.80+"></a>
  <a href="#installation-and-usage"><img src="https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg" alt="Platform: Linux | macOS | Windows"></a>
  <a href="https://github.com/SonNX24042005/agents-account-manager"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" alt="PRs Welcome"></a>
</p>

[English](README.md) | [Tiếng Việt](README_VI.md)

An intelligent multi-account manager and quota coordinator for AI coding agents, including **Antigravity**, **Claude Code**, and **Codex**.

---

## Key features

- **Multi-agent support**: Manage multiple accounts and quotas for Antigravity, Claude Code, and Codex in one unified place.
- **Interactive terminal UI (TUI)**: Monitor status, view real-time quota progress gauges, and trigger actions with keyboard shortcuts in terminal via `aam tui`.
- **Fast CLI commands & flags**: Manage, list, switch, refresh, filter accounts, and configure routing preferences directly from terminal commands (`aam list`, `aam switch`, `aam preference`...).
- **One-click account switching**: Switch active accounts instantly from the web dashboard, TUI, or CLI without manual credential copying.
- **Smart quota auto-selection**: Automatically detect the active model and switch to the account with the highest remaining quota when limits are approached.
- **Real-time quota monitoring**: Keep track of remaining requests, rate limits, and reset schedules across accounts.
- **Zero environment pollution**: Runs cleanly in the background without modifying shell profiles (`.bashrc`, `.zshrc`) or creating invasive aliases.
- **Local web dashboard**: Clean, responsive dark-mode interface hosted locally at `http://127.0.0.1:8045`.

---

## Installation and usage

### 1. One-line quick install

Install and launch the service immediately with a single command:

- **Linux / macOS:**
  ```bash
  curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/install.sh | bash
  ```
  *(Alternatively, run `./scripts/install.sh` directly if you have already cloned the repository).*

- **Windows (PowerShell):**
  ```powershell
  irm https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/install.ps1 | iex
  ```
  *(Alternatively, run `.\scripts\install.ps1` directly if cloned).*

The script automatically downloads the release binary for your platform, verifies integrity via SHA-256 checksum, installs the `aam` command, starts the background service, and opens the web dashboard.

### 2. Service commands (`aam`)

- **Open dashboard (default):**
  ```bash
  aam
  ```

- **Auto-start on boot (recommended):**
  ```bash
  aam autostart
  ```
  *Starts the service in the background on system boot and auto-recovers if stopped. To disable, run `aam disable`.*

- **Start service:**
  ```bash
  aam start
  ```

- **Check status:**
  ```bash
  aam status
  ```

- **Stop service:**
  ```bash
  aam stop
  ```

- **Restart service:**
  ```bash
  aam restart
  ```

- **Update to latest version:**
  ```bash
  aam update
  ```

- **Reinstall binary:**
  ```bash
  aam reinstall
  ```

- **Uninstall:**
  ```bash
  # Uninstall binary and service, keeping user data and accounts:
  aam uninstall

  # Uninstall and purge all configuration and accounts:
  aam uninstall --purge
  ```

### 3. Visual interface and system overview

- **Account list (default when running `aam`):**
  ```bash
  aam                         # List accounts across all agents (equivalent to 'aam list')
  # Or with filters:
  aam list --active           # Show only active accounts
  ```

- **System overview check:**
  ```bash
  aam check                   # Quickly check daemon status, active accounts, and quotas across all agents
  # Or alias:
  aam overview
  ```

- **Open web dashboard:**
  ```bash
  aam web                     # Open web dashboard in default browser (or 'aam open')
  ```

- **Interactive terminal UI (`aam tui`):**
  ```bash
  aam tui
  ```

  Key keyboard shortcuts in TUI:
  - `Tab` or `1`, `2`, `3`: Switch between agents (Antigravity, Codex, Claude).
  - `↑` / `↓` or `j` / `k`: Navigate through the account list.
  - `Enter` or `s`: Switch to the selected account (syncs instantly to OS Keyring and IDE database).
  - `+` or `n`: Open the add account menu (Google OAuth for Antigravity, Device OAuth for Codex, CLI import for Claude, or manual input).
  - `r`: Refresh quota data immediately.
  - `p`: Cycle routing preference (Auto -> Gemini -> Claude & GPT).
  - `a`: Auto-select the account with highest available quota.
  - `t`: Toggle auto-selection setting for the active agent.
  - `d`: Delete the selected account (with confirmation dialog `y`/`n`).
  - `w`: Open the local web dashboard in your default browser.
  - `q` or `Esc`: Exit TUI.

### 4. Agent-scoped commands (`aam <agent> <subcommand>`)

Clean, modular command syntax scoped to individual agents (`agy`, `codex`, `claude`):

- **Antigravity account management (`aam agy`):**
  ```bash
  aam agy                     # Quickly inspect Antigravity accounts
  aam agy list                # List accounts with options (--active, --json)
  aam agy switch 1            # Switch to Antigravity account #1
  aam agy login               # Google OAuth browser login
  aam agy add --email <email> --access-token <token>  # Manual account addition
  aam agy preference auto     # Set model routing preference (auto | gemini | claude_gpt)
  aam agy auto-select         # Auto-select highest quota account
  aam agy settings --enable   # Enable auto-selection for Antigravity
  aam agy refresh             # Refresh Antigravity quota
  aam agy reset               # Reset cooldowns
  aam agy delete 1            # Delete account #1
  ```

- **OpenAI Codex account management (`aam codex`):**
  ```bash
  aam codex                   # Quickly inspect Codex accounts
  aam codex list              # List accounts with options (--active, --json)
  aam codex switch 1          # Switch to Codex account #1
  aam codex login             # Browser Device OAuth login with terminal spinner
  aam codex import            # Import active credentials from local Codex CLI
  aam codex add --email <email> --token <token>  # Manual account addition
  aam codex auto-select       # Auto-select highest quota Codex account
  aam codex settings --enable # Enable auto-selection for Codex
  aam codex refresh           # Refresh Codex quota
  aam codex delete 1          # Delete account #1
  ```

- **Claude Code account management (`aam claude`):**
  ```bash
  aam claude                  # Quickly inspect Claude accounts
  aam claude list             # List accounts with options (--active, --json)
  aam claude switch 1         # Switch to Claude account #1
  aam claude import           # Import active credentials from local Claude Code CLI
  aam claude add --email <email> --token <token>  # Manual session token addition
  aam claude auto-select      # Auto-select highest quota Claude account
  aam claude refresh          # Refresh Claude quota
  aam claude delete 1         # Delete account #1
  ```

- **General commands (across all agents):**
  ```bash
  aam list                    # List all accounts across all agents
  aam switch 1                # Switch account globally
  aam refresh                 # Refresh quotas across all agents
  aam auto-select             # Trigger auto-selection
  aam settings                # View auto-selection settings for all agents
  ```

### 5. Quick uninstallation scripts

If you prefer to uninstall without using the CLI command:

- **Linux / macOS:**
  ```bash
  curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/uninstall.sh | bash
  # Or purge all account data:
  curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/uninstall.sh | bash -s -- --purge
  ```

- **Windows (PowerShell):**
  ```powershell
  irm https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/uninstall.ps1 | iex
  # Or purge all account data:
  .\scripts\uninstall.ps1 -Purge
  ```

Local web dashboard URL: [http://127.0.0.1:8045](http://127.0.0.1:8045)

---

## Managing accounts

The web dashboard provides dedicated sections for each supported agent:

- **Antigravity**: Sign in via Google OAuth or paste authorization credentials directly to manage Gemini and Claude model quotas.
- **Codex**: Sign in via browser OAuth or import an active local session file to monitor rate limits.
- **Claude**: Import your active Claude Code session to track usage and switch accounts.

### Auto-selection vs. manual selection

- **Auto-selection enabled**: The service continuously tracks quota levels and automatically activates the account with the most available capacity.
- **Manual mode**: Select any account manually from the dashboard whenever you want full control over which account is active.

After switching an account, existing CLI or IDE sessions pick up the active credentials on their next command or when restarted.

---

## Project structure

```
├── agent-relay/                # Rust backend daemon source code
│   ├── src/
│   │   ├── cli.rs              # Global CLI command manager (aam)
│   │   ├── client.rs           # Local daemon REST API client
│   │   ├── tui.rs              # Full-screen interactive terminal UI (Ratatui)
│   │   ├── storage/            # Local account storage and credential synchronization
│   │   ├── proxy/              # Local server, quota coordination, and web dashboard
│   │   ├── oauth/              # OAuth authorization flows
│   │   └── device/             # Device identification
│   ├── Cargo.lock
│   └── Cargo.toml
├── scripts/                    # Management & lifecycle scripts
│   ├── install.sh              # Linux / macOS installation & lifecycle script
│   ├── uninstall.sh            # Linux / macOS uninstallation script
│   ├── install.ps1             # Windows PowerShell installation & lifecycle script
│   └── uninstall.ps1           # Windows PowerShell uninstallation script
├── LICENSE                     # MIT License
├── README.md                   # English documentation
└── README_VI.md                # Vietnamese documentation
```

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
