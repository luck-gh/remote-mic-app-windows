$ErrorActionPreference='Stop'
Set-StrictMode -Version Latest
. (Join-Path $PSScriptRoot 'application-version.ps1')
$repositoryRoot=Split-Path -Parent $PSScriptRoot
$evidence=Join-Path $repositoryRoot 'target/dev/personal-version'
function Assert-Equal($Actual,$Expected){if($Actual -ne $Expected){throw "Expected $Expected, observed $Actual"}}
function Assert-Rejected([scriptblock]$Action,[string]$Expected){
    try{& $Action}catch{if($_.Exception.Message -notmatch $Expected){throw};return}
    throw 'Invalid version operation was accepted'
}
foreach($case in @(@('1.0.0','1.0.0'),@('1.0.0+1','1.0.0.1'),@('1.0.0+10','1.0.0.10'),@('1.0.1+65535','1.0.1.65535'))){
    Assert-Equal (ConvertTo-PublicAppVersion $case[0]) $case[1]
}
foreach($bad in @('1.0.0+0','1.0.0+01','1.0.0+65536','1.0.0+hash')){
    Assert-Rejected {ConvertTo-PublicAppVersion $bad} 'revision'
}
foreach($pair in @(@('1.0.0','1.0.0+1'),@('1.0.0+9','1.0.0+10'),@('1.0.0+65535','1.0.1+1'))){
    if((ConvertTo-ComparableAppVersion $pair[0]) -ge (ConvertTo-ComparableAppVersion $pair[1])){throw 'Comparable version order changed'}
}
Assert-Equal (ConvertTo-ComparableAppVersion '0.0.1') (ConvertTo-ComparableAppVersion '0.0.1.0')
foreach($case in @(@('1.0.0+1','1.0.0.1'),@('1.0.0+10','1.0.0.10'),@('0.2.0','0.2.0'))){
    $canonical=$case[0];$public=$case[1]
    $fixture=Join-Path $evidence ('public-'+$public)
    $configRoot=Join-Path $fixture 'src-tauri'
    $bundle=Join-Path $fixture 'target/release/bundle/nsis'
    New-Item -ItemType Directory -Force $configRoot,$bundle|Out-Null
    @{version=$canonical;productName='Fixture'}|ConvertTo-Json|Set-Content -Encoding utf8 (Join-Path $configRoot 'tauri.conf.json')
    $native=Join-Path $bundle "Fixture_$($canonical)_x64-setup.exe"
    [IO.File]::WriteAllBytes($native,[byte[]](1,2,3))
    'fixture-signature'|Set-Content -Encoding utf8 ($native+'.sig')
    $before=(Get-FileHash -LiteralPath $native -Algorithm SHA256).Hash
    $normalized=& (Join-Path $PSScriptRoot 'normalize-personal-package.ps1') -RepositoryRoot $fixture -BundleDirectory $bundle
    Assert-Equal ([IO.Path]::GetFileName($normalized)) "Fixture_$($public)_x64-setup.exe"
    Assert-Equal (Get-FileHash -LiteralPath $normalized -Algorithm SHA256).Hash $before
    Assert-Equal (& (Join-Path $PSScriptRoot 'normalize-personal-package.ps1') -RepositoryRoot $fixture -BundleDirectory $bundle) $normalized
    & (Join-Path $PSScriptRoot 'generate-updater-manifest.ps1') -Tag ('v'+$public) -RepositoryRoot $fixture|Out-Null
    $manifest=Get-Content -Raw (Join-Path $fixture 'artifacts/windows-release/latest.json')|ConvertFrom-Json
    Assert-Equal $manifest.version $canonical
    Assert-Equal $manifest.platforms.'windows-x86_64'.url "https://github.com/luck-gh/remote-mic-app-windows/releases/download/v$public/SayAll-Windows-$public-x64-setup.exe"
    Assert-Equal $manifest.platforms.'windows-x86_64'.signature 'fixture-signature'
    if($canonical -ne $public){Assert-Rejected {& (Join-Path $PSScriptRoot 'generate-updater-manifest.ps1') -Tag ('v'+$canonical) -RepositoryRoot $fixture} 'Tag .*tauri.conf.json'}
}
'passed: public version conversion/range, package/signature rename and idempotence, canonical manifest with public tag/assets, legacy three-part releases'
