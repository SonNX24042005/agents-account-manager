# Wrapper chuyển tiếp đến scripts/install.ps1
[CmdletBinding()]
param(
    [ValidateSet("install", "update", "uninstall", "reinstall", "status", "help")]
    [string]$Action = "install",
    [switch]$Purge,
    [switch]$KeepData,
    [string]$Version = ""
)

$localTarget = Join-Path $PSScriptRoot "scripts\install.ps1"
if (Test-Path $localTarget) {
    & $localTarget -Action $Action -Purge:$Purge -KeepData:$KeepData -Version $Version
} else {
    $script = Invoke-RestMethod -Uri "https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/install.ps1"
    Invoke-Expression "& { $script } -Action '$Action' -Purge:$Purge -KeepData:$KeepData -Version '$Version'"
}
