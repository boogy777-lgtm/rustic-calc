[CmdletBinding()]
param(
    [string] $OutputDirectory,
    [string] $CompilerPath,
    [ValidateSet('Win32', 'Slint', 'GPUI')]
    [string] $Edition = 'Win32',
    [string] $BinaryPath,
    [switch] $Overwrite
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

function Assert-X64Pe([string] $Path) {
    $stream = [IO.File]::Open($Path, [IO.FileMode]::Open, [IO.FileAccess]::Read, [IO.FileShare]::Read)
    $reader = $null
    try {
        $reader = New-Object IO.BinaryReader($stream)
        if ($stream.Length -lt 64 -or $reader.ReadUInt16() -ne 0x5A4D) {
            throw 'The release input is not a valid PE executable (missing DOS header).'
        }

        $stream.Position = 0x3c
        $peOffset = [long] $reader.ReadInt32()
        if ($peOffset -lt 0 -or ($peOffset + 26) -gt $stream.Length) {
            throw 'The release input has an invalid PE header offset.'
        }

        $stream.Position = $peOffset
        if ($reader.ReadUInt32() -ne 0x00004550) {
            throw 'The release input is not a valid PE executable (missing PE signature).'
        }
        $machine = $reader.ReadUInt16()
        $stream.Position = $peOffset + 22
        $characteristics = $reader.ReadUInt16()
        $stream.Position = $peOffset + 24
        $optionalHeaderMagic = $reader.ReadUInt16()
        if ($machine -ne 0x8664 -or $optionalHeaderMagic -ne 0x020b) {
            throw ('Release packaging requires an x64 PE32+ executable; found machine 0x{0:X4}, optional-header 0x{1:X4}.' -f $machine, $optionalHeaderMagic)
        }
        if (($characteristics -band 0x0002) -eq 0 -or ($characteristics -band 0x2000) -ne 0) {
            throw 'The release input must be an executable PE image, not a DLL.'
        }
    } finally {
        if ($null -ne $reader) { $reader.Dispose() }
        else { $stream.Dispose() }
    }
}

$root = Split-Path -Parent $PSScriptRoot
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $root 'dist' }
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)

if ($env:RUSTFLAGS -match 'target-cpu\s*=\s*native') {
    throw 'Release cannot use target-cpu=native.'
}

$manifest = [IO.File]::ReadAllText((Join-Path $root 'Cargo.toml'))
$version = [regex]::Match($manifest, '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
if (-not $version) { throw 'Cannot read numeric package version.' }

if (-not $BinaryPath) {
    $target = Join-Path $root 'target'
    if ($env:CARGO_TARGET_DIR) { $target = [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR) }
    $BinaryPath = Join-Path $target 'x86_64-pc-windows-msvc\release\calc1.exe'
}
$BinaryPath = [IO.Path]::GetFullPath($BinaryPath)
if (-not (Test-Path -LiteralPath $BinaryPath -PathType Leaf)) {
    throw "Release input executable not found: $BinaryPath. Build the selected edition before packaging."
}
Assert-X64Pe $BinaryPath
& python (Join-Path $root 'tools\check_windows_imports.py') $BinaryPath
if ($LASTEXITCODE -ne 0) { throw 'Release input failed the portable Windows import check.' }

$expectedDescription = switch ($Edition) {
    'Win32' { 'Rustic Calc' }
    'Slint' { 'Rustic Calc (Slint)' }
    'GPUI' { 'Rustic Calc (GPUI)' }
}
$product = (Get-Item -LiteralPath $BinaryPath).VersionInfo
if ($product.ProductName -ne 'Rustic Calc' -or
    $product.FileDescription -ne $expectedDescription -or
    $product.ProductVersion -ne $version) {
    throw ("PE metadata mismatch for edition {0}. Expected ProductName='Rustic Calc', FileDescription='{1}', ProductVersion='{2}'; found ProductName='{3}', FileDescription='{4}', ProductVersion='{5}'." -f
        $Edition, $expectedDescription, $version, $product.ProductName, $product.FileDescription, $product.ProductVersion)
}

if (-not $CompilerPath) {
    $candidate = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($null -ne $candidate) { $CompilerPath = $candidate.Source }
    else { $CompilerPath = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe' }
}
if (-not (Test-Path -LiteralPath $CompilerPath -PathType Leaf)) {
    throw 'Install Inno Setup 6 (ISCC.exe), or pass -CompilerPath.'
}
$CompilerPath = [IO.Path]::GetFullPath($CompilerPath)

$editionFiles = @{
    Win32 = @{
        Portable = 'RusticCalc.exe'
        Installer = 'RusticCalc-Setup.exe'
        Checksums = 'SHA256SUMS.txt'
        Notes = 'RELEASE_NOTES.md'
        Notices = 'THIRD-PARTY-NOTICES-Win32.txt'
    }
    Slint = @{
        Portable = 'RusticCalc-Slint.exe'
        Installer = 'RusticCalc-Slint-Setup.exe'
        Checksums = 'SHA256SUMS-Slint.txt'
        Notes = 'RELEASE_NOTES-Slint.md'
        Notices = 'THIRD-PARTY-NOTICES-Slint.txt'
    }
    GPUI = @{
        Portable = 'RusticCalc-GPUI.exe'
        Installer = 'RusticCalc-GPUI-Setup.exe'
        Checksums = 'SHA256SUMS-GPUI.txt'
        Notes = 'RELEASE_NOTES-GPUI.md'
        Notices = 'THIRD-PARTY-NOTICES-GPUI.txt'
    }
}
$files = $editionFiles[$Edition]
$artifactNames = @($files.Portable, $files.Installer, $files.Checksums, $files.Notes, $files.Notices)

# Never replace a previously built executable by default, including the legacy
# Win32 filenames. An intentional replacement requires the explicit flag.
if (-not $Overwrite) {
    $collisions = @($artifactNames | Where-Object {
        Test-Path -LiteralPath (Join-Path $OutputDirectory $_)
    })
    if ($collisions.Count -gt 0) {
        throw ("Refusing to overwrite existing {0} artifacts: {1}. Pass -Overwrite to replace this edition's outputs." -f
            $Edition, ($collisions -join ', '))
    }
}

[IO.Directory]::CreateDirectory($OutputDirectory) | Out-Null
$stageDirectory = Join-Path $OutputDirectory ('.packaging-' + $Edition + '-' + [guid]::NewGuid().ToString('N'))
if (Test-Path -LiteralPath $stageDirectory) { throw 'Temporary packaging directory unexpectedly already exists.' }
[IO.Directory]::CreateDirectory($stageDirectory) | Out-Null

try {
    $stagedPortable = Join-Path $stageDirectory $files.Portable
    Copy-Item -LiteralPath $BinaryPath -Destination $stagedPortable

    $stagedNotices = Join-Path $stageDirectory $files.Notices
    & "$PSScriptRoot\Build-LicenseNotices.ps1" -Edition $Edition -OutputPath $stagedNotices
    if (-not (Test-Path -LiteralPath $stagedNotices -PathType Leaf)) {
        throw "The $Edition third-party notice bundle was not generated."
    }

    & $CompilerPath `
        ('/DAppVersion=' + $version) `
        ('/DAppEdition=' + $Edition) `
        ('/DAppSourceExe="' + $stagedPortable + '"') `
        ('/DAppNoticeBundlePath="' + $stagedNotices + '"') `
        ('/DAppOutputDir="' + $stageDirectory + '"') `
        (Join-Path $root 'packaging\RusticCalc.iss')
    if ($LASTEXITCODE -ne 0) { throw "Inno Setup compilation failed with exit code $LASTEXITCODE." }

    $stagedInstaller = Join-Path $stageDirectory $files.Installer
    if (-not (Test-Path -LiteralPath $stagedInstaller -PathType Leaf)) {
        throw "Inno Setup did not produce the expected $($files.Installer)."
    }

    $checksumLines = foreach ($name in @($files.Portable, $files.Installer, $files.Notices)) {
        $hash = (Get-FileHash -LiteralPath (Join-Path $stageDirectory $name) -Algorithm SHA256).Hash.ToLowerInvariant()
        $hash + '  ' + $name
    }
    [IO.File]::WriteAllLines(
        (Join-Path $stageDirectory $files.Checksums),
        $checksumLines,
        (New-Object Text.UTF8Encoding($false))
    )

    if ($Edition -eq 'Win32') {
        $notes = "Rustic Calc $version for Windows 10/11 x64.`n`nPortable: $($files.Portable). Installer: $($files.Installer) (per-user, no administrator required).`n`nThe portable executable uses the static MSVC CRT configuration. See $($files.Notices) for the locked Windows dependency license and notice inventory. The application is MIT licensed; the bundled MSK catalogue retains Apache-2.0.`n`nBinaries are unsigned; verify $($files.Checksums) and the download origin. These notes make no claim that any test matrix has passed.`n"
    } else {
        $graphics = if ($Edition -eq 'Slint') { 'Slint/FemtoVG requires a compatible OpenGL renderer. Slint attribution is available in About.' } else { 'GPUI requires a compatible Direct3D 11 device and driver; CLI capture also requires a working Windows compositor.' }
        $notes = "Rustic Calc ($Edition) $version for Windows 10/11 x64.`n`nPortable: $($files.Portable). Installer: $($files.Installer) (per-user, no administrator required).`n`n$graphics`n`nSee $($files.Notices) for the locked Windows dependency license and notice inventory. The application is MIT licensed; the bundled MSK catalogue retains Apache-2.0.`n`nBinaries are unsigned; verify $($files.Checksums) and the download origin. These notes make no claim that any test matrix has passed.`n"
    }
    [IO.File]::WriteAllText(
        (Join-Path $stageDirectory $files.Notes),
        $notes,
        (New-Object Text.UTF8Encoding($false))
    )

    foreach ($name in $artifactNames) {
        $stagedPath = Join-Path $stageDirectory $name
        $destinationPath = Join-Path $OutputDirectory $name
        if ($Overwrite) {
            Move-Item -LiteralPath $stagedPath -Destination $destinationPath -Force
        } else {
            # File.Move without overwrite also protects against a destination
            # appearing after the collision preflight.
            [IO.File]::Move($stagedPath, $destinationPath)
        }
    }
} finally {
    Remove-Item -LiteralPath $stageDirectory -Recurse -Force -ErrorAction SilentlyContinue
}

Write-Host "Release $version ($Edition) prepared: $OutputDirectory"
Write-Host "Standalone notice asset: $($files.Notices)"
