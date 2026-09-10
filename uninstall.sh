#!/usr/bin/env bash
set -euo pipefail

# ==============================================================================
#  Agent Relay Manager (aam) - Kịch bản gỡ cài đặt (Uninstall script)
#  Sử dụng:
#    curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/uninstall.sh | bash
#    curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/uninstall.sh | bash -s -- --purge
#    ./uninstall.sh [--purge]
# ==============================================================================

INSTALL_DIR="$HOME/.local/bin"
DATA_DIR="$HOME/.agent-relay"
LEGACY_DATA_DIR="$HOME/.antigravity-relay"

show_help() {
    cat << 'EOF'
Agent Relay Manager (aam) - Hướng dẫn gỡ cài đặt

Cách sử dụng:
  ./uninstall.sh [tùy_chọn]

Tùy chọn:
  --purge, -p       Xóa toàn bộ thư mục dữ liệu cấu hình và tài khoản (~/.agent-relay)
  --keep-data       Giữ lại thư mục dữ liệu cấu hình mà không cần hỏi lại
  -h, --help        Hiển thị hướng dẫn này
EOF
    exit 0
}

PURGE=false
KEEP_DATA=false

for arg in "$@"; do
    case "$arg" in
        --purge|-p)
            PURGE=true
            ;;
        --keep-data|--no-purge)
            KEEP_DATA=true
            ;;
        -h|--help|help)
            show_help
            ;;
        *)
            echo "[cảnh báo] Bỏ qua tham số không xác định: $arg"
            ;;
    esac
done

echo "====================================================="
echo "   Gỡ cài đặt Agent Relay Manager (aam)"
echo "====================================================="

# 1. Dừng dịch vụ nếu đang chạy
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

# 2. Xóa các tệp thực thi và liên kết
echo "[uninstall] Đang xóa các tệp thực thi và liên kết lệnh..."
rm -f "$INSTALL_DIR/agent-relay"
rm -f "$INSTALL_DIR/aam"
rm -f "$INSTALL_DIR/agyr"
rm -f "$INSTALL_DIR/antigravity-relay"

# 3. Xử lý dọn dẹp dữ liệu cấu hình
if [ "$PURGE" = false ] && [ "$KEEP_DATA" = false ] && [ -t 0 ]; then
    echo ""
    read -r -p "Bạn có muốn xóa toàn bộ dữ liệu cấu hình và tài khoản tại $DATA_DIR? [y/N]: " confirm || confirm="n"
    case "$confirm" in
        [yY]|[yY][eE][sS])
            PURGE=true
            ;;
    esac
fi

if [ "$PURGE" = true ]; then
    echo "[uninstall] Đang xóa dữ liệu cấu hình tại $DATA_DIR..."
    rm -rf "$DATA_DIR" "$LEGACY_DATA_DIR"
    echo "[uninstall] Đã xóa toàn bộ thư mục dữ liệu cấu hình."
elif [ -d "$DATA_DIR" ]; then
    echo "[uninstall] Đã giữ lại thư mục dữ liệu tại $DATA_DIR"
    echo "            (Để xóa sạch hoàn toàn, bạn có thể xóa thư mục trên hoặc dùng cờ --purge)"
fi

echo ""
echo "Gỡ cài đặt thành công Agent Relay Manager (aam)!"
