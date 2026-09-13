# Kiến trúc hệ thống và cơ chế hoạt động (`agent-relay` / `aam`)

Tài liệu này mô tả chi tiết kiến trúc tổng thể, các phân hệ thành phần và các cơ chế cốt lõi của hệ thống quản lý tài khoản và điều phối hạn ngạch thông minh Agent Account Manager (`aam`).

---

## 1. Tổng quan kiến trúc hệ thống

Hệ thống được thiết kế theo mô hình dịch vụ nền (daemon) nhẹ, không xâm lấn, hoạt động độc lập và không can thiệp vào các tệp cấu hình shell của người dùng (`.bashrc`, `.zshrc`):

```
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                     Người dùng và công cụ                               │
│        [Antigravity CLI (agy)]      [Antigravity IDE]      [Codex CLI]    [Claude Code] │
└───────────────────────────┬───────────────────┬──────────────────┬──────────────┬───────┘
                            │                   │                  │              │
                     (Script bọc agy)           │                  │              │
                            │                   │                  │              │
                            ▼                   │                  │              │
┌───────────────────────────────────────────────┴──────────────────┴──────────────┴───────┐
│                                 OS keyring và tệp cấu hình cục bộ                       │
│   • Secret Service / GNOME Keyring (service: gemini, username: antigravity)             │
│   • SQLite state.vscdb (antigravityUnifiedStateSync.oauthToken)                         │
│   • ~/.codex/auth.json                                                                  │
│   • ~/.claude.json                                                                      │
└──────────────────────────────────────────────────▲──────────────────────────────────────┘
                                                   │ Đồng bộ tức thì (sync)
┌──────────────────────────────────────────────────┴──────────────────────────────────────┐
│                            agent-relay daemon (Rust Axum, cổng 8045)                    │
│                                                                                         │
│  ┌───────────────────────┐  ┌────────────────────────┐  ┌────────────────────────────┐  │
│  │   Background worker   │  │     Warmup service     │  │   Dynamic model router     │  │
│  │   • Quét quota (30s)  │  │   • Kích hoạt sớm 5h   │  │   • Transcript scanner     │  │
│  │   • Tự động chọn TK   │  │   • Ping mô hình nhẹ   │  │   • Quota delta tracker    │  │
│  └───────────────────────┘  └────────────────────────┘  └────────────────────────────┘  │
│                                                                                         │
│  ┌───────────────────────────────────────────────────────────────────────────────────┐  │
│  │                         Kho lưu trữ tài khoản và định danh phần cứng              │  │
│  │   • ~/.agent-relay/accounts/*.json (mã hóa an toàn, phân tách theo UUID)          │  │
│  │   • Cô lập định danh phần cứng (machine_id, mac_machine_id, dev_device_id)        │  │
│  └───────────────────────────────────────────────────────────────────────────────────┘  │
│                                                                                         │
│  ┌───────────────────────────────────────────────────────────────────────────────────┐  │
│  │                                 Giao diện người dùng                              │  │
│  │   • Dòng lệnh toàn cục (aam)   • Terminal UI (aam tui)   • Web dashboard (:8045)  │  │
│  └───────────────────────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Cơ chế kích hoạt sớm bộ đếm 5 giờ (warmup service)

### 2.1. Vấn đề thực tế
Hạn ngạch của các nhà cung cấp AI (Google Cloud Code PA, OpenAI ChatGPT Codex, Anthropic Claude) thường có chu kỳ trượt 5 giờ hoặc chu kỳ ngày/tuần:
- Cửa sổ 5 giờ chỉ bắt đầu tính thời gian đếm ngược (cooldown countdown) từ thời điểm tài khoản phát sinh yêu cầu (request) đầu tiên.
- Nếu một tài khoản phụ đang ở trạng thái đầy 100% hạn ngạch mà không được sử dụng, đồng hồ 5 giờ sẽ không bao giờ kích hoạt. Khi tài khoản chính cạn hạn ngạch và hệ thống chuyển sang tài khoản phụ, người dùng phải chờ đủ 5 giờ kể từ lúc đó thì tài khoản phụ mới được hồi phục lần đầu. Điều này làm lãng phí thời gian hồi hạn ngạch trong ngày.

### 2.2. Giải pháp và nguyên lý hoạt động
`WarmupService` (triển khai tại `agent-relay/src/proxy/warmup.rs`) chủ động kích hoạt sớm chu kỳ hồi phục 5 giờ bằng cách gửi một yêu cầu truy vấn siêu nhẹ (ping request) ngay khi tài khoản đạt đủ điều kiện:
1. **Điều kiện kích hoạt**:
   - Hạn ngạch 5 giờ của nhóm mô hình tương ứng đang ở mức 100% (`effective_percentage >= 100.0`).
   - Hạn ngạch tuần (weekly quota) vẫn còn khả dụng (`> 0%`).
   - Tài khoản chưa từng được kích hoạt hoặc lần kích hoạt gần nhất đã cách thời điểm hiện tại từ 5 giờ trở lên (`now - last_warmup_at >= 5 hours`).
2. **Hành vi khi kích hoạt**:
   - Gửi yêu cầu truy vấn tối thiểu đến endpoint chính thức của nhà cung cấp.
   - Nhận phản hồi thành công và cập nhật dấu mốc thời gian `last_warmup_at = Some(now)`.
   - Cửa sổ đếm ngược 5 giờ bắt đầu chạy ngay lập tức. Sau 5 giờ, tài khoản sẽ tự động nạp đầy 100% hạn ngạch mới.

### 2.3. Endpoint và mô hình theo từng agent

| Agent | Endpoint | Mô hình | Cấu hình & payload |
|---|---|---|---|
| **Antigravity** | `POST https://daily-cloudcode-pa.googleapis.com/v1internal:streamGenerateContent?alt=sse`<br>*(fallback: `cloudcode-pa.googleapis.com`)* | `gemini-3.6-flash-high` | Header `User-Agent: Antigravity/1.0.0`<br>`{"project": "aicode-consumers", "model": "gemini-3.6-flash-high", "request": {"contents": [{"role": "user", "parts": [{"text": "ping"}]}]}}` |
| **Codex** | `POST https://chatgpt.com/backend-api/codex/responses` | `gpt-5.6-luna` | Header `User-Agent: codex-cli/0.154.0`<br>`{"model": "gpt-5.6-luna", "input": [{"role": "user", "content": "ping"}], "store": false, "stream": true}` |
| **Claude** | `POST https://api.anthropic.com/v1/messages` | `claude-3-5-haiku-20241022` | Headers `anthropic-version: 2023-06-01`, `anthropic-beta: oauth-2025-04-20`<br>`{"model": "claude-3-5-haiku-20241022", "max_tokens": 1, "messages": [{"role": "user", "content": "ping"}]}` |

### 2.4. Điều phối và chu kỳ thực thi
- **Tiến trình nền (background task)**: Tích hợp trực tiếp vào vòng lặp kiểm tra của `TokenManager` (đối với Antigravity) và `AgentManager` (đối với Codex và Claude), chạy tự động mỗi 30 giây trong tiến trình `agent-relay.service`.
- **Làm mới thủ công**: Khi người dùng gọi lệnh làm mới hoặc API refresh (`/api/accounts`, `/api/agents/refresh`), warmup service sẽ đánh giá lại điều kiện và kích hoạt ngay nếu có tài khoản đủ chuẩn.

---

## 3. Cơ chế script bọc nhị phân cho Antigravity CLI (`agy`)

### 3.1. Mục tiêu thiết kế
- Cho phép người dùng chạy lệnh `agy` tự nhiên trên terminal mà luôn được tự động chuyển sang tài khoản có hạn ngạch cao nhất.
- Tự động chuyển tiếp cờ an toàn `--dangerously-skip-permissions` để tránh gián đoạn khi thực thi tác vụ tự động.
- Không sửa đổi biến môi trường, không tạo alias trong shell profile (`.bashrc`, `.zshrc`).
- Đảm bảo an toàn tuyệt đối cho tệp nhị phân gốc: tự động hoàn nguyên khi gỡ cài đặt.

### 3.2. Cấu trúc và luồng hoạt động
Khi tính năng script bọc được kích hoạt:
1. Tệp nhị phân thực thi gốc của `agy` (thường nằm tại `~/.local/bin/agy` hoặc trong `$PATH`) được đổi tên thành `agy-bin` tại cùng thư mục:
   ```bash
   mv -f "$INSTALL_DIR/agy" "$INSTALL_DIR/agy-bin"
   ```
2. Một script thực thi bash được tạo tại vị trí `agy`:
   ```bash
   #!/usr/bin/env bash
   # Tự động chọn tài khoản có hạn ngạch cao nhất cho Antigravity
   model=""
   iter_next=0
   for arg in "$@"; do
       if [ "$iter_next" -eq 1 ]; then
           model="$arg"
           iter_next=0
       elif [ "$arg" = "--model" ] || [ "$arg" = "-m" ]; then
           iter_next=1
       elif [[ "$arg" == --model=* ]]; then
           model="${arg#--model=}"
       elif [[ "$arg" == -m=* ]]; then
           model="${arg#-m=}"
       fi
   done

   if command -v aam >/dev/null 2>&1; then
       if [ -n "$model" ]; then
           aam agy auto-select --model "$model" >/dev/null 2>&1
       else
           aam agy auto-select >/dev/null 2>&1
       fi
   fi

   # Bảo đảm giữ nguyên cờ --dangerously-skip-permissions nếu chưa có
   has_skip=0
   for arg in "$@"; do
       if [ "$arg" = "--dangerously-skip-permissions" ]; then
           has_skip=1
           break
       fi
   done

   if [ "$has_skip" -eq 1 ]; then
       exec "$(dirname "$0")/agy-bin" "$@"
   else
       exec "$(dirname "$0")/agy-bin" --dangerously-skip-permissions "$@"
   fi
   ```
3. Khi người dùng thực thi `agy <lệnh>`:
   - Script tự động bóc tách cờ `--model` hoặc `-m` (nếu có) từ danh sách đối số và gọi `aam agy auto-select --model <mô-hình>` (hoặc gọi tự động theo phiên làm việc gần nhất nếu dùng `-c` / `--continue`).
   - Script gọi ngầm `aam agy auto-select` để kích hoạt tài khoản tối ưu và đồng bộ thông tin xác thực vào keyring hệ thống (`KeyringSync`) và cơ sở dữ liệu IDE (`IdeDbSync`).
   - Script kiểm tra cờ tham số, tự động chèn `--dangerously-skip-permissions` nếu chưa có.
   - Thay thế tiến trình hiện tại bằng `agy-bin` qua lệnh `exec`, giữ nguyên mã thoát (exit code) và các luồng nhập xuất chuẩn (stdin/stdout/stderr).

### 3.3. Thuật toán so sánh và chọn tài khoản tối ưu nhiều tầng
Khi chọn tài khoản tối ưu (`select_best_account_for_category` và `compare_quota_priority`), hệ thống áp dụng bộ tiêu chí xếp hạng phân tầng:
1. **Tầng 1 (Hạn ngạch 5 giờ)**: Tài khoản có phần trăm hạn ngạch 5 giờ cao hơn sẽ được ưu tiên hàng đầu.
2. **Tầng 2 (Hạn ngạch tuần - Tie-breaker 1)**: Khi các tài khoản hòa điểm hạn ngạch 5 giờ (ví dụ đều 100%), hệ thống so sánh hạn ngạch tuần còn lại. Tài khoản có tỷ lệ hạn ngạch tuần cao hơn (ví dụ 92% so với 67%) sẽ được ưu tiên chọn.
3. **Tầng 3 (Thời gian hồi phục 5 giờ - Tie-breaker 2)**: Khi hạn ngạch 5 giờ và hạn ngạch tuần bằng nhau, tài khoản có thời điểm reset 5 giờ đến sớm hơn (ví dụ còn ~30m so với ~4h 50m) sẽ được ưu tiên dùng trước để tận dụng tối đa chu kỳ làm mới hạn ngạch sắp tới.
4. **Tầng 4 (Giữ nguyên tài khoản active - Tie-breaker 3)**: Chỉ khi toàn bộ các tiêu chí trên hoàn toàn trùng khớp, hệ thống mới giữ nguyên tài khoản đang hoạt động để tránh chuyển đổi phiên không cần thiết.

### 3.4. Tích hợp vòng đời (lifecycle)
- **Cài đặt**: Được kích hoạt tự động trong `scripts/install.sh` (`setup_agy_wrapper`) và qua lệnh CLI `aam install` / `aam reinstall` (`setup_agy_wrapper_if_present` trong `agent-relay/src/cli.rs`).
- **Gỡ cài đặt**: Trong `scripts/uninstall.sh` và `aam uninstall`, hàm hoàn nguyên sẽ xóa script bọc và đổi tên `agy-bin` trở lại thành `agy`, đảm bảo trả lại nguyên trạng hệ thống.

---

## 4. Cơ chế hiển thị đếm ngược hạn ngạch liên tục

Nhằm giúp người dùng theo dõi sát sao chu kỳ reset 5 giờ:
- Khi tài khoản được kích hoạt sớm bởi warmup service, hạn ngạch vẫn hiển thị 100% nhưng mốc thời gian reset trong tương lai đã tồn tại.
- Hàm `primary_reset_countdown()` trong `client.rs` trích xuất thông tin đếm ngược của bucket reset sớm nhất mà không bị giới hạn bởi điều kiện phần trăm nhỏ hơn 100%.
- Cả ba giao diện hiển thị đều đồng bộ format thời gian:
  - Bảng tổng quan `aam check`: Cột hạn ngạch hiển thị dạng `100% (~4h 55m)`.
  - Bảng chi tiết `aam list` / `aam agy list`: Cột chi tiết hiển thị `5h: 100% (~4h 55m) · Tuần: 90%`.
  - Giao diện terminal `aam tui`: Danh sách tài khoản hiển thị `100% (~4h 55m)` và panel chi tiết bên phải hiển thị thanh tiến trình kèm nhãn `Gemini Models (100%) - hồi sau ~4h 55m`.

---

## 5. Cơ chế đồng bộ thông tin xác thực (credential synchronization)

### 5.1. Đồng bộ OS keyring (`KeyringSync`)
- Antigravity CLI sử dụng Secret Service API trên Linux (thông qua thư viện `go-keyring`).
- Khi người dùng chuyển đổi tài khoản hoặc hệ thống tự động chọn tài khoản, `agent-relay` ghi trực tiếp token mới vào dịch vụ keyring với định danh:
  - Service: `gemini`
  - Username: `antigravity`
  - Value: Access token mới nhất kèm thông tin xác thực.

### 5.2. Đồng bộ cơ sở dữ liệu Antigravity IDE (`IdeDbSync`)
- Cơ sở dữ liệu SQLite của Antigravity IDE được lưu trữ tại `~/.config/Antigravity IDE/User/globalStorage/state.vscdb`.
- Dữ liệu token được nén và mã hóa dưới dạng Protobuf nhị phân trong khóa `antigravityUnifiedStateSync.oauthToken`.
- `IdeDbSync` giải mã, cập nhật access token, refresh token và email mới, sau đó ghi ngược trở lại SQLite một cách an toàn.

### 5.3. Cô lập định danh phần cứng (`DeviceProfile`)
- Để tránh việc các tài khoản bị liên đới do sử dụng chung một thông số máy, mỗi tài khoản khi tạo mới được gán một hồ sơ phần cứng độc lập (`DeviceProfile`).
- Các giá trị `machine_id`, `mac_machine_id`, `dev_device_id`, `sqm_id` được sinh ngẫu nhiên theo chuẩn UUID v4 và lưu kèm trong tệp cấu hình của từng tài khoản.

---

## 6. Bảo mật và quản lý phiên (security model)

Hệ thống tuân thủ các nguyên tắc bảo mật nghiêm ngặt:
- **So sánh token thời gian cố định (constant-time comparison)**: Xác thực khóa quản trị `master_key` thông qua so sánh mã băm SHA-256 bằng hàm `ct_eq`, ngăn chặn hoàn toàn nguy cơ tấn công kênh kề qua đo đếm thời gian (timing attack).
- **Chống duyệt thư mục (path traversal protection)**: Mọi thao tác đọc, sửa, xóa tài khoản đều kiểm tra nghiêm ngặt định dạng UUID của định danh tài khoản, từ chối mọi chuỗi chứa ký tự phân tách thư mục (`/`, `\`, `..`).
- **Bảo vệ phiên giao diện web**: Giao diện web sử dụng cơ chế bootstrap token một lần (`/api/session/bootstrap`) và chuyển đổi sang session cookie (`aam_session`) với cờ `HttpOnly` và `SameSite=Strict`, không nhúng khóa bí mật vào mã JavaScript phía máy khách.
- **Không can thiệp shell profile**: Không sửa đổi `.bashrc`, `.zshrc`, không tạo alias toàn cục gây xung đột môi trường.
