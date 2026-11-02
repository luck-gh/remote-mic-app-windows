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
$out = Join-Path $root 'target/dev/tv-keyboard-filter'
$src = Join-Path $root 'drivers/SayAllTvKeyboard'
New-Item -ItemType Directory -Force $out | Out-Null
$sourceFiles = @(Get-ChildItem -LiteralPath $src -Recurse -File | Where-Object Extension -In '.c', '.h') + @(Get-Item -LiteralPath $PSCommandPath)
$sourceEntries = @($sourceFiles | Sort-Object FullName | ForEach-Object {
    [ordered]@{ path = [IO.Path]::GetRelativePath($root, $_.FullName).Replace('\', '/'); sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash.ToLowerInvariant() }
})
$identityText = ($sourceEntries | ForEach-Object { "$($_.path):$($_.sha256)" }) -join "`n"
$fingerprint = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($identityText))).ToLowerInvariant()
Set-Content -LiteralPath "$out/build_identity.h" -Encoding ascii -Value "#define TV_SOURCE_FINGERPRINT `"$fingerprint`""
$userFlags = @('/nologo', '/W4', '/WX', '/TC', "/I$($vc.FullName)/include", "/I$sdk/Include/10.0.26100.0/ucrt")
$userLink = @('/link', "/LIBPATH:$($vc.FullName)/lib/x64", "/LIBPATH:$sdk/Lib/10.0.26100.0/ucrt/x64", "/LIBPATH:$sdk/Lib/10.0.26100.0/um/x64")
& $compiler @userFlags "$src/observer.c" "$src/tests/observer_test.c" "/Fo$out/" "/Fe$out/observer_test.exe" @userLink > "$out/observer-build.log" 2>&1
if ($LASTEXITCODE -ne 0) { Get-Content "$out/observer-build.log" -Tail 25; exit $LASTEXITCODE }
& "$out/observer_test.exe" > "$out/observer-test.log" 2>&1
if ($LASTEXITCODE -ne 0) { Get-Content "$out/observer-test.log" -Tail 25; exit $LASTEXITCODE }
Get-Content "$out/observer-test.log"
& $compiler @userFlags /DSAYALL_TV_OFFLINE_TEST "$src/driver.c" "$src/observer.c" "$src/tests/driver_test.c" "/Fo$out/" "/Fe$out/driver_test.exe" @userLink > "$out/harness-build.log" 2>&1
if ($LASTEXITCODE -ne 0) { Get-Content "$out/harness-build.log" -Tail 25; exit $LASTEXITCODE }
& "$out/driver_test.exe" > "$out/harness-test.log" 2>&1
if ($LASTEXITCODE -ne 0) { Get-Content "$out/harness-test.log" -Tail 25; exit $LASTEXITCODE }
Get-Content "$out/harness-test.log"
if ($TestsOnly) { exit 0 }
if (!(Test-Path "$wdk/Include/10.0.26100.0/km/ntddk.h")) { throw 'Existing WDK NuGet 10.0.26100.6584 is required; no download is performed.' }
$kernelFlags = @('/nologo', '/c', '/W4', '/WX', '/TC', '/kernel', '/GS', '/guard:cf', '/D_AMD64_', '/DAMD64', '/D_WIN64', '/D_WIN32_WINNT=0x0A00', '/DNTDDI_VERSION=0x0A000006', '/DKMDF_VERSION_MAJOR=1', '/DKMDF_VERSION_MINOR=15', "/I$out", "/I$($vc.FullName)/include", "/I$wdk/Include/10.0.26100.0/km", "/I$wdk/Include/wdf/kmdf/1.15", "/I$sdk/Include/10.0.26100.0/shared", "/I$sdk/Include/10.0.26100.0/ucrt")
& $compiler @kernelFlags "$src/driver.c" "$src/observer.c" "$src/trace.c" "/Fo$out/" > "$out/kernel-build.log" 2>&1
if ($LASTEXITCODE -ne 0) { Get-Content "$out/kernel-build.log" -Tail 35; exit $LASTEXITCODE }
& $linker /nologo /driver /subsystem:native,10.00 /entry:FxDriverEntry /nodefaultlib /machine:x64 /dynamicbase /nxcompat /guard:cf /integritycheck "/out:$out/SayAllTvKeyboard.sys" "$out/driver.obj" "$out/observer.obj" "$out/trace.obj" "/LIBPATH:$wdk/Lib/10.0.26100.0/km/x64" "/LIBPATH:$wdk/Lib/wdf/kmdf/x64/1.15" ntoskrnl.lib hal.lib wmilib.lib BufferOverflowFastFailK.lib WdfLdr.lib WdfDriverEntry.lib > "$out/kernel-link.log" 2>&1
if ($LASTEXITCODE -ne 0) { Get-Content "$out/kernel-link.log" -Tail 25; exit $LASTEXITCODE }
[ordered]@{ schema_version = 1; source_fingerprint = $fingerprint; driver_sha256 = (Get-FileHash -LiteralPath "$out/SayAllTvKeyboard.sys" -Algorithm SHA256).Hash.ToLowerInvariant(); inputs = $sourceEntries } | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath "$out/source-identity.json" -Encoding utf8
Write-Output 'kernel_build=passed signature=missing installation=not_attempted capture=absent hardware=deferred'
