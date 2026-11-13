$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$recycler = Join-Path $root 'src-tauri/windows/recycle-retired-files.ps1'
. $recycler -FunctionsOnly
$fixture = Join-Path $root ('target/dev/unified-input/payload-lock-' + [Guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($fixture) | Out-Null
foreach ($name in @('sayall-windows-app.exe', 'sayall-helper.exe', 'frida-gadget.dll', 'settings.json')) {
    [IO.File]::WriteAllText((Join-Path $fixture $name), 'isolated payload fixture')
}
# 只运行本测试复制的命令解释器, stdin保持存活, 最终通过exit正常退出。
$mcp = Join-Path $fixture 'sayall-mcp.exe'
[IO.File]::Copy($env:ComSpec, $mcp)
$before = @{}
foreach ($file in Get-ChildItem -LiteralPath $fixture -File) { $before[$file.Name] = (Get-FileHash -LiteralPath $file.FullName).Hash }
# 回收边界替身只计数, RED阶段也绝不移动测试文件。
Add-Type 'public static class SayAllRecycle { public static int Calls; public static void File(string path) { Calls++; } }'
$start = [Diagnostics.ProcessStartInfo]::new()
$start.FileName = $mcp
$start.Arguments = '/D /Q'
$start.UseShellExecute = $false
$start.CreateNoWindow = $true
$start.RedirectStandardInput = $true
$start.RedirectStandardOutput = $true
$start.RedirectStandardError = $true
$process = [Diagnostics.Process]::Start($start)
try {
    if ($process.HasExited) { throw 'Isolated executable exited before the lock check' }
    $blocked = $false
    try { Invoke-SayAllRetiredFileRecycle $fixture -ProductFiles | Out-Null } catch { $blocked = $true }
    if (-not $blocked) { throw 'Running MCP did not block product recycling before the first mutation' }
    if ([SayAllRecycle]::Calls -ne 0) { throw 'Some product files were recycled before the MCP lock was detected' }
    $oldPreference = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { $result = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $recycler -InstallDirectory $fixture -CheckProductAccess 2>&1; $exitCode = $LASTEXITCODE }
    finally { $ErrorActionPreference = $oldPreference }
    if ($exitCode -ne 32 -or ($result -join "`n") -notmatch 'result=blocked') { throw 'Installer preflight did not report an explicit blocked result and exit 32' }
    foreach ($name in $before.Keys) {
        if ((Get-FileHash -LiteralPath (Join-Path $fixture $name)).Hash -ne $before[$name]) { throw 'A blocked preflight changed a payload or user state' }
    }
} finally {
    $process.StandardInput.WriteLine('exit')
    $process.StandardInput.Flush()
    if (-not $process.WaitForExit(5000)) { throw 'Isolated command interpreter did not exit normally' }
    $process.Dispose()
}
# 非MCP尾部文件也必须在回收第一项之前检查完整集合。
$locked = [IO.File]::Open((Join-Path $fixture 'frida-gadget.dll'), [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
try {
    $blocked = $false
    try { Invoke-SayAllRetiredFileRecycle $fixture -ProductFiles | Out-Null } catch { $blocked = $true }
    if (-not $blocked -or [SayAllRecycle]::Calls -ne 0) { throw 'Whole-set preflight did not block a locked non-MCP payload before recycling' }
} finally { $locked.Dispose() }
Assert-SayAllPayloadAccess $fixture
$hooks = Get-Content -LiteralPath (Join-Path $root 'src-tauri/windows/installer-hooks.nsh') -Raw
foreach ($entry in @('PREINSTALL', 'PREUNINSTALL')) {
    $body = [regex]::Match($hooks, '(?s)!macro NSIS_HOOK_' + $entry + '\b(.*?)!macroend').Groups[1].Value
    if ($body -notmatch 'SayAllVerifyPayloadAccess') { throw "$entry does not guard product changes against a running MCP" }
}
if ($hooks -notmatch '"sayall-mcp.exe"\s*\r?\n\s*StrCpy \$SayAllInstallStage mcp') { throw 'MCP resource lacks its fixed diagnostic component' }
Write-Output 'payload-lock: passed; running MCP and locked non-MCP block before recycling, installer preflight exits 32, original bytes preserved, release succeeds'
# 仅保留本次小型夹具到任务收尾, 由负责人按回收站规则清理。
