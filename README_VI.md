# Quản lý tài khoản agent

<p align="left">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT"></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.80%2B-orange.svg" alt="Rust 1.80+"></a>
  <a href="#cài-đặt-và-sử-dụng"><img src="https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey.svg" alt="Platform: Linux | macOS | Windows"></a>
  <a href="https://github.com/SonNX24042005/agents-account-manager"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen.svg" alt="PRs Welcome"></a>
</p>

[English](README.md) | [Tiếng Việt](README_VI.md)

Công cụ quản lý đa tài khoản và điều phối hạn ngạch thông minh dành cho các agent lập trình AI, bao gồm **Antigravity**, **Claude Code** và **Codex**.

---

## Tính năng nổi bật

- **Hỗ trợ đa agent**: Quản lý nhiều tài khoản và hạn ngạch tập trung cho Antigravity, Claude Code và Codex trong cùng một nơi.
- **Giao diện terminal tương tác toàn màn hình (TUI)**: Theo dõi trạng thái, hạn ngạch real-time và thao tác phím tắt nhanh trực tiếp trên terminal với lệnh `aam tui`.
- **Thao tác nhanh qua dòng lệnh (CLI)**: Đầy đủ các lệnh quản lý tài khoản, chuyển đổi, làm mới, lọc theo agent và cấu hình định tuyến ngay trên terminal (`aam list`, `aam switch`, `aam preference`...).
- **Chuyển đổi tài khoản tức thì**: Đổi tài khoản hoạt động nhanh chóng qua giao diện web, TUI hoặc CLI mà không cần sao chép thủ công thông tin đăng nhập.
- **Tự động chọn tài khoản thông minh**: Tự động nhận diện mô hình đang dùng và chuyển sang tài khoản có hạn ngạch cao nhất khi mức sử dụng sắp hết.
- **Theo dõi hạn ngạch trực quan**: Cập nhật liên tục số lượng yêu cầu còn lại, giới hạn sử dụng và thời gian đặt lại hạn ngạch của từng tài khoản.
- **Không can thiệp môi trường hệ thống**: Hoạt động nền độc lập, không sửa đổi các tệp cấu hình shell (`.bashrc`, `.zshrc`) và không tạo alias phức tạp.
- **Bảng điều khiển web cục bộ**: Giao diện tối giản, trực quan, hỗ trợ chế độ tối tại địa chỉ `http://127.0.0.1:8045`.

---

## Cài đặt và sử dụng

### 1. Cài đặt nhanh 1 dòng lệnh

Cài đặt và khởi chạy dịch vụ ngay lập tức với một câu lệnh:

- **Linux / macOS:**
  ```bash
  curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/install.sh | bash
  ```
  *(Hoặc chạy `./scripts/install.sh` nếu đã tải mã nguồn).*

- **Windows (PowerShell):**
  ```powershell
  irm https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/install.ps1 | iex
  ```
  *(Hoặc chạy `.\scripts\install.ps1` nếu đã tải mã nguồn).*

Kịch bản sẽ tự động tải bản phát hành phù hợp với hệ điều hành, xác thực mã băm SHA-256, cài đặt lệnh `aam`, khởi động dịch vụ nền và mở bảng điều khiển trên trình duyệt.

### 2. Các lệnh điều khiển (`aam`)

- **Mở bảng điều khiển (mặc định):**
  ```bash
  aam
  ```

- **Tự khởi động cùng hệ thống (khuyên dùng):**
  ```bash
  aam autostart
  ```
  *Dịch vụ sẽ tự động chạy ngầm mỗi khi bật máy và tự phục hồi nếu bị dừng. Để tắt, chạy lệnh `aam disable`.*

- **Khởi chạy dịch vụ chạy ngầm:**
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

- **Khởi động lại dịch vụ:**
  ```bash
  aam restart
  ```

- **Cập nhật lên phiên bản mới nhất:**
  ```bash
  aam update
  ```

- **Cài đặt lại binary:**
  ```bash
  aam reinstall
  ```

- **Gỡ cài đặt:**
  ```bash
  # Gỡ bỏ binary và dịch vụ, giữ lại dữ liệu cấu hình và tài khoản:
  aam uninstall

  # Gỡ bỏ hoàn toàn và xóa sạch dữ liệu tài khoản:
  aam uninstall --purge
  ```

### 3. Giao diện trực quan và kiểm tra tổng quan

- **Xem danh sách tài khoản (mặc định khi chạy `aam`):**
  ```bash
  aam                         # Xem danh sách tài khoản toàn bộ agent (tương đương 'aam list')
  # Hoặc lọc theo điều kiện:
  aam list --active           # Chỉ hiển thị tài khoản đang hoạt động
  ```

- **Kiểm tra tổng quan hệ thống:**
  ```bash
  aam check                   # Kiểm tra nhanh trạng thái dịch vụ, số tài khoản, hạn ngạch của tất cả agent
  # Hoặc alias:
  aam overview
  ```

- **Mở bảng điều khiển web:**
  ```bash
  aam web                     # Mở bảng điều khiển web trên trình duyệt mặc định (hoặc 'aam open')
  ```

- **Giao diện terminal tương tác toàn màn hình (`aam tui`):**
  ```bash
  aam tui
  ```

  Các phím tắt chính trong giao diện TUI:
  - `Tab` hoặc `1`, `2`, `3`: Chuyển đổi giữa các agent (Antigravity, Codex, Claude).
  - `↑` / `↓` hoặc `j` / `k`: Di chuyển và chọn tài khoản trong danh sách.
  - `Enter` hoặc `s`: Chuyển sang tài khoản đang chọn (đồng bộ ngay vào OS Keyring và IDE database).
  - `+` hoặc `n`: Mở menu thêm tài khoản mới (Google OAuth cho Antigravity, Device OAuth cho Codex, import cho Claude, hoặc nhập thủ công).
  - `r`: Làm mới dữ liệu hạn ngạch ngay lập tức.
  - `p`: Đổi chế độ ưu tiên định tuyến (Auto -> Gemini -> Claude & GPT).
  - `a`: Tự động chọn tài khoản có hạn ngạch cao nhất.
  - `t`: Bật/tắt chế độ tự động chọn tài khoản cho agent hiện tại.
  - `d`: Xóa tài khoản đang chọn (có hộp thoại xác nhận `y`/`n`).
  - `w`: Mở nhanh giao diện web trên trình duyệt mặc định.
  - `q` hoặc `Esc`: Thoát giao diện TUI.

### 4. Cấu trúc lệnh phân cấp theo từng agent (`aam <agent> <lệnh_con>`)

Tổ chức lệnh gọn gàng, rõ ràng theo từng agent với các bí danh ngắn (`agy`, `codex`, `claude`):

- **Quản lý tài khoản Antigravity (`aam agy`):**
  ```bash
  aam agy                     # Xem nhanh danh sách tài khoản Antigravity
  aam agy list                # Xem danh sách kèm tùy chọn (--active, --json)
  aam agy switch 1            # Chuyển tài khoản #1 của Antigravity
  aam agy login               # Đăng nhập Google OAuth qua trình duyệt
  aam agy add --email <email> --access-token <token>  # Thêm tài khoản thủ công
  aam agy preference auto     # Đổi chế độ ưu tiên mô hình (auto | gemini | claude_gpt)
  aam agy auto-select         # Tự động chọn tài khoản có hạn ngạch cao nhất
  aam agy settings --enable   # Bật tự động chọn cho Antigravity
  aam agy refresh             # Làm mới hạn ngạch Antigravity
  aam agy reset               # Đặt lại thời gian chờ (cooldowns)
  aam agy delete 1            # Xóa tài khoản #1 của Antigravity
  ```

- **Quản lý tài khoản OpenAI Codex (`aam codex`):**
  ```bash
  aam codex                   # Xem nhanh danh sách tài khoản Codex
  aam codex list              # Xem danh sách kèm tùy chọn (--active, --json)
  aam codex switch 1          # Chuyển sang tài khoản #1 của Codex
  aam codex login             # Đăng nhập thiết bị qua trình duyệt (Device OAuth)
  aam codex import            # Tự động nhập tài khoản từ Codex CLI cục bộ
  aam codex add --email <email> --token <token>  # Thêm tài khoản thủ công
  aam codex auto-select       # Tự động chọn tài khoản Codex tối ưu
  aam codex settings --enable # Bật tự động chọn cho Codex
  aam codex refresh           # Làm mới hạn ngạch Codex
  aam codex delete 1          # Xóa tài khoản #1 của Codex
  ```

- **Quản lý tài khoản Claude Code (`aam claude`):**
  ```bash
  aam claude                  # Xem nhanh danh sách tài khoản Claude
  aam claude list             # Xem danh sách kèm tùy chọn (--active, --json)
  aam claude switch 1         # Chuyển sang tài khoản #1 của Claude
  aam claude import           # Tự động nhập tài khoản từ Claude Code CLI cục bộ
  aam claude add --email <email> --token <token>  # Thêm session token thủ công
  aam claude auto-select      # Tự động chọn tài khoản Claude tối ưu
  aam claude refresh          # Làm mới hạn ngạch Claude
  aam claude delete 1         # Xóa tài khoản #1 của Claude
  ```

- **Kiểm tra và thao tác chung (toàn bộ agent):**
  ```bash
  aam list                    # Xem toàn bộ tài khoản của tất cả agent
  aam switch 1                # Chuyển tài khoản trong danh sách tổng
  aam refresh                 # Làm mới hạn ngạch cho toàn bộ agent
  aam auto-select             # Tự động chọn tài khoản có hạn ngạch cao nhất
  aam settings                # Xem trạng thái tự động chọn của tất cả agent
  ```

### 5. Kịch bản gỡ cài đặt nhanh

Nếu bạn muốn gỡ cài đặt trực tiếp mà không dùng lệnh CLI:

- **Linux / macOS:**
  ```bash
  curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/uninstall.sh | bash
  # Hoặc xóa sạch dữ liệu cấu hình:
  curl -fsSL https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/uninstall.sh | bash -s -- --purge
  ```

- **Windows (PowerShell):**
  ```powershell
  irm https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/uninstall.ps1 | iex
  # Hoặc xóa sạch dữ liệu cấu hình:
  .\scripts\uninstall.ps1 -Purge
  ```

Địa chỉ bảng điều khiển web: [http://127.0.0.1:8045](http://127.0.0.1:8045)

---

## Quản lý tài khoản

Bảng điều khiển cung cấp các mục riêng cho từng agent được hỗ trợ:

- **Antigravity**: Đăng nhập trực tiếp qua Google OAuth hoặc nhập thông tin xác thực để quản lý hạn ngạch các mô hình Gemini và Claude.
- **Codex**: Đăng nhập qua trình duyệt hoặc nhập tệp phiên làm việc cục bộ để theo dõi giới hạn lượt dùng.
- **Claude**: Nhập phiên làm việc hiện tại của Claude Code để theo dõi mức sử dụng và chuyển đổi tài khoản.

### Chế độ tự động và thủ công

- **Bật tự động chọn**: Dịch vụ liên tục theo dõi hạn ngạch và tự động kích hoạt tài khoản có mức sử dụng còn lại cao nhất.
- **Chế độ thủ công**: Bạn có thể bấm chọn bất kỳ tài khoản nào trên giao diện khi muốn chủ động chỉ định tài khoản làm việc.

Sau khi chuyển đổi tài khoản, các phiên làm việc CLI hoặc IDE của bạn sẽ áp dụng thông tin đăng nhập mới ở lượt chạy kế tiếp hoặc sau khi khởi động lại phiên đó.

---

## Cấu trúc thư mục

```
├── agent-relay/                # Mã nguồn Rust backend và daemon
│   ├── src/
│   │   ├── cli.rs              # Trình quản lý dòng lệnh toàn cục (aam)
│   │   ├── client.rs           # Khách gọi REST API nội bộ daemon
│   │   ├── tui.rs              # Giao diện terminal tương tác toàn màn hình (Ratatui)
│   │   ├── storage/            # Quản lý lưu trữ tài khoản và đồng bộ đăng nhập
│   │   ├── proxy/              # Máy chủ cục bộ, điều phối hạn ngạch và giao diện web
│   │   ├── oauth/              # Luồng xác thực đăng nhập OAuth
│   │   └── device/             # Định danh thiết bị
│   ├── Cargo.lock
│   └── Cargo.toml
├── scripts/                    # Thư mục chứa các kịch bản quản lý
│   ├── install.sh              # Kịch bản cài đặt và quản lý vòng đời trên Linux / macOS
│   ├── uninstall.sh            # Kịch bản gỡ cài đặt độc lập trên Linux / macOS
│   ├── install.ps1             # Kịch bản cài đặt và quản lý vòng đời trên Windows PowerShell
│   └── uninstall.ps1           # Kịch bản gỡ cài đặt độc lập trên Windows PowerShell
├── LICENSE                     # Giấy phép MIT
├── README.md                   # Tài liệu tiếng Anh
└── README_VI.md                # Tài liệu tiếng Việt
```

---

## Giấy phép

Dự án được phân phối theo giấy phép MIT. Xem chi tiết tại tệp [LICENSE](LICENSE).
