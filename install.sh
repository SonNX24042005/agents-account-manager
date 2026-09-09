#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
#  Agent Relay Manager 1-Line Installer (aam)
#  Usage: curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/install.sh | bash
# ==============================================================================

REPO="SonNX24042005/agents-account-manager"
INSTALL_DIR="$HOME/.local/bin"
mkdir -p "$INSTALL_DIR"

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"

case "$ARCH" in
    x86_64|amd64)
        TARGET_ARCH="x86_64"
        ;;
    aarch64|arm64)
        TARGET_ARCH="aarch64"
        ;;
    *)
        TARGET_ARCH="$ARCH"
        ;;
esac

echo "====================================================="
echo "   Cai dat Agent Relay Manager (aam)"
echo "====================================================="

# 1. Check if running locally inside repo
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-}" 2>/dev/null)" 2>/dev/null && pwd || true)"
LOCAL_REPO_DIR=""
if [ -n "$SCRIPT_DIR" ] && [ -d "$SCRIPT_DIR/agent-relay" ]; then
    LOCAL_REPO_DIR="$SCRIPT_DIR"
elif [ -d "$(pwd)/agent-relay" ]; then
    LOCAL_REPO_DIR="$(pwd)"
elif [ -n "$SCRIPT_DIR" ] && [ -d "$SCRIPT_DIR/antigravity-relay" ]; then
    LOCAL_REPO_DIR="$SCRIPT_DIR"
elif [ -d "$(pwd)/antigravity-relay" ]; then
    LOCAL_REPO_DIR="$(pwd)"
fi

if [ -n "$LOCAL_REPO_DIR" ]; then
    SRC_DIR="agent-relay"
    [ ! -d "$LOCAL_REPO_DIR/$SRC_DIR" ] && SRC_DIR="antigravity-relay"
    echo "[build] Phat hien ma nguon cuc bo tai $LOCAL_REPO_DIR, dang bien dich..."
    (cd "$LOCAL_REPO_DIR/$SRC_DIR" && cargo build --release -j 2)
    BIN_NAME="agent-relay"
    [ ! -f "$LOCAL_REPO_DIR/$SRC_DIR/target/release/$BIN_NAME" ] && BIN_NAME="antigravity-relay"
    cp -f "$LOCAL_REPO_DIR/$SRC_DIR/target/release/$BIN_NAME" "$INSTALL_DIR/agent-relay"
else
    # 2. Download release artifact from GitHub Releases
    RELEASE_URL="https://github.com/${REPO}/releases/latest/download/agent-relay-${OS}-${TARGET_ARCH}.tar.gz"
    FALLBACK_URL="https://github.com/${REPO}/releases/latest/download/antigravity-relay-${OS}-${TARGET_ARCH}.tar.gz"
    CHECKSUM_URL="${RELEASE_URL}.sha256"
    FALLBACK_CHECKSUM_URL="${FALLBACK_URL}.sha256"
    
    echo "[download] Dang tai ban phat hanh tu GitHub..."
    TMP_DIR="$(mktemp -d)"
    ARCHIVE="$TMP_DIR/agent-relay.tar.gz"
    CHECKSUM_FILE="$TMP_DIR/agent-relay.tar.gz.sha256"
    cleanup() {
        rm -rf "$TMP_DIR"
    }
    trap cleanup EXIT

    DOWNLOAD_SUCCESS=false
    if ! curl --proto '=https' --tlsv1.2 -fsSL --retry 3 --max-time 120 "$RELEASE_URL" -o "$ARCHIVE" 2>/dev/null; then
        curl --proto '=https' --tlsv1.2 -fsSL --retry 3 --max-time 120 "$FALLBACK_URL" -o "$ARCHIVE" 2>/dev/null || true
        CHECKSUM_URL="$FALLBACK_CHECKSUM_URL"
    fi

    if [ -f "$ARCHIVE" ] && [ -s "$ARCHIVE" ]; then
        # If a SHA256 checksum asset is published, verify integrity
        if curl --proto '=https' --tlsv1.2 -fsSL --retry 2 --max-time 30 "$CHECKSUM_URL" -o "$CHECKSUM_FILE" 2>/dev/null; then
            EXPECTED_SHA256="$(awk 'NR == 1 { print $1 }' "$CHECKSUM_FILE")"
            if [[ "$EXPECTED_SHA256" =~ ^[[:xdigit:]]{64}$ ]]; then
                if command -v sha256sum >/dev/null 2>&1; then
                    ACTUAL_SHA256="$(sha256sum "$ARCHIVE" | awk '{ print $1 }')"
                elif command -v shasum >/dev/null 2>&1; then
                    ACTUAL_SHA256="$(shasum -a 256 "$ARCHIVE" | awk '{ print $1 }')"
                else
                    ACTUAL_SHA256=""
                fi

                if [ -n "$ACTUAL_SHA256" ] && [ "${ACTUAL_SHA256,,}" != "${EXPECTED_SHA256,,}" ]; then
                    echo "[error] Checksum SHA-256 khong khop. Da huy cai dat."
                    exit 1
                fi
                echo "[verify] Da xac minh ma bam SHA-256 thanh cong."
            fi
        fi

        ARCHIVE_ENTRIES="$(tar -tzf "$ARCHIVE" 2>/dev/null || true)"
        ENTRY_COUNT="$(printf '%s\n' "$ARCHIVE_ENTRIES" | sed '/^[[:space:]]*$/d' | wc -l)"
        ONLY_ENTRY="$(printf '%s\n' "$ARCHIVE_ENTRIES" | sed '/^[[:space:]]*$/d;s#^\./##')"
        if [ "$ENTRY_COUNT" -eq 1 ] && { [ "$ONLY_ENTRY" = "agent-relay" ] || [ "$ONLY_ENTRY" = "antigravity-relay" ]; }; then
            tar -xzf "$ARCHIVE" -C "$TMP_DIR"
            EXTRACTED="$TMP_DIR/$ONLY_ENTRY"
            if [ -f "$EXTRACTED" ] && [ ! -L "$EXTRACTED" ]; then
                cp -f "$EXTRACTED" "$INSTALL_DIR/agent-relay"
                DOWNLOAD_SUCCESS=true
            fi
        fi
    fi

    # 3. Fallback: Build from git repository if prebuilt binary download or extraction failed
    if [ "$DOWNLOAD_SUCCESS" != "true" ]; then
        if command -v cargo >/dev/null 2>&1 && command -v git >/dev/null 2>&1; then
            echo "[build] Khong the tai pre-built binary, dang bien dich tu GitHub repo..."
            TMP_SRC="$(mktemp -d)"
            git clone --depth 1 "https://github.com/${REPO}.git" "$TMP_SRC/repo"
            SRC_DIR="$TMP_SRC/repo/agent-relay"
            [ ! -d "$SRC_DIR" ] && SRC_DIR="$TMP_SRC/repo/antigravity-relay"
            (cd "$SRC_DIR" && cargo build --release -j 2)
            if [ -f "$SRC_DIR/target/release/agent-relay" ]; then
                cp -f "$SRC_DIR/target/release/agent-relay" "$INSTALL_DIR/agent-relay"
            else
                cp -f "$SRC_DIR/target/release/antigravity-relay" "$INSTALL_DIR/agent-relay"
            fi
            rm -rf "$TMP_SRC"
        else
            echo "[error] Khong the tai ban phat hanh va may chua cai dat 'cargo' / 'git'."
            echo "        Vui long cai dat Rust (https://rustup.rs) hoac clone repo de build."
            exit 1
        fi
    fi
fi

chmod +x "$INSTALL_DIR/agent-relay"
ln -sf "$INSTALL_DIR/agent-relay" "$INSTALL_DIR/aam"
rm -f "$INSTALL_DIR/agyr"

# Ensure PATH
if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
    echo '[config] Dang bo sung ~/.local/bin vao PATH trong ~/.bashrc...'
    echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
    if [ -f "$HOME/.zshrc" ]; then
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.zshrc"
    fi
fi

echo ""
echo "Cai dat thanh cong lenh 'aam'!"
echo ""
echo "Cac lenh su dung:"
echo "  aam              Tu dong mo bang dieu khien web va chay dich vu"
echo "  aam update       Cap nhat lenh aam len phien ban moi nhat tu GitHub"
echo "  aam start        Khoi chay dich vu chay ngam"
echo "  aam autostart    Tu dong chay cung he thong (ke ca khi restart may)"
echo "  aam stop         Dung dich vu"
echo "  aam restart      Khoi dong lai dich vu"
echo "  aam status       Kiem tra trang thai hoat dong"
echo "  aam version      Xem phien ban hien tai"
echo "  aam disable      Tat tu khoi dong cung may"
echo ""
echo "Bang dieu khien: http://127.0.0.1:8045"
echo ""
echo "[start] Dang khoi dong va mo bang dieu khien..."
if ! "$INSTALL_DIR/aam"; then
    echo "[warning] Khong the tu dong mo trinh duyet. Hay chay lenh 'aam' de thu lai."
fi
