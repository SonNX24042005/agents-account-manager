# ==============================================================================
#  Agent Relay Manager (aam) - Windows PowerShell Uninstaller
#  Sử dụng:
#    irm https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/uninstall.ps1 | iex
#    .\scripts\uninstall.ps1 [-Purge]
# ==============================================================================

[CmdletBinding()]
param(
    [switch]$Purge,
    [switch]$KeepData
)

$ErrorActionPreference = "Stop"

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

Write-Host "====================================================="
Write-Host "   Gỡ cài đặt Agent Relay Manager (aam) trên Windows"
Write-Host "====================================================="

Write-Info "Đang dừng các tiến trình dịch vụ nếu đang hoạt động..."
Get-Process -Name "agent-relay", "aam", "antigravity-relay" -ErrorAction SilentlyContinue | ForEach-Object {
    try {
        Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
    } catch {}
}

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

# Remove path from User environment variable
$currentPath = [Environment]::GetEnvironmentVariable("Path", [EnvironmentVariableTarget]::User)
if ($currentPath) {
    $paths = $currentPath -split ";" | Where-Object { $_ -ne "" -and $_ -ne $InstallDir }
    $newPath = $paths -join ";"
    [Environment]::SetEnvironmentVariable("Path", $newPath, [EnvironmentVariableTarget]::User)
}

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
