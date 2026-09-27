$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$out=Join-Path $root 'target/frida-17.15.3'
$archive=Join-Path $out 'frida-core-devkit-17.15.3-windows-x86_64.exe'
if((Get-FileHash -LiteralPath $archive).Hash.ToLowerInvariant() -ne '52d4b60d0fb9f9e69f03c652d50d5f3f22c9c967b3d23ff26d33d3c9039bd2d3'){throw 'Official devkit hash mismatch'}
$source=[IO.File]::ReadAllText((Join-Path $root 'Testing/hid_host_probe.js'))
$literal=ConvertTo-Json -InputObject $source -Compress
[IO.File]::WriteAllText((Join-Path $out 'hid_host_probe_script.h'),'#define SAYALL_PROBE_SCRIPT '+$literal)
$vswhere='C:/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe'
$vs=& $vswhere -latest -products '*' -property installationPath
$vc=Get-ChildItem -LiteralPath (Join-Path $vs 'VC/Tools/MSVC') -Directory | Sort-Object Name -Descending | Select-Object -First 1
$sdk='C:/Program Files (x86)/Windows Kits/10'
$compiler=Join-Path $vc.FullName 'bin/Hostx64/x64/cl.exe'
$includes=@("/I$out","/I$($vc.FullName)/include","/I$sdk/Include/10.0.26100.0/ucrt","/I$sdk/Include/10.0.26100.0/shared","/I$sdk/Include/10.0.26100.0/um")
Add-Type @'
using System;
using System.IO;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
public static class SayAllBuildInputLock {
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern SafeFileHandle CreateFileW(string path,uint access,uint share,IntPtr sa,uint disposition,uint flags,IntPtr template);
 public static SafeFileHandle Directory(string path) {
  if((File.GetAttributes(path)&FileAttributes.ReparsePoint)!=0)throw new IOException("Reparse directory rejected");
  var file=CreateFileW(path,0x80,1,IntPtr.Zero,3,0x02200000,IntPtr.Zero);
  if(file.IsInvalid){file.Dispose();throw new IOException("Cannot lock build input directory");}
  if((File.GetAttributes(path)&FileAttributes.ReparsePoint)!=0){file.Dispose();throw new IOException("Build directory changed");}
  return file;
 }
}
'@
# These member hashes were independently derived by extracting the pinned
# official archive into a fresh directory; see official-link-inputs.json.
$members=@{
 'frida-core.h'='7d934aea720239e0b330059a911d0dd161583ef891dcb102ba006842e849a651'
 'frida-core.lib'='2a7e7ff532d1af6222deea363816ffb3b01171640e89f79bce26a41b2de47f72'
}
$locks=[Collections.Generic.List[IDisposable]]::new()
try {
 $ancestors=[Collections.Generic.List[string]]::new()
 $directory=[IO.DirectoryInfo]::new($out)
 while($null -ne $directory){$ancestors.Insert(0,$directory.FullName);$directory=$directory.Parent}
 foreach($directory in $ancestors){$locks.Add([SayAllBuildInputLock]::Directory($directory))}
 foreach($name in $members.Keys){
  $path=Join-Path $out $name
  if(([IO.File]::GetAttributes($path) -band [IO.FileAttributes]::ReparsePoint) -ne 0){throw 'Reparse member rejected'}
  $file=[IO.File]::Open($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
  $locks.Add($file)
  if([Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($file)).ToLowerInvariant() -ne $members[$name]){throw "Official build member mismatch: $name"}
 }
& $compiler /nologo /W3 /TC /MT /O2 /utf-8 @includes "$root/Testing/hid_host_probe.c" "/Fo$out/hid_host_probe.obj" "/Fe$out/sayall-hid-host-probe.exe" /link /DEPENDENTLOADFLAG:0x800 /DYNAMICBASE /NXCOMPAT "/LIBPATH:$out" "/LIBPATH:$($vc.FullName)/lib/x64" "/LIBPATH:$sdk/Lib/10.0.26100.0/ucrt/x64" "/LIBPATH:$sdk/Lib/10.0.26100.0/um/x64" cfgmgr32.lib wintrust.lib setupapi.lib onecore.lib authz.lib shell32.lib > (Join-Path $out 'probe-build.log') 2>&1
if($LASTEXITCODE -ne 0){Get-Content (Join-Path $out 'probe-build.log') -Tail 25;exit $LASTEXITCODE}
& $compiler /nologo /W3 /TC /MT /O2 /utf-8 @includes "$root/Testing/hid_host_probe_thread_test.c" "/Fo$out/hid_host_probe_thread_test.obj" "/Fe$out/hid_host_probe_thread_test.exe" /link /DEPENDENTLOADFLAG:0x800 /DYNAMICBASE /NXCOMPAT "/LIBPATH:$($vc.FullName)/lib/x64" "/LIBPATH:$sdk/Lib/10.0.26100.0/ucrt/x64" "/LIBPATH:$sdk/Lib/10.0.26100.0/um/x64" onecore.lib > (Join-Path $out 'probe-thread-test-build.log') 2>&1
if($LASTEXITCODE -ne 0){Get-Content (Join-Path $out 'probe-thread-test-build.log') -Tail 15;exit $LASTEXITCODE}
& (Join-Path $out 'hid_host_probe_thread_test.exe') > (Join-Path $root 'artifacts/hid-host-three-key-20260915/probe-thread-tests.log') 2>&1
if($LASTEXITCODE -ne 0){Get-Content (Join-Path $root 'artifacts/hid-host-three-key-20260915/probe-thread-tests.log') -Tail 10;exit $LASTEXITCODE}
@{recordedAt=(Get-Date).ToString('o');officialArchive='52d4b60d0fb9f9e69f03c652d50d5f3f22c9c967b3d23ff26d33d3c9039bd2d3';lockedMembers=$members;outputSha256=(Get-FileHash -LiteralPath (Join-Path $out 'sayall-hid-host-probe.exe')).Hash.ToLowerInvariant();cSha256=(Get-FileHash -LiteralPath (Join-Path $root 'Testing/hid_host_probe.c')).Hash.ToLowerInvariant();runtimeHeaderSha256=(Get-FileHash -LiteralPath (Join-Path $root 'Testing/hid_host_probe_runtime.h')).Hash.ToLowerInvariant();threadObserverSha256=(Get-FileHash -LiteralPath (Join-Path $root 'Testing/hid_host_probe_thread.h')).Hash.ToLowerInvariant();jsSha256=(Get-FileHash -LiteralPath (Join-Path $root 'Testing/hid_host_probe.js')).Hash.ToLowerInvariant()} | ConvertTo-Json -Depth 4 | Set-Content -Encoding utf8 (Join-Path $out 'probe-build-identity.json')
} finally {for($i=$locks.Count-1;$i -ge 0;$i--){$locks[$i].Dispose()}}
Write-Output 'probe_build=passed execution=not_started'
