param([string]$RepositoryRoot = '', [string]$BundleDirectory = '')
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'application-version.ps1')
if (-not $RepositoryRoot) { $RepositoryRoot=Split-Path -Parent $PSScriptRoot }
if (-not $BundleDirectory) { $BundleDirectory=Join-Path $RepositoryRoot 'target/release/bundle/nsis' }
$bundleRoot=(Resolve-Path -LiteralPath $BundleDirectory).Path
$config=Get-Content -Raw -Encoding UTF8 (Join-Path $RepositoryRoot 'src-tauri/tauri.conf.json')|ConvertFrom-Json
$publicVersion=ConvertTo-PublicAppVersion ([string]$config.version)
$native=Join-Path $bundleRoot "$($config.productName)_$($config.version)_x64-setup.exe"
$public=Join-Path $bundleRoot "$($config.productName)_$($publicVersion)_x64-setup.exe"
foreach($path in @($native,$public)) {
    if([IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($path)) -ne $bundleRoot){throw 'Package path escapes selected bundle directory'}
}
if($native -ne $public -and (Test-Path -LiteralPath $native -PathType Leaf)) {
    if ((Test-Path -LiteralPath ($public+'.sig')) -and -not (Test-Path -LiteralPath ($native+'.sig'))) { throw 'A stale signature exists for the unsigned replacement package' }
    Move-Item -LiteralPath $native -Destination $public -Force
    if(Test-Path -LiteralPath ($native+'.sig')){Move-Item -LiteralPath ($native+'.sig') -Destination ($public+'.sig') -Force}
}
if(-not (Test-Path -LiteralPath $public -PathType Leaf)){throw 'Expected personal installer is missing'}
Write-Output $public
