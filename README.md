# Agent Account Manager

Hệ thống quản trị đa tài khoản và điều phối hạn ngạch thông minh dành cho các AI coding agent, bao gồm **Antigravity CLI (`agy`)**, **Antigravity IDE**, **Claude Code**, **Cursor** và các agent trong hệ sinh thái AI coding.

---

## Tính năng chính

- **Hỗ trợ đa agent (Multi-agent ready)**: Thiết kế mở rộng để quản lý tài khoản, làm mới token và điều phối hạn ngạch cho nhiều coding agent khác nhau thay vì phụ thuộc vào một công cụ duy nhất.
- **Chuyển tài khoản siêu tốc 1-click**: Tự động đồng bộ sang OS Keyring (GNOME Keyring / Linux Secret Service) và cơ sở dữ liệu SQLite của Antigravity IDE (`state.vscdb`) trong chưa đầy 2ms.
- **Không gây ô nhiễm môi trường**: Chạy 100% độc lập, không chèn biến môi trường hay chỉnh sửa file shell (`.bashrc`, `.zshrc`), kết nối trực tiếp đến nhà cung cấp mô hình với tốc độ tối đa.
- **Tra cứu hạn ngạch theo thời gian thực**: Theo dõi hạn ngạch 5 giờ và hàng tuần của các nhóm mô hình (Gemini, Claude & GPT) kèm thời gian đếm ngược reset chính xác.
- **Tự động chọn tài khoản tốt nhất**: Tự động đánh giá và chọn tài khoản có hạn ngạch khả dụng cao nhất mỗi khi agent thực thi tác vụ.
- **Giao diện web tối giản**: Bảng điều khiển gọn gàng, tinh tế theo phong cách dark mode tối giản tại `http://127.0.0.1:8045`.

---

## Cài đặt và sử dụng

### 1. Cài đặt nhanh 1 dòng lệnh (không cần clone repo)

Tương tự như `claude code` hay `rustup`, bạn có thể cài đặt ngay lập tức ở bất kỳ máy nào bằng 1 lệnh curl:

```bash
curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/install.sh | bash
```

*(Hoặc nếu đã tải mã nguồn về máy, bạn có thể chạy trực tiếp `./install.sh`)*

Lệnh này tự động tải, xác minh checksum, cài đặt, khởi động dịch vụ và mở bảng điều khiển. Người dùng không phải nhập master key. Những lần sau chỉ cần chạy lệnh `aam` để mở lại bảng điều khiển với một phiên đăng nhập an toàn mới.

### 2. Các tùy chọn khởi chạy (`aam`)

- **Tự động chạy liên tục cùng hệ thống (Khuyên dùng - Auto-start on boot):**
  ```bash
  aam autostart
  ```
  *Dịch vụ sẽ tự khởi động ngầm mỗi khi mở máy, tự động hồi phục và bật lại sau 3 giây nếu bị tắt. Muốn dừng hoàn toàn chỉ cần gõ `aam stop` hoặc `aam disable`.*

- **Chạy nền thông thường:**
  ```bash
  aam start
  ```

- **Kiểm tra trạng thái:**
  ```bash
  aam status
  ```

- **Dừng dịch vụ:**
  ```bash
  aam stop
  ```

- **Cập nhật lên phiên bản mới nhất:**
  ```bash
  aam update
  ```

- **Khởi động lại dịch vụ:**
  ```bash
  aam restart
  ```

Truy cập bảng điều khiển web tại: [http://127.0.0.1:8045](http://127.0.0.1:8045)

### 3. Tự động đồng bộ không cần alias

Khi dịch vụ `aam` đang chạy, hệ thống sẽ tự động quét hạn ngạch ngầm và duy trì tài khoản tối ưu nhất vào OS Keyring và cấu hình agent. Bạn chỉ cần sử dụng các công cụ agent bình thường mà **không cần tạo bất kỳ alias hay cấu hình shell phức tạp nào**.

---

## Cấu trúc thư mục

```
├── agent-relay/                # Mã nguồn Rust backend và daemon
│   ├── src/
│   │   ├── cli.rs              # Trình quản lý dòng lệnh toàn cục (aam)
│   │   ├── storage/            # Quản lý tài khoản, OS Keyring và IDE SQLite
│   │   ├── proxy/              # Server Axum, quản lý token, tra cứu quota và UI
│   │   ├── oauth/              # Luồng đăng nhập OAuth an toàn
│   │   └── device/             # Định danh phần cứng độc lập
│   └── Cargo.toml
├── docs/                       # Tài liệu dự án và lộ trình phát triển
│   ├── ROADMAP.md              # Lộ trình và các giai đoạn phát triển
│   ├── CLI_REFERENCE.md        # Hướng dẫn chi tiết lệnh aam và API backend
│   ├── commands.md             # Tài liệu lệnh Antigravity CLI
│   └── tags.md                 # Tài liệu cờ tham số Antigravity CLI
├── .github/workflows/          # Tự động hóa CI/CD đóng gói release
├── install.sh                  # Script cài đặt 1 dòng lệnh qua curl
└── .gitignore
```

## Quản lý Antigravity, Codex và Claude

Giao diện có ba tab riêng. Mỗi tab chỉ hiển thị tài khoản, quota và công tắc tự động chọn của agent đó. Công tắc được lưu tại `agent-selection.json` trong thư mục dữ liệu của dịch vụ. Khi nâng cấp, Antigravity giữ chế độ tự động cũ; Codex và Claude mặc định tắt để bạn nhập và chọn tài khoản trước.

### Thêm và chuyển tài khoản

- **Antigravity:** tiếp tục đăng nhập Google hoặc nhập token trong tab Antigravity.
- **Codex:** mở tab Codex, chọn **Thêm tài khoản**:
  * *Đăng nhập ChatGPT / Codex (khuyên dùng)*: tự động mở liên kết đăng nhập trong trình duyệt (OpenAI OAuth) để bạn chọn tài khoản và cấp quyền; tài khoản sẽ tự động lưu và làm mới quota ngay khi hoàn tất.
  * *Nhập phiên từ tệp / máy*: nhập phiên hiện tại từ `~/.codex/auth.json` hoặc chọn tệp `auth.json` trên máy. Dịch vụ sử dụng `CODEX_HOME`, mặc định `~/.codex`. Chuyển tài khoản bằng tệp yêu cầu `cli_auth_credentials_store = "file"`; các chế độ `keyring`, `auto` và `ephemeral` được từ chối nếu cấu hình tường minh. Không tự sửa cấu hình hoặc Keychain của bạn.
- **Claude:** đăng nhập bằng `claude auth login`, rồi nhập phiên trong tab Claude tương tự. Tệp đăng nhập là `.credentials.json` trong `CLAUDE_CONFIG_DIR`, mặc định `~/.claude`. Hiện hỗ trợ chuyển bằng tệp trên Linux và Windows; chưa hỗ trợ chuyển Claude qua Keychain trên macOS.

Lặp lại đăng nhập và nhập phiên cho từng tài khoản. Nhập cùng một phiên sẽ cập nhật mục hiện có, không tạo bản sao. Tên/email dùng để nhận diện trên giao diện; thông tin xác thực nằm trong tệp đăng nhập gốc. Tài khoản có cùng email ở hai agent vẫn được quản lý riêng.

Tắt công tắc để chọn thủ công. Khi bật, dịch vụ chọn tài khoản có quota còn lại cao nhất trong agent đó, giữ tài khoản hiện tại nếu bằng điểm. Với Codex và Claude, điểm là phần trăm còn lại thấp nhất giữa cửa sổ ngắn và tuần; với Antigravity, dùng quota nhóm mô hình đã chọn và loại tài khoản hết quota tuần. Tài khoản có quota lỗi, chưa biết hoặc cũ quá 10 phút không được tự chọn. Đến giờ reset phải đọc lại quota, không tự coi là 100%.

Sau khi chuyển, mở lại phiên CLI/IDE của agent để nạp đăng nhập mới. Phiên đang chạy có thể giữ token trong bộ nhớ. Biến môi trường chứa API key và cấu hình xác thực do tổ chức quản lý có thể ưu tiên hơn tệp đăng nhập.

### Đọc quota và giới hạn tích hợp

- Codex cần CLI có `account/rateLimits/read` trong `codex app-server`. Dịch vụ khởi chạy một tiến trình đọc quota trong thư mục riêng, không tạo hội thoại hoặc gửi prompt. Mỗi tiến trình có thời gian chờ 25 giây, giới hạn đầu ra và được dọn sau khi đọc. Token được Codex làm mới sẽ được lưu lại. API key có thể nhập/chuyển thủ công nhưng không có quota thuê bao ChatGPT để xếp hạng.
- Claude đọc endpoint usage OAuth mà Claude Code sử dụng. Endpoint này chưa có cam kết API công khai ổn định; khi lỗi hoặc trả 429, giao diện hiển thị lỗi và ngừng dùng quota đó để tự chọn. Dịch vụ kiểm tra cách nhau ít nhất 5 phút, tôn trọng `Retry-After` trong khoảng 5 phút đến 24 giờ. Phiên Claude hết hạn cần đăng nhập lại bằng Claude Code rồi nhập lại vào danh sách. Chưa tự làm mới refresh token Claude.
- Antigravity giữ chu kỳ làm mới 30 giây. Codex/Claude kiểm tra mỗi 5 phút theo từng tài khoản, tuần tự và không chạy trùng. Nút làm mới không bỏ qua thời gian chờ để tránh bị giới hạn tần suất.
- Khi phát hiện phiên gốc chưa được nhận diện trong danh sách, tính năng tự chọn Codex/Claude chờ bạn nhập phiên đó thay vì ghi đè. Xóa tài khoản chỉ xóa khỏi danh sách quản lý, không đăng xuất CLI đang dùng.

Kho tài khoản Codex/Claude nằm tại `native-accounts.json` trong thư mục dữ liệu dịch vụ, được ghi với quyền `0600` trên Unix. Trước khi thay tệp đăng nhập, dịch vụ lưu một bản sao cạnh tệp gốc (`auth.aam.bak` hoặc `.credentials.aam.bak`). API danh sách chỉ trả metadata/quota, không trả token. Các endpoint `/api/agents/*` dùng cùng xác thực quản trị với giao diện hiện có.

Tài liệu đối chiếu ngày 2026-09-09: [Codex authentication](https://developers.openai.com/codex/auth), [Codex app-server](https://developers.openai.com/codex/app-server), [Claude authentication](https://code.claude.com/docs/en/authentication), [báo cáo giới hạn endpoint quota Claude](https://github.com/anthropics/claude-code/issues/30930). Môi trường kiểm tra: Codex CLI 0.153.4, Claude Code 2.1.250. Kiểm thử tự động sử dụng dữ liệu giả, không xác minh quota với tài khoản thuê bao thật.

Kiểm tra phần giao diện bằng Node.js mà không cần cài package:

```bash
node --test --test-concurrency=1 agent-relay/tests/ui.test.cjs
```

Các kiểm thử này kiểm tra logic đổi tab, phản hồi đến muộn, lưu công tắc thất bại, phạm vi thao tác và escape nội dung; không thay thế kiểm tra hiển thị trong trình duyệt.
