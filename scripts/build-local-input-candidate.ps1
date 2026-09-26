$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
Set-Location -LiteralPath $root
& pwsh -NoProfile -File (Join-Path $PSScriptRoot 'build-hid-host-helper.ps1')
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
$hidHelper=Join-Path $root 'target/dev/rc003-three-key/native/sayall-hid-host-helper.exe'
$env:SAYALL_HID_HOST_HELPER_SHA256=(Get-FileHash -LiteralPath $hidHelper).Hash.ToLowerInvariant()
# The final driver package must already have been built, reviewed, and locked.
# Microsoft signing can change SYS bytes: lock from that final package first.
Remove-Item Env:SAYALL_COMPONENT_HELPER_SHA256 -ErrorAction SilentlyContinue
& cargo rustc -p sayall-component-helper --release -- -C target-feature=+crt-static
if($LASTEXITCODE -ne 0){exit $LASTEXITCODE}
$helper=Join-Path $root 'target/release/sayall-component-helper.exe'
$env:SAYALL_COMPONENT_HELPER_SHA256=(Get-FileHash -LiteralPath $helper).Hash.ToLowerInvariant()
$configuration=@{bundle=@{createUpdaterArtifacts=$false;useLocalToolsDir=$true;resources=@{
  '../target/release/sayall-component-helper.exe'='sayall-component-helper.exe'
  '../target/dev/rc003-three-key/native/sayall-hid-host-helper.exe'='sayall-hid-host-helper.exe'
  '../target/frida-17.15.3/COPYING'='licenses/Frida-COPYING.txt'
  '../ATTRIBUTION.md'='licenses/ATTRIBUTION.md'
  '../target/input-driver/SayAllInput.inf'='SayAllInput/SayAllInput.inf'
  '../target/input-driver/SayAllInput.sys'='SayAllInput/SayAllInput.sys'
  '../target/input-driver/sayallinput.cat'='SayAllInput/sayallinput.cat'
}}}
$configPath=Join-Path $root 'target/sayall-input-candidate.json'
$configuration | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 $configPath
& pnpm tauri build --bundles nsis --config target/sayall-input-candidate.json --ci
exit $LASTEXITCODE
