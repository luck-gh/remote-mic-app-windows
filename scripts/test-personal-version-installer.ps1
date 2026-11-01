# Execute the production version-check macro with registry reads replaced at the
# OS boundary. No product registry keys, installed apps or capture state are touched.
$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$fixture=Join-Path $root 'target/dev/personal-version/installer-check'
New-Item -ItemType Directory -Force $fixture|Out-Null
$rendered=Get-Content -Raw (Join-Path $root 'target/release/nsis/x64/installer.nsi')
$plugins=[regex]::Match($rendered,'(?m)^!define ADDITIONALPLUGINSPATH "([^"]+)"').Groups[1].Value
if(-not $plugins){throw 'A preceding Tauri bundle is required to locate the fixed NSIS plugin'}
$compiler=[IO.Path]::GetFullPath((Join-Path $plugins '../../../makensis.exe'))
$hooks=Get-Content -Raw (Join-Path $root 'src-tauri/windows/installer-hooks.nsh')
$hooks=$hooks.Replace('ReadRegStr $R8 SHCTX "${UNINSTKEY}" "SemverVersion"',@'
StrCpy $R8 $FixtureInternal
${If} $FixturePresent == 1
  ClearErrors
${Else}
  SetErrors
${EndIf}
'@).Replace('ReadRegStr $R8 SHCTX "${UNINSTKEY}" "DisplayVersion"','StrCpy $R8 $FixtureLegacy').Replace('EnumRegValue $R9 SHCTX "${UNINSTKEY}" $R7',@'
${If} $FixturePresent == 0
  StrCpy $R9 ""
  SetErrors
${Else}
  StrCpy $R9 "SemverVersion"
  ClearErrors
${EndIf}
'@)
$hookPath=Join-Path $fixture 'hooks.nsh'
[IO.File]::WriteAllText($hookPath,$hooks,[Text.UTF8Encoding]::new($false))
$script=Join-Path $fixture 'check.nsi'
$executable=Join-Path $fixture 'check.exe'
$text=@'
Unicode true
SilentInstall silent
RequestExecutionLevel user
!include LogicLib.nsh
!include FileFunc.nsh
!addplugindir "__PLUGINS__"
!define VERSION "$FixtureCurrent"
!define UNINSTKEY "Unused-Fixture-Key"
!include "__HOOKS__"
OutFile "__OUTPUT__"
Var FixtureCurrent
Var FixtureInternal
Var FixtureLegacy
Var FixturePresent
Var Scenario
Function .onInit
  ${GetOptions} $CMDLINE "/CASE=" $Scenario
  StrCpy $SayAllInstallResultDirectory "$EXEDIR\logs"
  StrCpy $FixtureCurrent "1.0.0+10"
  StrCpy $FixtureInternal "1.0.0+9"
  StrCpy $FixtureLegacy "1.0.0.9"
  StrCpy $FixturePresent 1
  ${If} $Scenario == legacy
    StrCpy $FixtureCurrent "1.0.0+1"
    StrCpy $FixtureInternal ""
    StrCpy $FixtureLegacy "1.0.0"
    StrCpy $FixturePresent 0
  ${ElseIf} $Scenario == fresh
    StrCpy $FixtureInternal ""
    StrCpy $FixtureLegacy ""
    StrCpy $FixturePresent 0
  ${ElseIf} $Scenario == downgrade
    StrCpy $FixtureInternal "1.0.0+11"
  ${ElseIf} $Scenario == same
    StrCpy $FixtureInternal "1.0.0+10"
  ${ElseIf} $Scenario == invalid
    StrCpy $FixtureInternal "broken"
  ${ElseIf} $Scenario == empty
    StrCpy $FixtureInternal ""
  ${ElseIf} $Scenario == public-in-internal
    StrCpy $FixtureInternal "1.0.0.9"
  ${ElseIf} $Scenario == unreadable-present
    StrCpy $FixtureInternal ""
    StrCpy $FixturePresent 2
  ${EndIf}
  !insertmacro SayAllValidateUpgrade
FunctionEnd
Section
  FileOpen $R0 "$EXEDIR\$Scenario.passed" w
  FileWrite $R0 "validated"
  FileClose $R0
  SetErrorLevel 0
SectionEnd
'@.Replace('__PLUGINS__',$plugins).Replace('__HOOKS__',$hookPath).Replace('__OUTPUT__',$executable)
[IO.File]::WriteAllText($script,$text,[Text.UTF8Encoding]::new($false))
& $compiler /INPUTCHARSET UTF8 /V1 $script
if($LASTEXITCODE -ne 0){throw 'Personal version fixture failed to compile'}
$results=@()
foreach($case in @(@('legacy',0),@('fresh',0),@('upgrade',0),@('same',0),@('downgrade',1638),@('invalid',1603),@('empty',1603),@('public-in-internal',1603),@('unreadable-present',1603))){
    $marker=Join-Path $fixture ($case[0]+'.passed')
    $before=if(Test-Path $marker){(Get-Item $marker).LastWriteTimeUtc}else{[DateTime]::MinValue}
    $child=Start-Process -FilePath $executable -ArgumentList ('/CASE='+$case[0]) -PassThru -WindowStyle Hidden
    try{
        if(-not $child.WaitForExit(10000)){throw 'Version fixture timed out; no process was killed'}
        if($child.ExitCode -ne $case[1]){throw "Case $($case[0]) expected $($case[1]), observed $($child.ExitCode)"}
    }finally{$child.Dispose()}
    $after=if(Test-Path $marker){(Get-Item $marker).LastWriteTimeUtc}else{[DateTime]::MinValue}
    if(($case[1] -eq 0) -ne ($after -gt $before)){throw 'Version-check result and reachable section disagree'}
    $results+=@{scenario=$case[0];exitCode=$case[1];sectionReached=($after -gt $before)}
}
$results|ConvertTo-Json|Set-Content -Encoding utf8 (Join-Path $fixture 'result.json')
'passed: real NSIS version macro/plugin covers legacy upgrade, +9 to +10, repeat, downgrade rejection and invalid authoritative metadata'
