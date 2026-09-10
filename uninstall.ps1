# Wrapper chuyển tiếp đến scripts/uninstall.ps1
[CmdletBinding()]
param(
    [switch]$Purge,
    [switch]$KeepData
)

$localTarget = Join-Path $PSScriptRoot "scripts\uninstall.ps1"
if (Test-Path $localTarget) {
    & $localTarget -Purge:$Purge -KeepData:$KeepData
} else {
    $script = Invoke-RestMethod -Uri "https://raw.githubusercontent.com/SonNX24042005/agents-account-manager/main/scripts/uninstall.ps1"
    Invoke-Expression "& { $script } -Purge:$Purge -KeepData:$KeepData"
}
