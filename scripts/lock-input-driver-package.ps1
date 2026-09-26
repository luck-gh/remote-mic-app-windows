param([string]$PackageDirectory = 'target/input-driver', [switch]$DevelopmentUnsigned)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$package=(Resolve-Path -LiteralPath (Join-Path $root $PackageDirectory)).Path
$inf=Join-Path $package 'SayAllInput.inf'
$sys=Join-Path $package 'SayAllInput.sys'
$cat=Join-Path $package 'sayallinput.cat'
if ((Get-FileHash -LiteralPath $inf).Hash -ne (Get-FileHash -LiteralPath (Join-Path $root 'drivers/SayAllInput/SayAllInput.inf')).Hash) {throw 'INF differs from the reviewed source.'}
if (!$DevelopmentUnsigned) {
  $signtool='C:/Program Files (x86)/Windows Kits/10/bin/10.0.26100.0/x64/signtool.exe'
  foreach($member in @($inf,$sys)) {
    & $signtool verify /kp /v /c $cat $member
    if($LASTEXITCODE -ne 0){throw 'Microsoft kernel catalog/member verification failed; no identity updated.'}
  }
}
$hash=(Get-FileHash -LiteralPath $sys).Hash.ToLowerInvariant()
$text="// Build-time release identity. Regenerate from the final signed package; never supplied by IPC.`nconst INPUT_SYS_SHA256: &str = `"$hash`";`n"
[IO.File]::WriteAllText((Join-Path $root 'drivers/SayAllInput/package_identity.rs'),$text,[Text.UTF8Encoding]::new($false))
[ordered]@{recordedAt=(Get-Date).ToString('o');sysSha256=$hash;infSha256=(Get-FileHash -LiteralPath $inf).Hash.ToLowerInvariant();catalogSha256=(Get-FileHash -LiteralPath $cat).Hash.ToLowerInvariant();developmentUnsigned=[bool]$DevelopmentUnsigned;runtimeKernelVerification='always required, independent of build flag'} | ConvertTo-Json | Set-Content -Encoding utf8 (Join-Path $package 'package-identity-evidence.json')
Write-Output 'Fixed payload identity regenerated; runtime catalog verification remains mandatory.'
