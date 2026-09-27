$ErrorActionPreference='Stop'
$root=Split-Path -Parent $PSScriptRoot
$out=Join-Path $root 'artifacts/hid-gatt-access-20260919'
$vswhere='C:/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe'
$vs=& $vswhere -latest -products '*' -property installationPath
$vc=Get-ChildItem -LiteralPath (Join-Path $vs 'VC/Tools/MSVC') -Directory | Sort-Object Name -Descending | Select-Object -First 1
$sdk='C:/Program Files (x86)/Windows Kits/10'
$compiler=Join-Path $vc.FullName 'bin/Hostx64/x64/cl.exe'
$includes=@("/I$out","/I$($vc.FullName)/include","/I$sdk/Include/10.0.26100.0/ucrt","/I$sdk/Include/10.0.26100.0/shared","/I$sdk/Include/10.0.26100.0/um")
& $compiler /nologo /W3 /TC /MT /O2 /utf-8 @includes "$root/Testing/hid_gatt_metadata.c" "/Fo$out/hid_gatt_metadata.obj" "/Fe$out/hid-gatt-metadata.exe" /link /DEPENDENTLOADFLAG:0x800 /DYNAMICBASE /NXCOMPAT "/LIBPATH:$($vc.FullName)/lib/x64" "/LIBPATH:$sdk/Lib/10.0.26100.0/ucrt/x64" "/LIBPATH:$sdk/Lib/10.0.26100.0/um/x64" BluetoothApis.lib setupapi.lib onecore.lib advapi32.lib > (Join-Path $out 'win32-build.log') 2>&1
if($LASTEXITCODE -ne 0){Get-Content (Join-Path $out 'win32-build.log') -Tail 20;exit $LASTEXITCODE}
Write-Output 'win32_build=passed device_calls=not_started'
