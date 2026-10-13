$ErrorActionPreference = 'Stop'
$identityScript = Join-Path $PSScriptRoot 'build-identity.ps1'
$fixtureRoot = Join-Path (Join-Path (Split-Path -Parent $PSScriptRoot) 'target/dev/foreground-input') ("build-identity-fixture-" + [guid]::NewGuid().ToString('N'))
$fixtureParent = [IO.Path]::GetFullPath((Split-Path -Parent $fixtureRoot)).TrimEnd('\', '/')
$fixtureRoot = [IO.Path]::GetFullPath($fixtureRoot)
if (-not $fixtureRoot.StartsWith($fixtureParent + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase) -or (Test-Path -LiteralPath $fixtureRoot)) {
    throw 'The unique fixture destination is outside its allowed root or already exists'
}
$savedGithubSha = $env:GITHUB_SHA
$savedChannel = $env:SAYALL_BUILD_CHANNEL
$savedEvent = $env:GITHUB_EVENT_NAME

function Get-FixtureIdentity([string]$Root) {
    $output = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $identityScript -RepositoryRoot $Root
    if ($LASTEXITCODE -ne 0 -or @($output).Count -ne 1) { throw 'Identity helper failed on a controlled fixture' }
    return ($output | ConvertFrom-Json)
}

try {
    New-Item -ItemType Directory -Force -Path (Join-Path $fixtureRoot 'src'), (Join-Path $fixtureRoot 'artifacts') | Out-Null
    Set-Content -LiteralPath (Join-Path $fixtureRoot 'src/input.rs') -Value 'one' -NoNewline
    & git -C $fixtureRoot init -q
    & git -C $fixtureRoot -c user.name=Fixture -c user.email=fixture@example.invalid add src/input.rs
    & git -C $fixtureRoot -c user.name=Fixture -c user.email=fixture@example.invalid commit -qm fixture
    $head = (& git -C $fixtureRoot rev-parse HEAD).Trim()

    Remove-Item Env:GITHUB_SHA -ErrorAction SilentlyContinue
    Remove-Item Env:SAYALL_BUILD_CHANNEL -ErrorAction SilentlyContinue
    $clean = Get-FixtureIdentity $fixtureRoot
    if ($clean.sourceState -ne 'clean' -or $clean.sourceCommit -ne $head -or $clean.sourceFingerprint -notmatch '^[0-9a-f]{64}$') { throw 'Clean identity did not report an exact commit and fingerprint' }
    $env:SAYALL_BUILD_CHANNEL = 'local'
    $localBuild = Get-FixtureIdentity $fixtureRoot
    if ($localBuild.buildChannel -ne 'local') { throw 'Local build channel was not preserved' }

    Set-Content -LiteralPath (Join-Path $fixtureRoot 'artifacts/private-evidence.txt') -Value 'one' -NoNewline
    Set-Content -LiteralPath (Join-Path $fixtureRoot 'src/input.rs') -Value 'two' -NoNewline
    $dirty = Get-FixtureIdentity $fixtureRoot
    if ($dirty.sourceState -ne 'dirty' -or $null -ne $dirty.sourceCommit -or $dirty.sourceBaseCommit -ne $head -or $dirty.sourceFingerprint -eq $clean.sourceFingerprint) { throw 'Dirty source input was not distinguished from its baseline' }

    Set-Content -LiteralPath (Join-Path $fixtureRoot 'src/input.rs') -Value 'one' -NoNewline
    Set-Content -LiteralPath (Join-Path $fixtureRoot 'artifacts/private-evidence.txt') -Value 'two' -NoNewline
    $evidenceOnly = Get-FixtureIdentity $fixtureRoot
    if ($evidenceOnly.sourceState -ne 'dirty' -or $evidenceOnly.sourceFingerprint -ne $clean.sourceFingerprint) { throw 'Non-build evidence entered the source fingerprint' }

    $env:GITHUB_SHA = $head
    $ciClean = Get-FixtureIdentity $fixtureRoot
    if ($ciClean.sourceState -ne 'dirty' -or $null -ne $ciClean.sourceCommit -or $ciClean.payloadInputs -ne 'not-bundling') { throw 'CI base SHA overrode dirty working-tree state or incorrectly required bundle inputs' }
    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $identityScript -RepositoryRoot $fixtureRoot -IncludePayloads 2>$null | Out-Null; $missingPayloadExit = $LASTEXITCODE }
    finally { $ErrorActionPreference = $previousErrorAction }
    if ($missingPayloadExit -eq 0) { throw 'Explicit bundle identity accepted a missing staging receipt' }
    $env:GITHUB_SHA = ('0' * 40)
    $ciMismatch = Get-FixtureIdentity $fixtureRoot
    if ($null -ne $ciMismatch.sourceCommit) { throw 'Mismatched CI SHA was reported as the exact commit' }

    New-Item -ItemType Directory -Force -Path (Join-Path $fixtureRoot 'src-tauri'), (Join-Path $fixtureRoot 'target/dev/bundle-staging') | Out-Null
    $payloadPath = Join-Path $fixtureRoot 'src-tauri/sayall-helper.exe'
    Set-Content -LiteralPath $payloadPath -Value 'fixture payload' -NoNewline
    $payloadHash = (Get-FileHash -LiteralPath $payloadPath -Algorithm SHA256).Hash.ToLowerInvariant()
    $gadgetPath = Join-Path $fixtureRoot 'src-tauri/frida-gadget.dll'
    Set-Content -LiteralPath $gadgetPath -Value 'fixture gadget' -NoNewline
    $gadgetHash = (Get-FileHash -LiteralPath $gadgetPath -Algorithm SHA256).Hash.ToLowerInvariant()
    @{ resources = @(@{ name = 'sayall-helper.exe'; sha256 = $payloadHash }, @{ name = 'frida-gadget.dll'; sha256 = $gadgetHash }) } | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $fixtureRoot 'target/dev/bundle-staging/payloads.json') -Encoding utf8
    $env:GITHUB_SHA = $head
    Remove-Item Env:SAYALL_BUILD_CHANNEL -ErrorAction SilentlyContinue
    $withPayload = & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $identityScript -RepositoryRoot $fixtureRoot -IncludePayloads
    $withPayload = $withPayload | ConvertFrom-Json
    if ($withPayload.payloadInputs -ne 'included' -or $withPayload.sourceFingerprint -eq $clean.sourceFingerprint) { throw 'Current staged payload bytes were omitted from the bundle identity' }
    Set-Content -LiteralPath $payloadPath -Value 'changed after receipt' -NoNewline
    $mismatchExit = 0
    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & powershell.exe -NoProfile -ExecutionPolicy Bypass -File $identityScript -RepositoryRoot $fixtureRoot -IncludePayloads 2>$null | Out-Null; $mismatchExit = $LASTEXITCODE }
    finally { $ErrorActionPreference = $previousErrorAction }
    if ($mismatchExit -eq 0) { throw 'Changed staged payload was accepted after receipt creation' }

    $unknown = Get-FixtureIdentity $env:WINDIR
    if ($unknown.sourceState -ne 'unknown' -or $null -ne $unknown.sourceCommit -or $unknown.sourceFingerprint -ne 'unknown') { throw 'No-Git input was not reported as unknown' }
    if (($clean | ConvertTo-Json -Compress) -match [regex]::Escape($fixtureRoot)) { throw 'Identity output leaked a fixture path' }
    Write-Output 'Build identity fixture checks passed: clean, dirty, evidence exclusion, CI base, mismatch, and no-Git unknown.'
} finally {
    $env:GITHUB_SHA = $savedGithubSha
    $env:SAYALL_BUILD_CHANNEL = $savedChannel
    $env:GITHUB_EVENT_NAME = $savedEvent
    if (Test-Path -LiteralPath $fixtureRoot) {
        $resolvedFixture = [IO.Path]::GetFullPath($fixtureRoot).TrimEnd('\', '/')
        if ($resolvedFixture -ne $fixtureRoot -or -not $resolvedFixture.StartsWith($fixtureParent + [IO.Path]::DirectorySeparatorChar, [StringComparison]::OrdinalIgnoreCase)) {
            throw 'Refusing to recycle a fixture outside the unique task directory'
        }
        $fixtureItem = Get-Item -LiteralPath $fixtureRoot -Force
        if (-not $fixtureItem.PSIsContainer -or ($fixtureItem.Attributes -band [IO.FileAttributes]::ReparsePoint)) {
            throw 'Refusing to recycle a non-directory or reparse-point fixture'
        }
        $unexpectedReparsePoint = @(Get-ChildItem -LiteralPath $fixtureRoot -Force -Recurse | Where-Object { $_.Attributes -band [IO.FileAttributes]::ReparsePoint })
        if ($unexpectedReparsePoint.Count -ne 0) { throw 'Refusing to recycle a fixture containing reparse points' }

        . (Join-Path (Split-Path -Parent $PSScriptRoot) 'src-tauri/windows/recycle-retired-files.ps1') -FunctionsOnly
        Initialize-SayAllRecycleApi
        # File() succeeds only after IFileOperation reports recycle semantics through
        # PostDeleteItem and the original path disappears; it has no permanent-delete fallback.
        [SayAllRecycle]::File($fixtureRoot)
        if (Test-Path -LiteralPath $fixtureRoot) { throw 'Fixture remained after the Recycle Bin operation' }
    }
}
