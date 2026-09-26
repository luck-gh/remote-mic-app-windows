param([switch]$TestsOnly)
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$wdk = Join-Path $root 'target/wdk-nuget/10.0.26100.6584/c'
$sdk = 'C:/Program Files (x86)/Windows Kits/10'
$vswhere = 'C:/Program Files (x86)/Microsoft Visual Studio/Installer/vswhere.exe'
$vs = & $vswhere -latest -products '*' -property installationPath
$vc = Get-ChildItem -LiteralPath (Join-Path $vs 'VC/Tools/MSVC') -Directory | Sort-Object Name -Descending | Select-Object -First 1
$compiler = Join-Path $vc.FullName 'bin/Hostx64/x64/cl.exe'
$linker = Join-Path $vc.FullName 'bin/Hostx64/x64/link.exe'
$out = Join-Path $root 'target/input-driver'
New-Item -ItemType Directory -Force $out | Out-Null
$src = Join-Path $root 'drivers/SayAllInput'
$inc = @("/I$($vc.FullName)/include", "/I$sdk/Include/10.0.26100.0/ucrt")
& $compiler /nologo /W4 /WX /TC @inc "$src/input_state.c" "$src/tests/input_state_test.c" "/Fo$out/" "/Fe$out/input_state_test.exe" /link "/LIBPATH:$($vc.FullName)/lib/x64" "/LIBPATH:$sdk/Lib/10.0.26100.0/ucrt/x64" "/LIBPATH:$sdk/Lib/10.0.26100.0/um/x64" > "$out/state-build.log" 2>&1
if ($LASTEXITCODE -ne 0) { Get-Content "$out/state-build.log" -Tail 35; exit $LASTEXITCODE }
& "$out/input_state_test.exe" > "$out/state-test.log" 2>&1
if ($LASTEXITCODE -ne 0) { Get-Content "$out/state-test.log" -Tail 35; exit $LASTEXITCODE }
Get-Content "$out/state-test.log"
if ($TestsOnly) { exit 0 }
if (!(Test-Path "$wdk/Include/10.0.26100.0/km/ntddk.h")) { throw 'Locked WDK NuGet 10.0.26100.6584 is unavailable.' }
$kernel = @('/nologo','/c','/W4','/WX','/TC','/kernel','/GS','/guard:cf','/D_AMD64_','/DAMD64','/D_WIN64','/D_WIN32_WINNT=0x0A00','/DNTDDI_VERSION=0x0A000008','/DKMDF_VERSION_MAJOR=1','/DKMDF_VERSION_MINOR=15',"/I$($vc.FullName)/include","/I$wdk/Include/10.0.26100.0/km","/I$wdk/Include/wdf/kmdf/1.15","/I$sdk/Include/10.0.26100.0/shared","/I$sdk/Include/10.0.26100.0/ucrt")
& $compiler @kernel "$src/driver.c" "$src/input_state.c" "/Fo$out/" > "$out/kernel-build.log" 2>&1
if ($LASTEXITCODE -ne 0) { Get-Content "$out/kernel-build.log" -Tail 45; exit $LASTEXITCODE }
& $linker /nologo /driver /subsystem:native,10.00 /entry:FxDriverEntry /nodefaultlib /machine:x64 /dynamicbase /nxcompat /guard:cf /integritycheck "/out:$out/SayAllInput.sys" "$out/driver.obj" "$out/input_state.obj" "/LIBPATH:$wdk/Lib/10.0.26100.0/km/x64" "/LIBPATH:$wdk/Lib/wdf/kmdf/x64/1.15" ntoskrnl.lib hal.lib wmilib.lib hidparse.lib BufferOverflowFastFailK.lib WdfLdr.lib WdfDriverEntry.lib > "$out/kernel-link.log" 2>&1
if ($LASTEXITCODE -ne 0) { Get-Content "$out/kernel-link.log" -Tail 35; exit $LASTEXITCODE }
Copy-Item -LiteralPath "$src/SayAllInput.inf" -Destination $out
Write-Output 'kernel_build=passed signature=missing installation=not_attempted hardware=deferred'
