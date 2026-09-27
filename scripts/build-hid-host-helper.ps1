param([switch]$SourceB,[switch]$TestLifecycle)
$ErrorActionPreference='Stop'
if($SourceB -and $TestLifecycle){throw 'Select only one fixed build target'}
$root=Split-Path -Parent $PSScriptRoot
$devkit=Join-Path $root 'target/frida-17.15.3'
$out=Join-Path $root $(if($SourceB){'target/dev/rc003-three-key/source-b'}else{'target/dev/rc003-three-key/native'})
$nativeSource=Join-Path $root $(if($SourceB){'Testing/rc003_wdf_pdo_source.c'}else{'native/hid-host-helper/main.c'})
$scriptSource=Join-Path $root $(if($SourceB){'Testing/rc003_wdf_pdo_source.js'}else{'native/hid-host-helper/runtime.js'})
$executable=Join-Path $out $(if($SourceB){'rc003-wdf-pdo-source.exe'}else{'sayall-hid-host-helper.exe'})
if($TestLifecycle){
 $out=Join-Path $root 'target/dev/rc003-three-key/controller-test'
 $nativeSource=Join-Path $root 'native/hid-host-helper/lifecycle.test.c'
 $executable=Join-Path $out 'lifecycle-test.exe'
}
[IO.Directory]::CreateDirectory($out) | Out-Null
$source=[IO.File]::ReadAllText($scriptSource)
$literals=for($offset=0;$offset -lt $source.Length;$offset+=1024){
 ConvertTo-Json -InputObject $source.Substring($offset,[Math]::Min(1024,$source.Length-$offset)) -Compress
}
[IO.File]::WriteAllText((Join-Path $out 'runtime_script.h'),"static const char SAYALL_RUNTIME_SCRIPT[] =`n"+($literals -join "`n")+";`n")
$vs=& 'C:/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe' -latest -products '*' -property installationPath
$vc=Get-ChildItem -LiteralPath (Join-Path $vs 'VC/Tools/MSVC') -Directory | Sort-Object Name -Descending | Select-Object -First 1
$sdk='C:/Program Files (x86)/Windows Kits/10'
$compiler=Join-Path $vc.FullName 'bin/Hostx64/x64/cl.exe'
$includes=@("/I$out","/I$devkit","/I$($vc.FullName)/include","/I$sdk/Include/10.0.26100.0/ucrt","/I$sdk/Include/10.0.26100.0/shared","/I$sdk/Include/10.0.26100.0/um")
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
 $directory=[IO.DirectoryInfo]::new($devkit)
 while($null -ne $directory){$ancestors.Insert(0,$directory.FullName);$directory=$directory.Parent}
 foreach($directory in $ancestors){$locks.Add([SayAllBuildInputLock]::Directory($directory))}
 foreach($name in $members.Keys){
  $path=Join-Path $devkit $name
  if(([IO.File]::GetAttributes($path) -band [IO.FileAttributes]::ReparsePoint) -ne 0){throw 'Reparse member rejected'}
  $file=[IO.File]::Open($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
  $locks.Add($file)
  if([Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($file)).ToLowerInvariant() -ne $members[$name]){throw "Official build member mismatch: $name"}
 }

& $compiler /nologo /W3 /TC /MT /O2 /utf-8 @includes $nativeSource "/Fo$out/helper.obj" "/Fe$executable" /link /SUBSYSTEM:WINDOWS /ENTRY:wmainCRTStartup /DEPENDENTLOADFLAG:0x800 /DYNAMICBASE /NXCOMPAT "/LIBPATH:$devkit" "/LIBPATH:$($vc.FullName)/lib/x64" "/LIBPATH:$sdk/Lib/10.0.26100.0/ucrt/x64" "/LIBPATH:$sdk/Lib/10.0.26100.0/um/x64" cfgmgr32.lib wintrust.lib setupapi.lib hid.lib onecore.lib authz.lib shell32.lib > (Join-Path $out 'build.log') 2>&1
$code=$LASTEXITCODE
if($code -ne 0){Get-Content -LiteralPath (Join-Path $out 'build.log') -Tail 25}
else { [pscustomobject]@{result='built';sha256=(Get-FileHash -LiteralPath $executable).Hash.ToLowerInvariant()} | ConvertTo-Json -Compress }
} finally { for($i=$locks.Count-1;$i -ge 0;$i--){$locks[$i].Dispose()} }
exit $code
