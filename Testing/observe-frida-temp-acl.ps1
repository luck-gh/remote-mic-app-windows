param([int]$Seconds=35)
$ErrorActionPreference='Stop'
if($Seconds -lt 1 -or $Seconds -gt 45){throw 'Bounded observer only'}
$destination=Join-Path (Split-Path -Parent $PSScriptRoot) 'artifacts/hid-host-three-key-20260915/agent-temp-acl-observation.json'
$watcher=[IO.FileSystemWatcher]::new([IO.Path]::GetTempPath(),'frida-agent.dll')
$watcher.IncludeSubdirectories=$true
$watcher.EnableRaisingEvents=$true
$source='SayAllFixedFridaAgentObservation'
$subscription=Register-ObjectEvent -InputObject $watcher -EventName Created -SourceIdentifier $source
$observed=@()
$started=Get-Date
try {
 while(((Get-Date)-$started).TotalSeconds -lt $Seconds){
  $event=Wait-Event -SourceIdentifier $source -Timeout 1
  if($null -eq $event){continue}
  $path=$event.SourceEventArgs.FullPath
  Remove-Event -EventIdentifier $event.EventIdentifier
  # Paths remain in memory. Read ACLs only; do not hold or modify the payload.
  $levels=@()
  for($depth=0;$depth -lt 3;$depth++){
   try {
    $acl=Get-Acl -LiteralPath $path
    $aces=@($acl.Access | ForEach-Object {
     try {$sid=$_.IdentityReference.Translate([Security.Principal.SecurityIdentifier]).Value} catch {$sid='unknown'}
     $label=switch($sid){'S-1-1-0'{'Everyone'} 'S-1-5-11'{'AuthenticatedUsers'} 'S-1-5-19'{'LocalService'} 'S-1-5-32-545'{'Users'} 'S-1-15-2-1'{'AllAppPackages'} 'S-1-5-18'{'System'} 'S-1-5-32-544'{'Administrators'} default {'OtherRedacted'}}
     @{principal=$label;rights=$_.FileSystemRights.ToString();type=$_.AccessControlType.ToString();inherited=$_.IsInherited}
    })
    $levels+=@{depth=$depth;aclKnown=$true;aces=$aces}
   }catch {$levels+=@{depth=$depth;aclKnown=$false}}
   $path=Split-Path -Parent $path
  }
  $observed+=@{at=(Get-Date).ToString('o');basename='frida-agent.dll';levels=$levels}
 }
} finally {
 Unregister-Event -SourceIdentifier $source
 if($null -ne $subscription){Remove-Job -Job $subscription -Force}
 $watcher.Dispose()
 @{started=$started.ToString('o');finished=(Get-Date).ToString('o');observations=$observed;securityChanged=$false;pathsRedacted=$true}|ConvertTo-Json -Depth 9|Set-Content -LiteralPath $destination
}
Write-Output "agent_acl_observer=finished observations=$($observed.Count)"
