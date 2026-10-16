param(
    [Parameter(Mandatory = $true)][string]$Installer,
    [string]$ConfigPath = "",
    [string]$SignaturePath = ""
)
$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
if (-not $ConfigPath) { $ConfigPath = Join-Path $repo 'src-tauri/tauri.conf.json' }
if (-not $SignaturePath) { $SignaturePath = "$Installer.sig" }
& cargo run --release --locked -p sayall-windows-app --example verify-updater-signing -- $Installer $SignaturePath $ConfigPath
if ($LASTEXITCODE -ne 0) { throw 'Updater signature does not match the configured personal public key; publishing is blocked.' }
