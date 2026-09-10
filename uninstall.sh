#!/usr/bin/env bash
set -euo pipefail

# Wrapper chuyển tiếp đến scripts/uninstall.sh
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-}" 2>/dev/null)" 2>/dev/null && pwd || true)"
if [ -n "$SCRIPT_DIR" ] && [ -f "$SCRIPT_DIR/scripts/uninstall.sh" ]; then
    exec "$SCRIPT_DIR/scripts/uninstall.sh" "$@"
elif [ -f "$(pwd)/scripts/uninstall.sh" ]; then
    exec "$(pwd)/scripts/uninstall.sh" "$@"
else
    exec curl -fsSL "https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/uninstall.sh" | bash -s -- "$@"
fi
