$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$installer=Get-Content -Raw (Join-Path $root 'src-tauri/windows/installer.nsi')
$hooks=Get-Content -Raw (Join-Path $root 'src-tauri/windows/installer-hooks.nsh')
foreach($field in @('FileVersion','ProductVersion')){
    if(-not $installer.Contains('VIAddVersionKey "'+$field+'" "${VERSIONWITHBUILD}"')){throw ($field+' must use the public four-part version')}
}
if(-not $installer.Contains('"DisplayVersion" "${VERSIONWITHBUILD}"')){throw 'Installed application list must use the public version'}
if(-not $installer.Contains('"SemverVersion" "${VERSION}"')){throw 'Installer must store internal comparison version separately'}
if(-not $hooks.Contains('ReadRegStr $R8 SHCTX "${UNINSTKEY}" "SemverVersion"')){throw 'Comparison must read its internal version field'}
'passed: installer file metadata and installed application list use public versions; comparison value is separate'
