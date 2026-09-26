# Starts the fixed readonly metadata script through the actual Explorer desktop.
param([switch]$Win32)
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$path=Join-Path $root 'Testing/probe-rc003-hid-gatt.ps1'
$expected='2cec62f9b91e9ad99c725880b802dd98a70fc79aa98feb4a38f4a616160c7836'
if((Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant() -ne $expected){throw 'Probe source pin mismatch'}
$literal="'"+$root.Replace("'","''")+"'"
$command=@(
 '$ErrorActionPreference=''Stop''',
 ('Set-Location -LiteralPath '+$literal),
 '$path=Join-Path (Get-Location).Path ''Testing/probe-rc003-hid-gatt.ps1''',
 '$file=[IO.File]::Open($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)',
 'try {',
 '$sha=[Security.Cryptography.SHA256]::Create()',
 'try {$hash=[BitConverter]::ToString($sha.ComputeHash($file)).Replace(''-'','''').ToLowerInvariant()} finally {$sha.Dispose()}',
 ('if($hash -ne '''+$expected+'''){throw ''Probe source pin mismatch''}'),
 '$file.Position=0;$reader=[IO.StreamReader]::new($file,[Text.Encoding]::UTF8,$true,4096,$true)',
 'try {$text=$reader.ReadToEnd()} finally {$reader.Dispose()}',
 $(if($Win32){'& ([ScriptBlock]::Create($text)) -Win32'}else{'& ([ScriptBlock]::Create($text))'}),
 '} finally {$file.Dispose()}'
) -join [Environment]::NewLine
$encoded=[Convert]::ToBase64String([Text.Encoding]::Unicode.GetBytes($command))
$shell=New-Object -ComObject Shell.Application
$desktop=$shell.Windows().FindWindowSW(0,0,8,[ref]0,1)
if(!$desktop){throw 'Explorer desktop unavailable'}
$exe=Join-Path $env:WINDIR 'System32/WindowsPowerShell/v1.0/powershell.exe'
$desktop.Document.Application.ShellExecute($exe,"-NoProfile -STA -WindowStyle Hidden -EncodedCommand $encoded",$root,'open',0)
[pscustomobject]@{launchedAt=(Get-Date).ToString('o');sourceSha256=$expected;launch='Explorer';executionPolicyChanged=$false;report=$(if($Win32){'artifacts/hid-gatt-access-20260919/win32-result.json'}else{'artifacts/hid-gatt-access-20260919/winrt-result.json'})} | ConvertTo-Json -Compress
