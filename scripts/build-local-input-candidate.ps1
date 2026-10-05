$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $root
# beforeBuildCommand builds/stages the unified Helper and locked Gadget.
Remove-Item Env:VITE_SAYALL_RUNTIME_SIMULATION -ErrorAction SilentlyContinue
Remove-Item Env:SAYALL_WINDOWS_RUNTIME_SIMULATION -ErrorAction SilentlyContinue
$configuration=@{bundle=@{createUpdaterArtifacts=$false;useLocalToolsDir=$true}}
$configPath=Join-Path $root 'target/sayall-input-candidate.json'
New-Item -ItemType Directory -Force -Path (Split-Path -Parent $configPath) | Out-Null
$configuration | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 $configPath
& pnpm tauri build --bundles nsis --config target/sayall-input-candidate.json --ci
exit $LASTEXITCODE
