#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
#  Agent Relay Manager (aam) - Kịch bản quản lý cài đặt
#  Sử dụng:
#    Cài đặt:        curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/install.sh | bash
#    Gỡ cài đặt:    curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/install.sh | bash -s -- uninstall
#    Cập nhật:       curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/install.sh | bash -s -- update
#    Cài đặt lại:    curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/install.sh | bash -s -- reinstall
#    Trạng thái:     curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/install.sh | bash -s -- status
# ==============================================================================

REPO="SonNX24042005/agents-account-manager"
INSTALL_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.agent-relay"
LEGACY_DATA_DIR="$HOME/.antigravity-relay"

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

show_help() {
    cat << 'EOF'
Agent Relay Manager (aam) - Kịch bản quản lý cài đặt

Cách sử dụng:
  ./scripts/install.sh [lệnh] [tùy_chọn]

Các lệnh khả dụng:
  install            Cài đặt aam và khởi động dịch vụ (mặc định)
  update, upgrade    Cập nhật aam lên phiên bản mới nhất từ GitHub
  uninstall, remove  Gỡ cài đặt aam khỏi hệ thống
  reinstall          Cài đặt lại binary và cấu hình
  status             Xem trạng thái hoạt động của dịch vụ
  help, -h, --help   Hiển thị hướng dẫn này

Tùy chọn gỡ cài đặt (uninstall):
  --purge, -p        Xóa toàn bộ thư mục dữ liệu cấu hình và tài khoản
  --keep-data        Giữ lại thư mục dữ liệu cấu hình mà không cần hỏi lại
EOF
    exit 0
}

do_install() {
    echo "====================================================="
    echo "   Cài đặt Agent Relay Manager (aam)"
    echo "====================================================="

    # 1. Check if running locally inside repo
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]:-}" 2>/dev/null)" 2>/dev/null && pwd || true)"
    LOCAL_REPO_DIR=""
    if [ -n "$SCRIPT_DIR" ] && [ -d "$SCRIPT_DIR/../agent-relay" ]; then
        LOCAL_REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
    elif [ -n "$SCRIPT_DIR" ] && [ -d "$SCRIPT_DIR/agent-relay" ]; then
        LOCAL_REPO_DIR="$SCRIPT_DIR"
    elif [ -d "$(pwd)/agent-relay" ]; then
        LOCAL_REPO_DIR="$(pwd)"
    elif [ -n "$SCRIPT_DIR" ] && [ -d "$SCRIPT_DIR/../antigravity-relay" ]; then
        LOCAL_REPO_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
    elif [ -n "$SCRIPT_DIR" ] && [ -d "$SCRIPT_DIR/antigravity-relay" ]; then
        LOCAL_REPO_DIR="$SCRIPT_DIR"
    elif [ -d "$(pwd)/antigravity-relay" ]; then
        LOCAL_REPO_DIR="$(pwd)"
    fi

    if [ -n "$LOCAL_REPO_DIR" ]; then
        SRC_DIR="agent-relay"
        [ ! -d "$LOCAL_REPO_DIR/$SRC_DIR" ] && SRC_DIR="antigravity-relay"
        echo "[build] Phát hiện mã nguồn cục bộ tại $LOCAL_REPO_DIR, đang biên dịch..."
        (cd "$LOCAL_REPO_DIR/$SRC_DIR" && cargo build --release -j 2)
        BIN_NAME="agent-relay"
        [ ! -f "$LOCAL_REPO_DIR/$SRC_DIR/target/release/$BIN_NAME" ] && BIN_NAME="antigravity-relay"
        cp -f "$LOCAL_REPO_DIR/$SRC_DIR/target/release/$BIN_NAME" "$INSTALL_DIR/agent-relay"
    else
        # 2. Download release artifact from GitHub Releases
        RELEASE_URL="https://github.com/${REPO}/releases/latest/download/agent-relay-${OS}-${TARGET_ARCH}.tar.gz"
        FALLBACK_URL="https://github.com/${REPO}/releases/latest/download/antigravity-relay-${OS}-${TARGET_ARCH}.tar.gz"
        CHECKSUM_URL="${RELEASE_URL}.sha256"
        FALLBACK_CHECKSUM_URL="${FALLBACK_CHECKSUM_URL:-${FALLBACK_URL}.sha256}"
        
        echo "[download] Đang tải bản phát hành từ GitHub..."
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
                        echo "[error] Mã kiểm tra SHA-256 không khớp. Đã hủy cài đặt."
                        exit 1
                    fi
                    echo "[verify] Đã xác minh mã băm SHA-256 thành công."
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
                echo "[build] Không thể tải pre-built binary, đang biên dịch từ GitHub repository..."
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
                echo "[error] Không thể tải bản phát hành và máy chưa cài đặt 'cargo' / 'git'."
                echo "        Vui lòng cài đặt Rust (https://rustup.rs) hoặc clone repo để build."
                exit 1
            fi
        fi
    fi

    chmod +x "$INSTALL_DIR/agent-relay"
    ln -sf "$INSTALL_DIR/agent-relay" "$INSTALL_DIR/aam"
    rm -f "$INSTALL_DIR/agyr"

    # Ensure PATH
    if [[ ":$PATH:" != *":$HOME/.local/bin:"* ]]; then
        echo '[config] Đang bổ sung ~/.local/bin vào PATH trong ~/.bashrc...'
        echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.bashrc"
        if [ -f "$HOME/.zshrc" ]; then
            echo 'export PATH="$HOME/.local/bin:$PATH"' >> "$HOME/.zshrc"
        fi
    fi

    echo ""
    echo "Cài đặt thành công lệnh 'aam'!"
    echo ""
    echo "Các lệnh sử dụng:"
    echo "  aam                    Tự động mở bảng điều khiển web và chạy dịch vụ"
    echo "  aam update             Cập nhật lệnh aam lên phiên bản mới nhất từ GitHub"
    echo "  aam start              Khởi chạy dịch vụ chạy ngầm"
    echo "  aam autostart          Tự động chạy cùng hệ thống (kể cả khi restart máy)"
    echo "  aam stop               Dừng dịch vụ"
    echo "  aam restart            Khởi động lại dịch vụ"
    echo "  aam status             Kiểm tra trạng thái hoạt động"
    echo "  aam version            Xem phiên bản hiện tại"
    echo "  aam disable            Tắt tự khởi động cùng máy"
    echo "  aam reinstall          Cài đặt lại binary và liên kết lệnh"
    echo "  aam uninstall          Gỡ cài đặt aam (thêm --purge để xóa cả dữ liệu)"
    echo ""
    echo "Bảng điều khiển: http://127.0.0.1:8045"
    echo ""
    echo "[start] Đang khởi động và mở bảng điều khiển..."
    if ! "$INSTALL_DIR/aam"; then
        echo "[cảnh báo] Không thể tự động mở trình duyệt. Hãy chạy lệnh 'aam' để thử lại."
    fi
}

do_update() {
    echo "====================================================="
    echo "   Cập nhật Agent Relay Manager (aam)"
    echo "====================================================="
    if [ -x "$INSTALL_DIR/aam" ]; then
        "$INSTALL_DIR/aam" update
    else
        echo "[update] Chưa phát hiện bản cài đặt aam, tiến hành cài đặt mới..."
        do_install
    fi
}

do_status() {
    if [ -x "$INSTALL_DIR/aam" ]; then
        "$INSTALL_DIR/aam" status
    else
        echo "Agent Relay Manager (aam) chưa được cài đặt."
    fi
}

do_uninstall() {
    local purge=false
    local keep_data=false

    for arg in "$@"; do
        case "$arg" in
            --purge|-p)
                purge=true
                ;;
            --keep-data|--no-purge)
                keep_data=true
                ;;
        esac
    done

    echo "====================================================="
    echo "   Gỡ cài đặt Agent Relay Manager (aam)"
    echo "====================================================="

    # 1. Dừng dịch vụ và tắt tự khởi động
    echo "[uninstall] Đang dừng dịch vụ nếu đang hoạt động..."
    if [ -x "$INSTALL_DIR/aam" ]; then
        "$INSTALL_DIR/aam" stop >/dev/null 2>&1 || true
        "$INSTALL_DIR/aam" disable >/dev/null 2>&1 || true
    fi

    if command -v systemctl >/dev/null 2>&1; then
        systemctl --user stop agent-relay.service >/dev/null 2>&1 || true
        systemctl --user stop antigravity-relay.service >/dev/null 2>&1 || true
        systemctl --user disable agent-relay.service >/dev/null 2>&1 || true
        systemctl --user disable antigravity-relay.service >/dev/null 2>&1 || true
    fi

    rm -f "$HOME/.config/systemd/user/agent-relay.service"
    rm -f "$HOME/.config/systemd/user/antigravity-relay.service"

    if command -v systemctl >/dev/null 2>&1; then
        systemctl --user daemon-reload >/dev/null 2>&1 || true
    fi

    if command -v pkill >/dev/null 2>&1; then
        pkill -f "agent-relay run" >/dev/null 2>&1 || true
        pkill -f "antigravity-relay run" >/dev/null 2>&1 || true
    fi

    # 2. Xóa các tệp thực thi và liên kết lệnh
    echo "[uninstall] Đang xóa các tệp thực thi và liên kết..."
    rm -f "$INSTALL_DIR/agent-relay"
    rm -f "$INSTALL_DIR/aam"
    rm -f "$INSTALL_DIR/agyr"
    rm -f "$INSTALL_DIR/antigravity-relay"

    # 3. Xử lý dọn dẹp dữ liệu cấu hình
    if [ "$purge" = false ] && [ "$keep_data" = false ] && [ -t 0 ]; then
        echo ""
        read -r -p "Bạn có muốn xóa toàn bộ dữ liệu cấu hình và tài khoản tại $DATA_DIR? [y/N]: " confirm || confirm="n"
        case "$confirm" in
            [yY]|[yY][eE][sS])
                purge=true
                ;;
        esac
    fi

    if [ "$purge" = true ]; then
        echo "[uninstall] Đang xóa dữ liệu cấu hình tại $DATA_DIR..."
        rm -rf "$DATA_DIR" "$LEGACY_DATA_DIR"
        echo "[uninstall] Đã xóa toàn bộ thư mục dữ liệu cấu hình."
    elif [ -d "$DATA_DIR" ]; then
        echo "[uninstall] Đã giữ lại thư mục dữ liệu tại $DATA_DIR"
        echo "            (Để xóa sạch hoàn toàn, bạn có thể xóa thư mục trên hoặc dùng cờ --purge)"
    fi

    echo ""
    echo "Gỡ cài đặt thành công Agent Relay Manager (aam)!"
}

do_reinstall() {
    echo "====================================================="
    echo "   Cài đặt lại Agent Relay Manager (aam)"
    echo "====================================================="
    do_uninstall --keep-data
    echo ""
    do_install
}

# Main command dispatch
ACTION="${1:-install}"
case "$ACTION" in
    install)
        do_install
        ;;
    update|upgrade)
        do_update
        ;;
    uninstall|remove)
        shift 1 2>/dev/null || true
        do_uninstall "$@"
        ;;
    reinstall)
        do_reinstall
        ;;
    status)
        do_status
        ;;
    help|--help|-h)
        show_help
        ;;
    *)
        echo "[lỗi] Lệnh không hợp lệ: '$ACTION'"
        echo ""
        show_help
        ;;
esac
