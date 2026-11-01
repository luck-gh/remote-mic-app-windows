# 版本唯一来源仍为 tauri.conf.json; 仅适配公开数字与内部 SemVer 的表示.
function ConvertTo-PublicAppVersion([string]$Version) {
    if ($Version -match '^(\d+\.\d+\.\d+)\+([1-9]\d*)$') {
        if ([uint64]$Matches[2] -gt 65535) { throw 'Personal revision exceeds Windows version range' }
        return $Matches[1] + '.' + $Matches[2]
    }
    if ($Version.Contains('+')) { throw 'Personal revision must be one positive numeric build identifier' }
    return $Version
}

function ConvertTo-ComparableAppVersion([string]$Version) {
    $public=ConvertTo-PublicAppVersion $Version
    if($public -match '^\d+\.\d+\.\d+$'){$public += '.0'}
    return [Version]$public
}
