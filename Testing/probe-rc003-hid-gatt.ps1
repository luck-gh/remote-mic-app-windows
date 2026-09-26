# GPL-3.0-only. Ordinary-user WinRT metadata probe, fixed selected RC003.
# No CCCD writes, notifications, Report(2A4D) reads or MaintainConnection changes.
param([switch]$Win32)
$ErrorActionPreference='Stop'
$root=if($PSScriptRoot){Split-Path -Parent $PSScriptRoot}else{(Get-Location).Path}
$out=Join-Path $root 'artifacts/hid-gatt-access-20260919'
[IO.Directory]::CreateDirectory($out) | Out-Null
$log=Join-Path $out $(if($Win32){'win32-metadata.log'}else{'winrt-metadata.log'})
$summary=Join-Path $out $(if($Win32){'win32-result.json'}else{'winrt-result.json'})
$script:clock=[Diagnostics.Stopwatch]::StartNew()
$script:events=[Collections.Generic.List[object]]::new()
$script:ownServices=[Collections.Generic.List[object]]::new()
$script:device=$null;$script:stage='initialize';$script:runId=[Guid]::NewGuid().ToString('N')
function Note([string]$phase,$fields=@{}) {
 $entry=[ordered]@{runId=$script:runId;at=(Get-Date).ToString('o');elapsedMs=$script:clock.ElapsedMilliseconds;phase=$phase}
 foreach($k in $fields.Keys){$entry[$k]=$fields[$k]};$script:events.Add($entry)
 [IO.File]::AppendAllText($log,($entry | ConvertTo-Json -Compress)+[Environment]::NewLine,[Text.UTF8Encoding]::new($false))
}
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
using System.Security.Principal;
public static class SayAllGattIdentity {
 [DllImport("kernel32.dll")] static extern int GetCurrentPackageFullName(ref uint size,StringBuilder name);
 [DllImport("kernel32.dll")] static extern int GetPackageFullName(IntPtr process,ref uint size,StringBuilder name);
 [DllImport("kernel32.dll",SetLastError=true)] static extern IntPtr OpenProcess(uint access,bool inherit,int pid);
 [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
 [DllImport("advapi32.dll",SetLastError=true)] static extern bool OpenProcessToken(IntPtr process,uint access,out IntPtr token);
 [DllImport("advapi32.dll",SetLastError=true)] static extern bool GetTokenInformation(IntPtr token,int cls,out int value,int bytes,out int written);
 public static int Package(int pid) {uint size=0;if(pid==0)return GetCurrentPackageFullName(ref size,null);var h=OpenProcess(0x1000,false,pid);if(h==IntPtr.Zero)return -Marshal.GetLastWin32Error();try{return GetPackageFullName(h,ref size,null);}finally{CloseHandle(h);}}
 public static bool Elevated() {int value,bytes;if(!GetTokenInformation(WindowsIdentity.GetCurrent().Token,20,out value,4,out bytes))throw new System.ComponentModel.Win32Exception();return value!=0;}
 public static bool SameUser(int pid) {var h=OpenProcess(0x1000,false,pid);if(h==IntPtr.Zero)return false;IntPtr token=IntPtr.Zero;try{if(!OpenProcessToken(h,8,out token))return false;using(var identity=new WindowsIdentity(token)){return identity.User.Equals(WindowsIdentity.GetCurrent().User);}}finally{if(token!=IntPtr.Zero)CloseHandle(token);CloseHandle(h);}}
}
"@
[Windows.Devices.Bluetooth.BluetoothLEDevice,Windows.Devices.Bluetooth,ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Bluetooth.BluetoothCacheMode,Windows.Devices.Bluetooth,ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Bluetooth.GenericAttributeProfile.GattDeviceService,Windows.Devices.Bluetooth,ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Bluetooth.GenericAttributeProfile.GattDeviceServicesResult,Windows.Devices.Bluetooth,ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Bluetooth.GenericAttributeProfile.GattCharacteristicsResult,Windows.Devices.Bluetooth,ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Bluetooth.GenericAttributeProfile.GattDescriptorsResult,Windows.Devices.Bluetooth,ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Bluetooth.GenericAttributeProfile.GattReadResult,Windows.Devices.Bluetooth,ContentType=WindowsRuntime] | Out-Null
[Windows.Devices.Enumeration.DeviceAccessStatus,Windows.Devices.Enumeration,ContentType=WindowsRuntime] | Out-Null
[Windows.Storage.Streams.DataReader,Windows.Storage.Streams,ContentType=WindowsRuntime] | Out-Null
Add-Type -AssemblyName System.Runtime.WindowsRuntime
$script:asTask=@([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {$_.Name -eq 'AsTask' -and $_.IsGenericMethod -and $_.GetGenericArguments().Count -eq 1 -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1'})[0]
function Await($operation,[Type]$resultType,[string]$phase) {
 $script:stage=$phase;$remaining=90000-$script:clock.ElapsedMilliseconds
 if($remaining -le 0){try{$operation.Cancel()}catch{};throw [TimeoutException]::new('budget')}
 $watch=[Diagnostics.Stopwatch]::StartNew();Note ($phase+'_begin')
 $task=$script:asTask.MakeGenericMethod($resultType).Invoke($null,@($operation))
 if(-not $task.Wait([int][Math]::Min(10000,$remaining))) {
  try{$operation.Cancel();Note ($phase+'_cancel_requested')}catch{Note ($phase+'_cancel_failed')}
  Note ($phase+'_timeout') @{cleanupConfirmed=$false}
  throw [TimeoutException]::new('async_timeout') # Never read Result after timeout.
 }
 Note ($phase+'_completed') @{durationMs=$watch.ElapsedMilliseconds};return $task.Result
}
function Status($result,[string]$phase) {
 $errorCode=$null;if($null -ne $result.ProtocolError){$errorCode=[int]$result.ProtocolError}
 Note $phase @{status=$result.Status.ToString();protocolError=$errorCode}
 if($result.Status.ToString() -ne 'Success'){throw [InvalidOperationException]::new('gatt_status')}
}
function ReadStatic($object,[string]$phase,[int]$maximum) {
 $result=Await ($object.ReadValueAsync([Windows.Devices.Bluetooth.BluetoothCacheMode]::Uncached)) ([Windows.Devices.Bluetooth.GenericAttributeProfile.GattReadResult]) $phase
 Status $result ($phase+'_status');$length=[int]$result.Value.Length
 if($length -lt 1 -or $length -gt $maximum){Note ($phase+'_shape_rejected') @{length=$length};throw [InvalidOperationException]::new('metadata_length')}
 $bytes=[byte[]]::new($length);$reader=[Windows.Storage.Streams.DataReader]::FromBuffer($result.Value)
 try{$reader.ReadBytes($bytes)}finally{$reader.Dispose()};return ,$bytes
}
$exitCode=1;$settingsPath=Join-Path $env:APPDATA 'app.getsayall.remote-mic.windows/settings.json';$before=$null
try {
 $session=[Diagnostics.Process]::GetCurrentProcess().SessionId
 $explorers=@(Get-Process explorer -ErrorAction SilentlyContinue | Where-Object {$_.SessionId -eq $session -and [SayAllGattIdentity]::SameUser($_.Id)})
 $elevated=[SayAllGattIdentity]::Elevated();$package=[SayAllGattIdentity]::Package(0)
 $apps=@(Get-Process sayall-windows-app -ErrorAction SilentlyContinue | Where-Object {$_.SessionId -eq $session -and [SayAllGattIdentity]::SameUser($_.Id) -and $_.Path -eq 'D:\Program Files\无线麦 SayAll\sayall-windows-app.exe'})
 Note 'identity' @{pid=$PID;session=$session;elevated=$elevated;sameExplorerUser=($explorers.Count -gt 0);packageQuery=$package;apartment=[Threading.Thread]::CurrentThread.GetApartmentState().ToString();appCount=$apps.Count;appPackageQuery=$(if($apps.Count -eq 1){[SayAllGattIdentity]::Package($apps[0].Id)}else{$null})}
 if($elevated -or $explorers.Count -eq 0 -or $package -ne 15700 -or $apps.Count -ne 1){throw [InvalidOperationException]::new('identity_gate')}
 $script:stage='app_ready_gate'
 $diagnostic=Join-Path $env:LOCALAPPDATA 'SayAll/Logs/sayall-diagnostic.log'
 $states=@(Get-Content -LiteralPath $diagnostic -Tail 2500 | Where-Object {$_ -match ('\bpid='+$apps[0].Id+'\b') -and $_ -match 'input_context_sync phase='})
 $ready=($states.Count -gt 0 -and $states[-1] -match 'input_context_sync phase=ready model=rc003 connected=true')
 Note 'app_ready_gate' @{readyObserved=$ready}
 if(-not $ready){throw [InvalidOperationException]::new('app_not_ready')}
 $before=(Get-FileHash -LiteralPath $settingsPath -Algorithm SHA256).Hash
 $settings=[IO.File]::ReadAllText($settingsPath) | ConvertFrom-Json;$selected=[string]$settings.selected_remote_id
 if([string]::IsNullOrWhiteSpace($selected)){throw [InvalidOperationException]::new('selection_missing')}
 $script:device=Await ([Windows.Devices.Bluetooth.BluetoothLEDevice]::FromIdAsync($selected)) ([Windows.Devices.Bluetooth.BluetoothLEDevice]) 'device_open'
 if($null -eq $script:device -or -not [string]::Equals($script:device.DeviceId,$selected,[StringComparison]::OrdinalIgnoreCase)){throw [InvalidOperationException]::new('selection_mismatch')}
 Note 'device_selected' @{exactDeviceId=$true;connected=($script:device.ConnectionStatus.ToString() -eq 'Connected')}
 if($script:device.ConnectionStatus.ToString() -ne 'Connected'){throw [InvalidOperationException]::new('device_disconnected')}
 Note 'device_access_current' @{status=$script:device.DeviceAccessInformation.CurrentStatus.ToString()}
 $access=Await ($script:device.RequestAccessAsync()) ([Windows.Devices.Enumeration.DeviceAccessStatus]) 'device_access'
 Note 'device_access_status' @{access=$access.ToString()};if($access.ToString() -ne 'Allowed'){throw [UnauthorizedAccessException]::new('device_access_denied')}
 $hid=[Guid]'00001812-0000-1000-8000-00805f9b34fb';$cache=[Windows.Devices.Bluetooth.BluetoothCacheMode]::Uncached
 $services=Await ($script:device.GetGattServicesForUuidAsync($hid,$cache)) ([Windows.Devices.Bluetooth.GenericAttributeProfile.GattDeviceServicesResult]) 'hid_services'
 foreach($service in $services.Services){$script:ownServices.Add($service)}
 Status $services 'hid_services_status';Note 'hid_services_count' @{count=$services.Services.Count}
 if($services.Services.Count -ne 1){throw [InvalidOperationException]::new('hid_not_unique')}
 $service=$services.Services[0]
 # This service originates from the exact selected device. Its public interface
 # identity must also contain the researched RC003 HID revision, never a host guess.
 $revision=([string]$service.DeviceId -match '(?i)dev_vid&012717_pid&32b8_rev&00a4')
 Note 'hid_identity' @{selectedDeviceService=$true;uuid1812=($service.Uuid -eq $hid);rc003Revision=$revision}
 if($service.Uuid -ne $hid -or -not $revision){throw [InvalidOperationException]::new('hid_identity_unverified')}
 if($Win32) {
  $script:stage='win32_metadata'
  $native=Join-Path $out 'hid-gatt-metadata.exe'
  $nativeLock=[IO.File]::Open($native,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
  $process=$null
  try {
   $hasher=[Security.Cryptography.SHA256]::Create()
   try{$nativeHash=[BitConverter]::ToString($hasher.ComputeHash($nativeLock)).Replace('-','').ToLowerInvariant()}finally{$hasher.Dispose()}
   if($nativeHash -ne 'a74390b08eea3e2333dcb7649e61a5102a1997e890536663ba21c44b22604fc3'){throw [InvalidOperationException]::new('native_pin')}
   $info=[Diagnostics.ProcessStartInfo]::new($native);$info.UseShellExecute=$false;$info.CreateNoWindow=$true
   $info.RedirectStandardInput=$true;$info.RedirectStandardOutput=$true;$info.RedirectStandardError=$true
   $info.StandardInputEncoding=[Text.UnicodeEncoding]::new($false,$false)
   $process=[Diagnostics.Process]::new();$process.StartInfo=$info
   if(-not $process.Start()){throw [InvalidOperationException]::new('native_start')}
   $stdout=$process.StandardOutput.ReadToEndAsync();$stderr=$process.StandardError.ReadToEndAsync()
   $process.StandardInput.WriteLine([string]$service.DeviceId);$process.StandardInput.Close()
   Note 'win32_started' @{pid=$process.Id;ordinaryUser=$true;readOnly=$true}
   if(-not $process.WaitForExit(35000)){
    Note 'win32_pending' @{pid=$process.Id;executable='hid-gatt-metadata.exe';sha256=$nativeHash;cleanupConfirmed=$false;forcedTermination=$false;softObservationDeadline=$true}
    # Public synchronous APIs have no hard cancellation contract. Keep the one
    # owned process/reference under observation; never spawn another or kill it.
    while(-not $process.WaitForExit(5000)){}
    Note 'win32_pending_returned' @{pid=$process.Id;cleanupConfirmed=$true}
   }
   # Output is fixed anonymous native JSON; no raw stderr/paths are persisted.
   foreach($line in ($stdout.Result -split "`r?`n")){if($line){$item=$line | ConvertFrom-Json;Note ('native_'+$item.stage) @{hresult=$item.hresult;count=$item.count;nativeElapsedMs=$item.elapsedMs}}}
   Note 'win32_exit' @{exit=$process.ExitCode;stderrPresent=($stderr.Result.Length -gt 0)}
   if($process.ExitCode -ne 0){throw [InvalidOperationException]::new('native_failed')}
  } finally {if($process){$process.Dispose()};$nativeLock.Dispose()}
 } else {
 $access=Await ($service.RequestAccessAsync()) ([Windows.Devices.Enumeration.DeviceAccessStatus]) 'service_access'
 Note 'service_access_status' @{access=$access.ToString()};if($access.ToString() -ne 'Allowed'){throw [UnauthorizedAccessException]::new('service_access_denied')}
 $reports=Await ($service.GetCharacteristicsForUuidAsync([Guid]'00002a4d-0000-1000-8000-00805f9b34fb',$cache)) ([Windows.Devices.Bluetooth.GenericAttributeProfile.GattCharacteristicsResult]) 'report_characteristics'
 Status $reports 'report_characteristics_status';Note 'report_count' @{count=$reports.Characteristics.Count}
 if($reports.Characteristics.Count -gt 32){throw [InvalidOperationException]::new('report_limit')};$ordinal=0
 foreach($report in $reports.Characteristics) {
  $ordinal++;Note 'report_metadata' @{ordinal=$ordinal;properties=[int]$report.CharacteristicProperties}
  $refs=Await ($report.GetDescriptorsForUuidAsync([Guid]'00002908-0000-1000-8000-00805f9b34fb',$cache)) ([Windows.Devices.Bluetooth.GenericAttributeProfile.GattDescriptorsResult]) 'report_references'
  Status $refs 'report_references_status';Note 'report_reference_count' @{ordinal=$ordinal;count=$refs.Descriptors.Count}
  if($refs.Descriptors.Count -gt 1){throw [InvalidOperationException]::new('reference_not_unique')}
  foreach($reference in $refs.Descriptors){$bytes=ReadStatic $reference 'report_reference_read' 2;if($bytes.Length -ne 2){throw [InvalidOperationException]::new('reference_shape')};Note 'report_reference' @{ordinal=$ordinal;reportId=[int]$bytes[0];reportType=[int]$bytes[1]}}
 }
 $maps=Await ($service.GetCharacteristicsForUuidAsync([Guid]'00002a4b-0000-1000-8000-00805f9b34fb',$cache)) ([Windows.Devices.Bluetooth.GenericAttributeProfile.GattCharacteristicsResult]) 'report_maps'
 Status $maps 'report_maps_status';Note 'report_map_count' @{count=$maps.Characteristics.Count}
 if($maps.Characteristics.Count -gt 1){throw [InvalidOperationException]::new('map_not_unique')}
 foreach($map in $maps.Characteristics){$bytes=ReadStatic $map 'report_map_read' 65536;$hasher=[Security.Cryptography.SHA256]::Create();try{$digest=[BitConverter]::ToString($hasher.ComputeHash($bytes)).Replace('-','').ToLowerInvariant()}finally{$hasher.Dispose()};Note 'report_map_metadata' @{length=$bytes.Length;sha256=$digest}}
 }
 $exitCode=0
} catch {
 $errorObject=$_.Exception;while($errorObject.InnerException){$errorObject=$errorObject.InnerException}
 $category=if($errorObject.Message -match '^(native_pin|native_start|native_timeout|native_failed|app_not_ready|budget|async_timeout|gatt_status|metadata_length|identity_gate|selection_missing|selection_mismatch|device_disconnected|device_access_denied|hid_not_unique|hid_identity_unverified|service_access_denied|report_limit|reference_not_unique|reference_shape|map_not_unique)$'){$errorObject.Message}else{'api_exception'}
 Note 'failure' @{category=$category;stage=$script:stage;exceptionType=$errorObject.GetType().Name;hresult=('0x{0:x8}' -f ($errorObject.HResult -band 0xffffffffL))}
} finally {
 $released=0
 foreach($service in $script:ownServices){try{$service.Dispose();$released++}catch{Note 'service_dispose_failed' @{hresult=$_.Exception.HResult};$exitCode=1}}
 if($null -ne $script:device){try{$script:device.Dispose();Note 'own_device_disposed'}catch{Note 'device_dispose_failed' @{hresult=$_.Exception.HResult};$exitCode=1}}
 $same=$null;if($before){try{$same=($before -eq (Get-FileHash -LiteralPath $settingsPath -Algorithm SHA256).Hash)}catch{Note 'configuration_hash_unavailable' @{hresult=$_.Exception.HResult}}}
 Note 'terminal' @{exit=$exitCode;ownServicesDisposed=$released;configurationUnchanged=$same;cccdWrites=0;notificationSubscriptions=0;reportValueReads=0;otherDeviceReferencesClosed=0;physicalConnectionUnchangedProven=$false}
 @{exit=$exitCode;events=$script:events} | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath $summary -Encoding UTF8
}
exit $exitCode
