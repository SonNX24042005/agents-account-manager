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
- **Chuyển đổi tài khoản tức thì**: Đổi tài khoản hoạt động nhanh chóng qua giao diện mà không cần sao chép thủ công thông tin đăng nhập.
- **Tự động chọn tài khoản thông minh**: Tự động nhận diện và chuyển sang tài khoản còn nhiều hạn ngạch nhất khi tài khoản hiện tại sắp hết mức sử dụng.
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

### 3. Kịch bản gỡ cài đặt nhanh

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
