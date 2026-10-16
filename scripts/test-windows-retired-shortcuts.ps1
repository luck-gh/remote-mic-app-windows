$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
. (Join-Path $root 'src-tauri/windows/recycle-retired-files.ps1') -FunctionsOnly
$fixture = Join-Path $root 'target/dev/personal-branding/shortcut-fixture-updates'
if (Test-Path -LiteralPath $fixture) { throw 'Fixture already exists; review before reuse' }
$programs = Join-Path $fixture 'Programs'
$desktop = Join-Path $fixture 'Desktop'
$install = Join-Path $fixture 'Install'
$oldFolder = Join-Path $programs '无线麦 SayAll'
foreach ($folder in @($oldFolder, $desktop, $install)) { [IO.Directory]::CreateDirectory($folder) | Out-Null }
$target = Join-Path $install 'sayall-windows-app.exe'
[IO.File]::WriteAllText($target, 'fixture only; never execute')
$shell = New-Object -ComObject WScript.Shell
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
[StructLayout(LayoutKind.Sequential)] public struct ShortcutKey { public Guid id; public uint pid; }
[StructLayout(LayoutKind.Explicit, Size=24)] public struct ShortcutValue { [FieldOffset(0)] public ushort vt; [FieldOffset(8)] public IntPtr text; }
[ComImport, Guid("886D8EEB-8CF2-4446-8D02-CDBA1DBDCF99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface ShortcutStore {
 void GetCount(out uint count); void GetAt(uint index, out ShortcutKey key);
 void GetValue(ref ShortcutKey key, out ShortcutValue value); void SetValue(ref ShortcutKey key, ref ShortcutValue value); void Commit();
}
public static class ShortcutFixtureProperty {
 [DllImport("shell32.dll", CharSet=CharSet.Unicode, PreserveSig=false)]
 static extern void SHGetPropertyStoreFromParsingName(string path, IntPtr context, uint flags, ref Guid iid, out ShortcutStore store);
 [DllImport("ole32.dll")] static extern int PropVariantClear(ref ShortcutValue value);
 public static void SetAppId(string path, string id) {
  Guid iid = typeof(ShortcutStore).GUID; ShortcutStore store;
  SHGetPropertyStoreFromParsingName(path, IntPtr.Zero, 2, ref iid, out store);
  ShortcutKey key = new ShortcutKey { id = new Guid("9F4C2855-9F79-4B39-A8D0-E1D42DE1D5F3"), pid = 5 };
  ShortcutValue value = new ShortcutValue { vt = 31, text = Marshal.StringToCoTaskMemUni(id) };
  try { store.SetValue(ref key, ref value); store.Commit(); }
  finally { PropVariantClear(ref value); Marshal.FinalReleaseComObject(store); }
 }
}
'@
function New-TestShortcut([string]$Path, [string]$Target) {
    $link = $shell.CreateShortcut($Path)
    try { $link.TargetPath = $Target; $link.Save() }
    finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($link) }
}
try {
    $matching = Join-Path $oldFolder '无线麦 SayAll.lnk'
    $different = Join-Path $desktop '无线麦 SayAll.lnk'
    $other = Join-Path $programs 'User shortcut.lnk'
    New-TestShortcut $matching $target
    New-TestShortcut $different (Join-Path $install 'other.exe')
    New-TestShortcut $other $target
    $before = (Get-FileHash -LiteralPath $different).Hash
    Invoke-SayAllRetiredShortcutRecycle $install $programs $desktop
    if (Test-Path -LiteralPath $matching) { throw 'Matching retired shortcut remained' }
    if (Test-Path -LiteralPath $oldFolder) { throw 'Empty retired folder remained' }
    if ((Get-FileHash -LiteralPath $different).Hash -ne $before) { throw 'Different target changed' }
    if (-not (Test-Path -LiteralPath $other)) { throw 'Non-retired shortcut changed' }
    if ($script:ReplacedShortcutKinds -ne 1) { throw 'Matched default did not request recreation' }
    $custom = Join-Path $programs 'User-selected folder'
    [IO.Directory]::CreateDirectory($custom) | Out-Null
    $customOld = Join-Path $custom '无线麦 SayAll.lnk'
    $customNew = Join-Path $custom '无线麦.lnk'
    New-TestShortcut $customOld $target
    [ShortcutFixtureProperty]::SetAppId($customOld, 'fixture.personal.branding')
    if ((Get-SayAllShortcutAppId $customOld) -ne 'fixture.personal.branding') { throw 'Fixture AppUserModelID not observed' }
    Invoke-SayAllRetiredShortcutRecycle $install $programs $desktop 'User-selected folder' 1 0
    if ((Test-Path -LiteralPath $customOld) -or -not (Test-Path -LiteralPath $customNew)) { throw 'Update did not preserve custom location with new name' }
    if ($script:ReplacedShortcutKinds -ne 0) { throw 'Custom update requested an unrelated default shortcut' }
    if ((Get-SayAllShortcutAppId $customNew) -ne 'fixture.personal.branding') { throw 'Custom AppUserModelID changed' }
    $readback = $shell.CreateShortcut($customNew)
    try { if ($readback.Description -ne '无线麦 · 个人版') { throw 'Custom replacement personal identity description changed' }; if ($readback.TargetPath -ne $target -or $readback.IconLocation -ne ($target + ',0')) { throw 'Custom replacement target/icon not preserved' } }
    finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($readback) }
    $declined = Join-Path $programs 'Declined folder'
    [IO.Directory]::CreateDirectory($declined) | Out-Null
    New-TestShortcut (Join-Path $declined '无线麦 SayAll.lnk') $target
    Invoke-SayAllRetiredShortcutRecycle $install $programs $desktop '>Declined folder' 1 0
    if (Test-Path -LiteralPath (Join-Path $declined '无线麦.lnk')) { throw 'No-create selection was ignored' }
    Invoke-SayAllRetiredShortcutRecycle $install $programs $desktop '' 1 0
    if ($script:ReplacedShortcutKinds -ne 0) { throw 'Absent/mismatched legacy shortcut requested new entry' }
    $conflict = Join-Path $programs 'Conflict folder'
    [IO.Directory]::CreateDirectory($conflict) | Out-Null
    $conflictOld = Join-Path $conflict '无线麦 SayAll.lnk'
    $conflictNew = Join-Path $conflict '无线麦.lnk'
    New-TestShortcut $conflictOld $target
    New-TestShortcut $conflictNew (Join-Path $install 'other.exe')
    $conflictHash = (Get-FileHash -LiteralPath $conflictNew).Hash
    $conflictRejected = $false
    try { Invoke-SayAllRetiredShortcutRecycle $install $programs $desktop '' 1 0 } catch { $conflictRejected = $true }
    if (-not $conflictRejected -or -not (Test-Path -LiteralPath $conflictOld) -or (Get-FileHash -LiteralPath $conflictNew).Hash -ne $conflictHash) { throw 'Name collision did not preserve both links' }
    $recycleShell = New-Object -ComObject Shell.Application
    try {
        $items = @($recycleShell.Namespace(10).Items() | Where-Object { $_.ExtendedProperty('System.Recycle.DeletedFrom') -like "$fixture*" })
        if ($items.Count -lt 2) { throw 'Recycle Bin side effects not observed' }
    } finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($recycleShell) }
    $rejected = $false
    try { Invoke-SayAllRetiredShortcutRecycle $install '' $desktop } catch { $rejected = $true }
    if (-not $rejected) { throw 'Invalid shortcut root accepted' }
    if ((Get-FileHash -LiteralPath $different).Hash -ne $before) { throw 'Failure path changed user shortcut' }
    Write-Output 'retired-shortcuts: passed matching_recycled=true different_target_preserved=true other_name_preserved=true failure_preserved=true update_custom=true no_create=true absent=true app_id_preserved=true name_conflict_preserved=true'
} finally { [void][Runtime.InteropServices.Marshal]::FinalReleaseComObject($shell) }
# This fixture contains only this test's synthetic files. Retain no source/config copies.
Initialize-SayAllRecycleApi
[SayAllRecycle]::File($fixture)
