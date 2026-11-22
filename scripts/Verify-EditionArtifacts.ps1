[CmdletBinding()]
param([Parameter(Mandatory = $true)][string] $Directory)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$Directory = [IO.Path]::GetFullPath($Directory)
$editions = @(
    @{ Base = 'RusticCalc'; Manifest = 'SHA256SUMS.txt'; Notice = 'THIRD-PARTY-NOTICES-Win32.txt' },
    @{ Base = 'RusticCalc-Slint'; Manifest = 'SHA256SUMS-Slint.txt'; Notice = 'THIRD-PARTY-NOTICES-Slint.txt' },
    @{ Base = 'RusticCalc-GPUI'; Manifest = 'SHA256SUMS-GPUI.txt'; Notice = 'THIRD-PARTY-NOTICES-GPUI.txt' }
)

$lines = foreach ($edition in $editions) {
    $manifest = Join-Path $Directory $edition.Manifest
    $files = @("$($edition.Base).exe", "$($edition.Base)-Setup.exe", $edition.Notice)
    $recorded = @{}
    foreach ($line in [IO.File]::ReadAllLines($manifest)) {
        if ($line -cnotmatch '^([a-f0-9]{64})  ([A-Za-z0-9-]+\.(?:exe|txt))$' -or
            $files -cnotcontains $Matches[2] -or $recorded.ContainsKey($Matches[2])) {
            throw "Invalid or duplicate checksum entry in $manifest."
        }
        $recorded[$Matches[2]] = $Matches[1]
    }
    if ($recorded.Count -ne 3) { throw "Expected two executables and one notice bundle in $manifest." }
    foreach ($file in $files) {
        $path = Join-Path $Directory $file
        $actual = (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant()
        if ($actual -cne $recorded[$file]) { throw "SHA-256 mismatch for $path." }
        "$actual  $file"
    }
}
$combined = Join-Path $Directory 'SHA256SUMS-Editions.txt'
if (Test-Path -LiteralPath $combined) {
    if (([IO.File]::ReadAllLines($combined) -join "`n") -cne ($lines -join "`n")) {
        throw "Existing combined checksum manifest differs: $combined"
    }
} else {
    $stream = [IO.File]::Open($combined, [IO.FileMode]::CreateNew)
    try {
        $content = [Text.Encoding]::UTF8.GetBytes(($lines -join "`n") + "`n")
        $stream.Write($content, 0, $content.Length)
    } finally {
        $stream.Dispose()
    }
}
Write-Host "PASS: all six executables and three notice bundles match. Manifest: $combined"
