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
- **One-click account switching**: Switch active accounts instantly from the dashboard without manual credential copying.
- **Smart quota auto-selection**: Automatically detect and switch to the account with the highest remaining quota when limits are approached.
- **Real-time quota monitoring**: Keep track of remaining requests, rate limits, and reset schedules across accounts.
- **Zero environment pollution**: Runs cleanly in the background without modifying shell profiles (`.bashrc`, `.zshrc`) or creating invasive aliases.
- **Local web dashboard**: Clean, responsive dark-mode interface hosted locally at `http://127.0.0.1:8045`.

---

## Installation and usage

### 1. One-line quick install

Install and launch the service immediately with a single command:

- **Linux / macOS:**
  ```bash
  curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/install.sh | bash
  ```
  *(Alternatively, run `./scripts/install.sh` or `./install.sh` directly if cloned).*

- **Windows (PowerShell):**
  ```powershell
  irm https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/install.ps1 | iex
  ```
  *(Alternatively, run `.\scripts\install.ps1` or `.\install.ps1` directly if cloned).*

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

### 3. Quick uninstallation scripts

If you prefer to uninstall without using the CLI command:

- **Linux / macOS:**
  ```bash
  curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/uninstall.sh | bash
  # Or purge all account data:
  curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/uninstall.sh | bash -s -- --purge
  ```

- **Windows (PowerShell):**
  ```powershell
  irm https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/uninstall.ps1 | iex
  # Or purge all account data:
  .\uninstall.ps1 -Purge
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
├── install.sh                  # Root wrapper forwarding to scripts/install.sh
├── uninstall.sh                # Root wrapper forwarding to scripts/uninstall.sh
├── install.ps1                 # Root wrapper forwarding to scripts/install.ps1
├── uninstall.ps1               # Root wrapper forwarding to scripts/uninstall.ps1
├── LICENSE                     # MIT License
├── README.md                   # English documentation
└── README_VI.md                # Vietnamese documentation
```

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
