# ==============================================================================
#  Agent Relay Manager (aam) - Windows PowerShell Installer
#  Sử dụng:
#    Cài đặt:        irm https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/install.ps1 | iex
#    Gỡ cài đặt:    .\install.ps1 -Action uninstall [-Purge]
#    Cập nhật:       .\install.ps1 -Action update
# ==============================================================================

[CmdletBinding()]
param(
    [ValidateSet("install", "update", "uninstall", "reinstall", "status", "help")]
    [string]$Action = "install",
    [switch]$Purge,
    [switch]$KeepData,
    [string]$Version = ""
)

$ErrorActionPreference = "Stop"

$Repo = "SonNX24042005/agents-account-manager"
$InstallDir = Join-Path $env:LOCALAPPDATA "agent-relay\bin"
$DataDir = Join-Path $env:USERPROFILE ".agent-relay"
$LegacyDataDir = Join-Path $env:USERPROFILE ".antigravity-relay"

function Write-Info {
    param([string]$Message)
    Write-Host "[info] $Message" -ForegroundColor Cyan
}

function Write-Success {
    param([string]$Message)
    Write-Host "[ok] $Message" -ForegroundColor Green
}

function Write-Warn {
    param([string]$Message)
    Write-Host "[cảnh báo] $Message" -ForegroundColor Yellow
}

function Write-Err {
    param([string]$Message)
    Write-Host "[lỗi] $Message" -ForegroundColor Red
}

function Show-Help {
    Write-Host "Agent Relay Manager (aam) - Kịch bản quản lý cài đặt trên Windows"
    Write-Host ""
    Write-Host "Cách sử dụng:"
    Write-Host "  .\install.ps1 [-Action <lệnh>] [-Purge] [-KeepData]"
    Write-Host ""
    Write-Host "Các lệnh khả dụng:"
    Write-Host "  install      Cài đặt aam và mở bảng điều khiển (mặc định)"
    Write-Host "  update       Cập nhật lên phiên bản mới nhất từ GitHub"
    Write-Host "  uninstall    Gỡ cài đặt aam khỏi hệ thống"
    Write-Host "  reinstall    Cài đặt lại binary và đường dẫn"
    Write-Host "  status       Xem trạng thái hoạt động của dịch vụ"
    Write-Host "  help         Hiển thị hướng dẫn này"
    Write-Host ""
    Write-Host "Tùy chọn cho gỡ cài đặt (uninstall):"
    Write-Host "  -Purge       Xóa toàn bộ thư mục dữ liệu cấu hình và tài khoản ($DataDir)"
    Write-Host "  -KeepData    Giữ lại dữ liệu cấu hình mà không cần hỏi lại"
}

function Stop-RelayService {
    Write-Info "Đang dừng tiến trình dịch vụ nếu đang hoạt động..."
    Get-Process -Name "agent-relay", "aam", "antigravity-relay" -ErrorAction SilentlyContinue | ForEach-Object {
        try {
            Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
        } catch {}
    }
}

function Ensure-PathEnvironment {
    $currentPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
    $paths = $currentPath -split ";" | Where-Object { $_ -ne "" }
    if ($paths -notcontains $InstallDir) {
        Write-Info "Đang bổ sung $InstallDir vào biến môi trường PATH của người dùng..."
        $newPath = ($paths + $InstallDir) -join ";"
        [Environment]::SetEnvironmentVariable("Path", $newPath, [EnvironmentVariableTarget]::User)
        $env:Path = "$env:Path;$InstallDir"
        Write-Success "Đã thêm đường dẫn vào PATH."
    }
}

function Remove-PathEnvironment {
    $currentPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
    if ($currentPath) {
        $paths = $currentPath -split ";" | Where-Object { $_ -ne "" -and $_ -ne $InstallDir }
        $newPath = $paths -join ";"
        [Environment]::SetEnvironmentVariable("Path", $newPath, [EnvironmentVariableTarget]::User)
    }
}

function Invoke-Install {
    Write-Host "====================================================="
    Write-Host "   Cài đặt Agent Relay Manager (aam) cho Windows"
    Write-Host "====================================================="

    if (-not (Test-Path $InstallDir)) {
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    }

    $arch = if ([System.Environment]::Is64BitOperatingSystem) { "x86_64" } else { "x86" }
    $assetName = "agent-relay-windows-$arch.zip"
    $releaseUrl = if ($Version -ne "") {
        "https://github.com/$Repo/releases/download/v$Version/$assetName"
    } else {
        "https://github.com/$Repo/releases/latest/download/$assetName"
    }
    $checksumUrl = "$releaseUrl.sha256"

    $tempDir = Join-Path $env:TEMP ("aam-install-" + [Guid]::NewGuid().ToString())
    New-Item -ItemType Directory -Path $tempDir -Force | Out-Null

    try {
        $zipPath = Join-Path $tempDir $assetName
        $checksumPath = Join-Path $tempDir "$assetName.sha256"

        Write-Info "Đang tải bản phát hành từ GitHub..."
        try {
            Invoke-WebRequest -Uri $releaseUrl -OutFile $zipPath -UseBasicParsing -TimeoutSec 120
        } catch {
            Write-Err "Không thể tải tệp phát hành từ $releaseUrl: $_"
            exit 1
        }

        # Checksum verification
        try {
            Invoke-WebRequest -Uri $checksumUrl -OutFile $checksumPath -UseBasicParsing -TimeoutSec 30 -ErrorAction SilentlyContinue
            if (Test-Path $checksumPath) {
                $expectedHash = (Get-Content $checksumPath -Raw).Trim().Split()[0].ToLower()
                if ($expectedHash.Length -eq 64) {
                    $actualHash = (Get-FileHash -Path $zipPath -Algorithm SHA256).Hash.ToLower()
                    if ($actualHash -ne $expectedHash) {
                        Write-Err "Mã băm SHA-256 không khớp. Đã hủy cài đặt."
                        exit 1
                    }
                    Write-Success "Đã xác minh mã băm SHA-256 thành công."
                }
            }
        } catch {
            Write-Warn "Không thể kiểm tra checksum, tiếp tục giải nén..."
        }

        Write-Info "Đang giải nén gói cài đặt..."
        Expand-Archive -Path $zipPath -DestinationPath $tempDir -Force

        $extractedExe = Join-Path $tempDir "agent-relay.exe"
        if (-not (Test-Path $extractedExe)) {
            $extractedExe = Join-Path $tempDir "antigravity-relay.exe"
        }

        if (-not (Test-Path $extractedExe)) {
            Write-Err "Không tìm thấy tệp thực thi bên trong gói zip."
            exit 1
        }

        Stop-RelayService

        $targetExe = Join-Path $InstallDir "agent-relay.exe"
        $targetAam = Join-Path $InstallDir "aam.exe"

        Copy-Item -Path $extractedExe -Destination $targetExe -Force
        Copy-Item -Path $extractedExe -Destination $targetAam -Force
        Write-Success "Đã cài đặt tệp thực thi vào $InstallDir"

        Ensure-PathEnvironment

        Write-Host ""
        Write-Success "Cài đặt thành công lệnh 'aam'!"
        Write-Host ""
        Write-Host "Các lệnh sử dụng:"
        Write-Host "  aam                    Tự động mở bảng điều khiển web và chạy dịch vụ"
        Write-Host "  aam update             Cập nhật lệnh aam lên phiên bản mới nhất từ GitHub"
        Write-Host "  aam start              Khởi chạy dịch vụ chạy ngầm"
        Write-Host "  aam stop               Dừng dịch vụ"
        Write-Host "  aam restart            Khởi động lại dịch vụ"
        Write-Host "  aam status             Kiểm tra trạng thái hoạt động"
        Write-Host "  aam version            Xem phiên bản hiện tại"
        Write-Host "  aam reinstall          Cài đặt lại binary"
        Write-Host "  aam uninstall          Gỡ cài đặt aam (thêm --purge để xóa cả dữ liệu)"
        Write-Host ""
        Write-Host "Bảng điều khiển: http://127.0.0.1:8045"
        Write-Host ""
        Write-Info "Đang khởi động dịch vụ và mở bảng điều khiển..."

        Start-Process -FilePath $targetAam
    } finally {
        Remove-Item -Path $tempDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Invoke-Uninstall {
    Write-Host "====================================================="
    Write-Host "   Gỡ cài đặt Agent Relay Manager (aam) trên Windows"
    Write-Host "====================================================="

    Stop-RelayService

    Write-Info "Đang xóa các tệp thực thi..."
    $filesToRemove = @(
        Join-Path $InstallDir "agent-relay.exe",
        Join-Path $InstallDir "aam.exe",
        Join-Path $InstallDir "antigravity-relay.exe"
    )

    foreach ($file in $filesToRemove) {
        if (Test-Path $file) {
            Remove-Item -Path $file -Force -ErrorAction SilentlyContinue
            Write-Info "Đã xóa: $file"
        }
    }

    if ((Test-Path $InstallDir) -and (Get-ChildItem -Path $InstallDir -Force | Measure-Object).Count -eq 0) {
        Remove-Item -Path $InstallDir -Force -ErrorAction SilentlyContinue
        $parentDir = Split-Path -Path $InstallDir -Parent
        if ((Test-Path $parentDir) -and (Get-ChildItem -Path $parentDir -Force | Measure-Object).Count -eq 0) {
            Remove-Item -Path $parentDir -Force -ErrorAction SilentlyContinue
        }
    }

    Remove-PathEnvironment

    $shouldPurge = $Purge
    if (-not $shouldPurge -and -not $KeepData -and [Environment]::UserInteractive) {
        $confirm = Read-Host "Bạn có muốn xóa toàn bộ dữ liệu cấu hình và tài khoản tại $DataDir? [y/N]"
        if ($confirm -match "^[yY](es)?$") {
            $shouldPurge = $true
        }
    }

    if ($shouldPurge) {
        Write-Info "Đang xóa thư mục dữ liệu cấu hình tại $DataDir..."
        if (Test-Path $DataDir) {
            Remove-Item -Path $DataDir -Recurse -Force -ErrorAction SilentlyContinue
        }
        if (Test-Path $LegacyDataDir) {
            Remove-Item -Path $LegacyDataDir -Recurse -Force -ErrorAction SilentlyContinue
        }
        Write-Success "Đã xóa toàn bộ thư mục dữ liệu cấu hình."
    } elseif (Test-Path $DataDir) {
        Write-Info "Đã giữ lại thư mục cấu hình tại $DataDir (dùng -Purge để xóa sạch)."
    }

    Write-Host ""
    Write-Success "Gỡ cài đặt thành công Agent Relay Manager (aam)!"
}

function Invoke-Update {
    Write-Host "====================================================="
    Write-Host "   Cập nhật Agent Relay Manager (aam)"
    Write-Host "====================================================="
    $aamPath = Join-Path $InstallDir "aam.exe"
    if (Test-Path $aamPath) {
        & $aamPath update
    } else {
        Write-Info "Chưa phát hiện bản cài đặt aam, tiến hành cài đặt mới..."
        Invoke-Install
    }
}

function Invoke-Status {
    $aamPath = Join-Path $InstallDir "aam.exe"
    if (Test-Path $aamPath) {
        & $aamPath status
    } else {
        Write-Host "Agent Relay Manager (aam) chưa được cài đặt."
    }
}

switch ($Action) {
    "install"   { Invoke-Install }
    "update"    { Invoke-Update }
    "uninstall" { Invoke-Uninstall }
    "reinstall" {
        Invoke-Uninstall
        Invoke-Install
    }
    "status"    { Invoke-Status }
    "help"      { Show-Help }
    default     { Show-Help }
}
