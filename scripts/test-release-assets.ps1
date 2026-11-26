$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$repo = Split-Path -Parent $PSScriptRoot
$fixture = Join-Path $repo ('target/dev/release-assets/' + [guid]::NewGuid().ToString('N'))
$bundle = Join-Path $fixture 'target/release/bundle/nsis'
$config = Join-Path $fixture 'src-tauri'
$staged = Join-Path $fixture 'artifacts/windows-release'
$installer = Join-Path $bundle 'Fixture_1.0.0_x64-setup.exe'
$sig = "$installer.sig"
$assetName = 'SayAll-Windows-1.0.0-x64-setup.exe'
$generator = Join-Path $PSScriptRoot 'generate-updater-manifest.ps1'

function Assert-Rejected([scriptblock]$Action, [string]$Reason) {
    try {
        & $Action
    } catch {
        if ($_.Exception.Message -notmatch $Reason) { throw }
        return
    }
    throw "Expected rejection for $Reason"
}

New-Item -ItemType Directory -Path $bundle, $config | Out-Null
'{"version":"1.0.0","productName":"Fixture"}' | Set-Content -LiteralPath (Join-Path $config 'tauri.conf.json') -Encoding utf8
[IO.File]::WriteAllBytes($installer, [byte[]](1, 2, 3))
Assert-Rejected { & $generator -Tag 'v1.0.0.1' -RepositoryRoot $fixture } 'Tag .*tauri.conf.json'
Assert-Rejected { & $generator -Tag 'v1.0.0' -RepositoryRoot $fixture } 'Updater signature is missing'
if (Test-Path -LiteralPath $staged) { throw 'Rejected input produced release assets' }

'fixture-signature' | Set-Content -LiteralPath $sig -Encoding utf8
& $generator -Tag 'v1.0.0' -RepositoryRoot $fixture
$names = @(Get-ChildItem -LiteralPath $staged -File | Sort-Object Name | ForEach-Object Name)
if (($names -join ',') -cne "latest.json,$assetName") { throw "Unexpected release assets: $names" }
if ((Get-FileHash -LiteralPath (Join-Path $staged $assetName)).Hash -ne (Get-FileHash -LiteralPath $installer).Hash) {
    throw 'The staged installer bytes changed'
}
$update = Get-Content -LiteralPath (Join-Path $staged 'latest.json') -Raw | ConvertFrom-Json
$windows = $update.platforms.'windows-x86_64'
if ($update.version -ne '1.0.0' -or
    $windows.url -ne "https://github.com/luck-gh/remote-mic-app-windows/releases/download/v1.0.0/$assetName" -or
    $windows.signature -cne (Get-Content -LiteralPath $sig -Raw).Trim()) {
    throw 'Update manifest does not identify the exact staged installer and its signature'
}
'passed: legacy version and required signature, exact two-asset release staging, installer bytes and inline signature preserved'
