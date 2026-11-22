[CmdletBinding()]
param([switch] $IncludeEditions)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$buildPath = Join-Path $PSScriptRoot 'Build-Release.ps1'
$issPath = Join-Path $root 'packaging\RusticCalc.iss'
$buildText = [IO.File]::ReadAllText($buildPath)
$issText = [IO.File]::ReadAllText($issPath)

foreach ($scriptName in @('Build-Portable.ps1', 'Build-LicenseNotices.ps1', 'License-Notice-Exceptions.psd1', 'Build-Release.ps1', 'Build-Editions.ps1', 'Verify-EditionArtifacts.ps1', 'Test-Installer.ps1', 'Smoke-Test.ps1', 'Test-SlintDpi.ps1')) {
    $tokens = $null
    $parseErrors = $null
    $scriptPath = Join-Path $PSScriptRoot $scriptName
    [System.Management.Automation.Language.Parser]::ParseFile(
        $scriptPath,
        [ref] $tokens,
        [ref] $parseErrors
    ) | Out-Null
    if ($parseErrors.Count -gt 0) {
        throw ("$scriptName has PowerShell syntax errors: " + ($parseErrors -join '; '))
    }
}

$noticeExceptions = Import-PowerShellDataFile -LiteralPath (Join-Path $PSScriptRoot 'License-Notice-Exceptions.psd1')
foreach ($legalText in $noticeExceptions.LegalTexts.Values) {
    $hasPinnedSource = $legalText.SourceUrl -match '^https://raw\.githubusercontent\.com/[^/]+/[^/]+/[0-9a-f]{40}/' -or
        $legalText.SourceUrl -ceq 'https://www.apache.org/licenses/LICENSE-2.0.txt'
    if (-not $hasPinnedSource -or $legalText.Sha256 -cnotmatch '^[0-9a-f]{64}$' -or
        [string]::IsNullOrWhiteSpace($legalText.SourceKind) -or [string]::IsNullOrWhiteSpace($legalText.ProvenanceNote)) {
        throw "License exception is missing source, attribution explanation or a pinned digest: $($legalText.Label)"
    }
}
foreach ($package in $noticeExceptions.Packages.Values) {
    if ($package.Files -contains 'MacApacheText' -and $package.Files -notcontains 'MacAuthorNotice') {
        throw 'The Apache text cannot substitute for the pinned publisher grant and author notice.'
    }
}

if ($buildText -notmatch "ValidateSet\('Win32', 'Slint', 'GPUI'\)") {
    throw 'Release packaging must expose exactly the three supported edition names.'
}
foreach ($required in @(
    'Assert-X64Pe',
    'check_windows_imports.py',
    '0x8664',
    '0x020b',
    '0x2000',
    'Rustic Calc (Slint)',
    'Rustic Calc (GPUI)',
    'RusticCalc-Slint.exe',
    'RusticCalc-GPUI.exe',
    'RusticCalc-Slint-Setup.exe',
    'RusticCalc-GPUI-Setup.exe',
    'SHA256SUMS-Slint.txt',
    'SHA256SUMS-GPUI.txt',
    'Build-LicenseNotices.ps1',
    'THIRD-PARTY-NOTICES-Win32.txt',
    'THIRD-PARTY-NOTICES-Slint.txt',
    'THIRD-PARTY-NOTICES-GPUI.txt',
    '$files.Notices',
    '$stagedNotices',
    'AppNoticeBundlePath=',
    'ProductName -ne ''Rustic Calc''',
    '$product.FileDescription -ne $expectedDescription',
    'if (-not $Overwrite)',
    'if ($Overwrite)',
    'Refusing to overwrite existing',
    'AppEdition=',
    '/DAppSourceExe="',
    '/DAppOutputDir="'
)) {
    if (-not $buildText.Contains($required)) { throw "Build-Release.ps1 is missing contract: $required" }
}

& python (Join-Path $root 'tools\test_windows_imports.py')
if ($LASTEXITCODE -ne 0) { throw 'Portable Windows import tests failed.' }

$appIds = [regex]::Matches(
    $issText,
    '(?mi)^\s*#define EditionAppId "\{\{([0-9A-F-]{36})\}"\s*$'
) | ForEach-Object { $_.Groups[1].Value }
if ($appIds.Count -ne 3 -or @($appIds | Select-Object -Unique).Count -ne 3) {
    throw 'Each edition must have a distinct, stable installer AppId.'
}
foreach ($required in @(
    '#ifndef AppEdition',
    '#if AppEdition == "Win32"',
    '#elif AppEdition == "Slint"',
    '#elif AppEdition == "GPUI"',
    'DefaultDirName={localappdata}\Programs\{#EditionDirectoryName}',
    '#ifndef AppNoticeBundlePath',
    'Source: "{#AppNoticeBundlePath}"; DestDir: "{app}\licenses"; DestName: "THIRD-PARTY-NOTICES.txt"',
    'Rustic Calc Slint',
    'Rustic Calc GPUI',
    'RusticCalc-Slint-Setup',
    'RusticCalc-GPUI-Setup',
    'Name: "{group}\{#EditionAppName}"',
    'Name: "{autodesktop}\{#EditionAppName}"',
    'EditionAppId "{{2C85D442-432B-42CD-987B-1738EAE56F13}'
)) {
    if (-not $issText.Contains($required)) { throw "RusticCalc.iss is missing contract: $required" }
}

$aliasTaskIndex = $issText.IndexOf('Name: "cmdalias"', [StringComparison]::Ordinal)
$aliasGuardIndex = $issText.LastIndexOf('#if AppEdition == "Win32"', $aliasTaskIndex, [StringComparison]::Ordinal)
$aliasGuardEnd = $issText.IndexOf('#endif', $aliasTaskIndex, [StringComparison]::Ordinal)
if ($aliasTaskIndex -lt 0 -or $aliasGuardIndex -lt 0 -or $aliasGuardEnd -lt $aliasTaskIndex) {
    throw 'The optional CMD alias task must exist only in the Win32 installer edition.'
}

$uninstallIndex = $issText.IndexOf('[UninstallRun]', [StringComparison]::Ordinal)
$codeEndIndex = $issText.IndexOf('procedure CurUninstallStepChanged', [StringComparison]::Ordinal)
$aliasCodeGuard = $issText.LastIndexOf('#if AppEdition == "Win32"', $uninstallIndex, [StringComparison]::Ordinal)
$aliasCodeGuardEnd = $issText.IndexOf('#endif', $codeEndIndex, [StringComparison]::Ordinal)
if ($uninstallIndex -lt 0 -or $codeEndIndex -lt $uninstallIndex -or
    $aliasCodeGuard -lt 0 -or $aliasCodeGuardEnd -lt $codeEndIndex) {
    throw 'CMD alias ownership and uninstall hooks must be compiled only for Win32.'
}

$noticeScript = Join-Path $PSScriptRoot 'Build-LicenseNotices.ps1'
$noticeTestDir = Join-Path ([IO.Path]::GetTempPath()) ('rustic-calc-notices-' + [guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($noticeTestDir) | Out-Null
$noticeEditions = if ($IncludeEditions) { @('Win32', 'Slint', 'GPUI') } else { @('Win32') }
try {
    foreach ($editionName in $noticeEditions) {
        $first = Join-Path $noticeTestDir "$editionName-first.txt"
        $second = Join-Path $noticeTestDir "$editionName-second.txt"
        & $noticeScript -Edition $editionName -OutputPath $first
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $first -PathType Leaf)) {
            throw "$editionName notice generation failed."
        }
        & $noticeScript -Edition $editionName -OutputPath $second
        if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $second -PathType Leaf)) {
            throw "$editionName repeat notice generation failed."
        }
        if ((Get-FileHash -LiteralPath $first -Algorithm SHA256).Hash -cne
            (Get-FileHash -LiteralPath $second -Algorithm SHA256).Hash) {
            throw "$editionName notice output is not deterministic."
        }
        $noticeText = [IO.File]::ReadAllText($first)
        if (-not $noticeText.StartsWith("Rustic Calc - Third-Party License and Notice Bundle`nEdition: $editionName`n")) {
            throw "$editionName notice output is missing its edition header."
        }
        # Upstream license appendices can contain sample placeholders. Test
        # the provenance records above, not text copied verbatim from dependencies.
        foreach ($requiredNoticeContract in @(
            'cargo tree --offline --locked --target x86_64-pc-windows-msvc --edges normal --prefix none',
            'cargo tree --offline --locked --target x86_64-pc-windows-msvc --edges normal,build --prefix none',
            'exact name, version, and repository',
            'LF-normalized UTF-8 SHA-256: '
        )) {
            if (-not $noticeText.Contains($requiredNoticeContract)) {
                throw "$editionName notice output is missing audit contract: $requiredNoticeContract"
            }
        }
        if ($editionName -eq 'Slint') {
            foreach ($generatedInput in @('gl_generator 0.14.0', 'khronos_api 3.1.0')) {
                if ($noticeText -notmatch "(?m)^Package: $([regex]::Escape($generatedInput))`r?`nGraph: build-only/codegen dependency closure$") {
                    throw "Slint notice output is missing generated-code input from the build closure: $generatedInput"
                }
            }
            if ($noticeText -match '(?m)^Package: (skia-bindings|skia-safe|spirv) ') {
                throw 'Slint notice output includes inactive Skia/SPIR-V packages outside its resolved target graph.'
            }
        }
    }
} finally {
    Remove-Item -LiteralPath $noticeTestDir -Recurse -Force
}

$hashTestDir = Join-Path ([IO.Path]::GetTempPath()) ('rustic-calc-hashes-' + [guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($hashTestDir) | Out-Null
try {
    $editionAssets = @(
        @{ Suffix = ''; Notice = 'THIRD-PARTY-NOTICES-Win32.txt' },
        @{ Suffix = '-Slint'; Notice = 'THIRD-PARTY-NOTICES-Slint.txt' },
        @{ Suffix = '-GPUI'; Notice = 'THIRD-PARTY-NOTICES-GPUI.txt' }
    )
    $expectedLines = foreach ($editionAsset in $editionAssets) {
        $suffix = $editionAsset.Suffix
        $names = @("RusticCalc$suffix.exe", "RusticCalc$suffix-Setup.exe", $editionAsset.Notice)
        $entries = foreach ($name in $names) {
            $path = Join-Path $hashTestDir $name
            [IO.File]::WriteAllText($path, "fixture: $name")
            (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() + "  $name"
        }
        [IO.File]::WriteAllLines((Join-Path $hashTestDir "SHA256SUMS$suffix.txt"), $entries)
        $entries
    }
    $verify = Join-Path $PSScriptRoot 'Verify-EditionArtifacts.ps1'
    & $verify -Directory $hashTestDir
    $combined = Join-Path $hashTestDir 'SHA256SUMS-Editions.txt'
    if (([IO.File]::ReadAllLines($combined) -join "`n") -cne ($expectedLines -join "`n")) {
        throw 'Combined manifest does not contain the expected six executables and three notice bundles.'
    }
    & $verify -Directory $hashTestDir
    [IO.File]::AppendAllText((Join-Path $hashTestDir 'RusticCalc-GPUI-Setup.exe'), 'tampered')
    try {
        & $verify -Directory $hashTestDir
        throw 'Checksum verifier accepted a modified EXE.'
    } catch {
        if ($_.Exception.Message -notlike 'SHA-256 mismatch for *') { throw }
    }
    if (([IO.File]::ReadAllLines($combined) -join "`n") -cne ($expectedLines -join "`n")) {
        throw 'A failed verification modified the combined manifest.'
    }
} finally {
    Remove-Item -LiteralPath $hashTestDir -Recurse -Force
}
Write-Host 'PASS: packaging contracts, deterministic offline notice bundles, combined manifest, and rejection of changed artifacts.'
