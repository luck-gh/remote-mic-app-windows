$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $root
# beforeBuildCommand builds/stages the unified Helper and locked Gadget.
Remove-Item Env:VITE_SAYALL_RUNTIME_SIMULATION -ErrorAction SilentlyContinue
Remove-Item Env:SAYALL_WINDOWS_RUNTIME_SIMULATION -ErrorAction SilentlyContinue
$env:SAYALL_BUILD_CHANNEL='local'
$env:SAYALL_BUNDLE_BUILD='1'
$configuration=@{bundle=@{createUpdaterArtifacts=$false;useLocalToolsDir=$true}}
$configPath=Join-Path $root 'target/sayall-input-candidate.json'
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $configPath) | Out-Null
$configuration | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 $configPath
& pnpm tauri build --bundles nsis --config target/sayall-input-candidate.json --ci
$buildExit = $LASTEXITCODE
if ($buildExit -ne 0) { exit $buildExit }
$targetRoot = if ([string]::IsNullOrWhiteSpace($env:CARGO_TARGET_DIR)) {
    Join-Path $root 'target'
} elseif ([IO.Path]::IsPathRooted($env:CARGO_TARGET_DIR)) {
    $env:CARGO_TARGET_DIR
} else {
    Join-Path $root $env:CARGO_TARGET_DIR
}
& powershell -NoProfile -ExecutionPolicy Bypass -File (Join-Path $root 'scripts/verify-frontend-embedded.ps1') -Executable (Join-Path $targetRoot 'release/sayall-windows-app.exe') -FrontendDirectory (Join-Path $root 'dist')
exit $LASTEXITCODE
