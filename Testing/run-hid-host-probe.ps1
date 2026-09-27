param([ValidateSet('inspect','prepare','capture')][string]$Mode='inspect')
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$exe=Join-Path $root 'target/frida-17.15.3/sayall-hid-host-probe.exe'
$evidence=Join-Path $root 'artifacts/hid-host-future-open-20260919'
$expected='5add917c65f070f0a9bc49b4b8ed4736fea31dab8515b18a68da3e65cef8896c'
Add-Type @'
using System;
using System.IO;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;
public static class SayAllProbePathLock {
 [DllImport("kernel32.dll",CharSet=CharSet.Unicode,SetLastError=true)] static extern SafeFileHandle CreateFileW(string path,uint access,uint share,IntPtr sa,uint disposition,uint flags,IntPtr template);
 public static SafeFileHandle Directory(string path) {
  if((File.GetAttributes(path)&FileAttributes.ReparsePoint)!=0)throw new IOException("Reparse directory rejected");
  var file=CreateFileW(path,0x80,1,IntPtr.Zero,3,0x02200000,IntPtr.Zero);
  if(file.IsInvalid){file.Dispose();throw new IOException("Cannot lock directory");}
  if((File.GetAttributes(path)&FileAttributes.ReparsePoint)!=0){file.Dispose();throw new IOException("Directory changed");}
  return file;
 }
}
'@
$locks=[Collections.Generic.List[IDisposable]]::new()
try {
 $ancestors=[Collections.Generic.List[string]]::new()
 $directory=[IO.DirectoryInfo]::new([IO.Path]::GetDirectoryName($exe))
 while($null -ne $directory){$ancestors.Insert(0,$directory.FullName);$directory=$directory.Parent}
 foreach($directory in $ancestors){$locks.Add([SayAllProbePathLock]::Directory($directory))}
 if(([IO.File]::GetAttributes($exe) -band [IO.FileAttributes]::ReparsePoint) -ne 0){throw 'Reparse executable rejected'}
 $file=[IO.File]::Open($exe,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
 $locks.Add($file)
 $hash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($file)).ToLowerInvariant()
 if($hash -ne $expected){throw 'Frozen probe hash mismatch'}
 $start=Get-Date
 $runId=[Guid]::NewGuid().ToString('N')
 $process=Start-Process -FilePath $exe -ArgumentList "--$Mode $runId" -Verb RunAs -WindowStyle Hidden -PassThru
 # The probe owns its capture/cleanup timers. Never force termination on timeout.
 $limit=if($Mode -eq 'inspect'){30000}elseif($Mode -eq 'capture'){190000}else{190000}
 $finished=$process.WaitForExit($limit)
 $events=@(Get-WinEvent -LogName Application -FilterXPath "*[System[Provider[@Name='SayAllInput'] and EventID=1002]]" -MaxEvents 500 -ErrorAction SilentlyContinue | Where-Object { $_.TimeCreated -ge $start -and $_.Properties[0].Value.StartsWith("hid_host_probe run_id=$runId probe_pid=$($process.Id) ") } | Sort-Object TimeCreated | ForEach-Object {@{at=$_.TimeCreated.ToString('o');message=$_.Properties[0].Value}})
 @{runId=$runId;startedAt=$start.ToString('o');finished=$finished;mode=$Mode;probePid=$process.Id;nativeExit=$(if($finished){$process.ExitCode}else{$null});events=$events} | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 (Join-Path $evidence "$Mode-result.json")
 if(!$finished){throw 'Probe has not exited within the bounded wait; left untouched.'}
 Write-Output "probe_mode=$Mode exit=$($process.ExitCode) event_count=$($events.Count)"
 exit $process.ExitCode
} finally {for($i=$locks.Count-1;$i -ge 0;$i--){$locks[$i].Dispose()}}
