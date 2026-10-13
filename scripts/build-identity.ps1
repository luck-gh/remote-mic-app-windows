param(
    [string]$RepositoryRoot = (Split-Path -Parent $PSScriptRoot),
    [switch]$WriteManifest,
    [switch]$CargoFormat,
    [switch]$IncludePayloads
)
$ErrorActionPreference = 'Stop'

function Invoke-GitText([string[]]$Arguments) {
    $previousErrorAction = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { $result = @(& git -C $RepositoryRoot @Arguments 2>$null | ForEach-Object { [string]$_ }); $exitCode = $LASTEXITCODE }
    catch { $result = @(); $exitCode = 1 }
    finally { $ErrorActionPreference = $previousErrorAction }
    if ($exitCode -ne 0) { return [pscustomobject]@{ ok = $false; lines = @() } }
    return [pscustomobject]@{ ok = $true; lines = $result }
}

function Get-InputHash([string]$Path) {
    try {
        $algorithm = [System.Security.Cryptography.SHA256]::Create()
        try {
            $stream = [IO.File]::OpenRead($Path)
            try {
                return [BitConverter]::ToString($algorithm.ComputeHash($stream)).Replace('-', '').ToLowerInvariant()
            } finally {
                $stream.Dispose()
            }
        } finally {
            $algorithm.Dispose()
        }
    }
    catch {
        $inputName = Split-Path -Leaf $Path
        throw "Could not hash build input '$inputName' (error=$($_.Exception.GetType().Name), hresult=$($_.Exception.HResult))"
    }
}

$headResult = Invoke-GitText @('rev-parse', 'HEAD')
$head = if ($headResult.ok -and $headResult.lines.Count -eq 1 -and $headResult.lines[0] -match '^[0-9a-f]{40}$') { $headResult.lines[0] } else { $null }
$statusResult = Invoke-GitText @('status', '--porcelain', '--untracked-files=all')
$gitAvailable = $null -ne $head -and $statusResult.ok
$state = if (-not $gitAvailable) { 'unknown' } elseif ($statusResult.lines.Count -eq 0) { 'clean' } else { 'dirty' }

$allowedExtensions = @('.rs', '.toml', '.json', '.js', '.mjs', '.cjs', '.ts', '.tsx', '.vue', '.css', '.html', '.nsh', '.ps1', '.py', '.svg', '.png', '.ico', '.lock', '.yml', '.yaml')
$allowedRoots = @('src', 'src-tauri', 'crates', 'hardware', 'scripts', 'public', 'contracts')
$rootInputs = @('Cargo.toml', 'Cargo.lock', 'package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml', 'index.html', 'vite.config.ts', 'tsconfig.json', 'tsconfig.app.json', 'tsconfig.node.json')
$trackedResult = Invoke-GitText @('ls-files', '-co', '--exclude-standard')
$records = [System.Collections.Generic.List[string]]::new()
if ($trackedResult.ok) {
    foreach ($relative in $trackedResult.lines) {
        $normalized = $relative.Replace('\', '/')
        $rootName = ($normalized -split '/', 2)[0]
        $isAllowed = $rootInputs -contains $normalized -or ($allowedRoots -contains $rootName -and [IO.Path]::GetExtension($normalized).ToLowerInvariant() -in $allowedExtensions)
        if (-not $isAllowed -or $normalized -match '(^|/)(target|node_modules|artifacts|\.git|dist|\.pnpm-store)(/|$)') { continue }
        $fullPath = Join-Path $RepositoryRoot ($normalized.Replace('/', [IO.Path]::DirectorySeparatorChar))
        if (-not (Test-Path -LiteralPath $fullPath -PathType Leaf)) { continue }
        $fileHash = Get-InputHash $fullPath
        $records.Add("source`n$normalized`n$fileHash")
    }
}

# Include the exact staged helper/Gadget payload bytes recorded by this build's existing staging receipt.
$receiptPath = Join-Path $RepositoryRoot 'target/dev/bundle-staging/payloads.json'
if ($IncludePayloads -and -not (Test-Path -LiteralPath $receiptPath -PathType Leaf)) { throw 'Current bundle staging receipt is missing' }
if ($IncludePayloads) {
    $receipt = Get-Content -LiteralPath $receiptPath -Raw | ConvertFrom-Json
    $resources = @($receipt.resources | Sort-Object name)
    if ($resources.Count -eq 0 -or @($resources | Where-Object { $_.name -eq 'sayall-helper.exe' }).Count -ne 1 -or @($resources | Where-Object { $_.name -eq 'frida-gadget.dll' }).Count -ne 1) {
        throw 'Current bundle receipt does not contain the required x64 payload set'
    }
    foreach ($resource in $resources) {
        $payloadPath = Join-Path (Join-Path $RepositoryRoot 'src-tauri') $resource.name
        if (-not (Test-Path -LiteralPath $payloadPath -PathType Leaf)) { throw 'Staged bundle input is missing' }
        $actual = Get-InputHash $payloadPath
        if ($actual -ne $resource.sha256) { throw 'Staged bundle input changed after receipt creation' }
        $records.Add("payload`n$($resource.name)`n$actual")
    }
}

$sha = [System.Security.Cryptography.SHA256]::Create()
try {
    $canonical = [System.Text.Encoding]::UTF8.GetBytes(($records | Sort-Object) -join "`n")
    $fingerprint = [BitConverter]::ToString($sha.ComputeHash($canonical)).Replace('-', '').ToLowerInvariant()
} finally { $sha.Dispose() }

$buildChannel = if (-not [string]::IsNullOrWhiteSpace($env:SAYALL_BUILD_CHANNEL)) { $env:SAYALL_BUILD_CHANNEL }
elseif ([string]::IsNullOrWhiteSpace($env:GITHUB_SHA)) { 'unknown' }
elseif ($env:GITHUB_EVENT_NAME -eq 'pull_request') { 'pr_preview' }
elseif ($env:GITHUB_EVENT_NAME -eq 'push') { 'stable' }
else { 'unknown' }

$identity = [ordered]@{
    sourceCommit = if ($state -eq 'clean' -and ([string]::IsNullOrWhiteSpace($env:GITHUB_SHA) -or $env:GITHUB_SHA -eq $head)) { $head } else { $null }
    sourceBaseCommit = $head
    sourceState = $state
    sourceFingerprint = if ($gitAvailable) { $fingerprint } else { 'unknown' }
    buildChannel = $buildChannel
    payloadInputs = if ($IncludePayloads) { 'included' } else { 'not-bundling' }
}
if ($WriteManifest) {
    $manifestPath = Join-Path $RepositoryRoot 'target/dev/foreground-input/build-identity-current.json'
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $manifestPath) | Out-Null
    $identity | ConvertTo-Json | Set-Content -LiteralPath $manifestPath -Encoding utf8
}
if ($CargoFormat) {
    "source_commit=$($identity.sourceCommit)"
    "source_base_commit=$($identity.sourceBaseCommit)"
    "source_state=$($identity.sourceState)"
    "source_fingerprint=$($identity.sourceFingerprint)"
    "build_channel=$($identity.buildChannel)"
    "payload_inputs=$($identity.payloadInputs)"
} else {
    $identity | ConvertTo-Json -Compress
}
