[CmdletBinding()]
param([string] $OutputDirectory, [string] $CompilerPath)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $OutputDirectory) { $OutputDirectory = Join-Path $root 'dist' }
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
if ($env:RUSTFLAGS -match 'target-cpu\s*=\s*native') { throw 'Release cannot use target-cpu=native.' }
$manifest = [IO.File]::ReadAllText((Join-Path $root 'Cargo.toml'))
$version = [regex]::Match($manifest, '(?m)^version\s*=\s*"([0-9]+\.[0-9]+\.[0-9]+)"').Groups[1].Value
if (-not $version) { throw 'Cannot read numeric package version.' }
if (-not $CompilerPath) {
    $candidate = Get-Command ISCC.exe -ErrorAction SilentlyContinue
    if ($null -ne $candidate) { $CompilerPath = $candidate.Source }
    else { $CompilerPath = Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe' }
}
if (-not (Test-Path -LiteralPath $CompilerPath)) { throw 'Install Inno Setup 6 (ISCC.exe), or pass -CompilerPath.' }
$target = Join-Path $root 'target'
if ($env:CARGO_TARGET_DIR) { $target = [IO.Path]::GetFullPath($env:CARGO_TARGET_DIR) }
$binary = Join-Path $target 'x86_64-pc-windows-msvc\release\calc1.exe'
if (-not (Test-Path -LiteralPath $binary)) { throw 'Build with cargo build --release --locked first.' }
$product = (Get-Item -LiteralPath $binary).VersionInfo
if ($product.ProductName -ne 'Rustic Calc' -or $product.ProductVersion -ne $version) { throw 'PE version metadata does not match Cargo/version branding.' }
[IO.Directory]::CreateDirectory($OutputDirectory) | Out-Null
$portable = Join-Path $OutputDirectory 'RusticCalc.exe'
Copy-Item -LiteralPath $binary -Destination $portable -Force
& $CompilerPath ('/DAppVersion=' + $version) ('/DAppSourceExe=' + $portable) ('/DAppOutputDir=' + $OutputDirectory) (Join-Path $root 'packaging\RusticCalc.iss')
if ($LASTEXITCODE -ne 0) { throw 'Inno Setup compilation failed.' }
$lines = foreach ($name in @('RusticCalc.exe', 'RusticCalc-Setup.exe')) {
    $hash = (Get-FileHash -LiteralPath (Join-Path $OutputDirectory $name) -Algorithm SHA256).Hash.ToLowerInvariant()
    $hash + '  ' + $name
}
[IO.File]::WriteAllLines((Join-Path $OutputDirectory 'SHA256SUMS.txt'), $lines, (New-Object Text.UTF8Encoding($false)))
$notes = "Rustic Calc $version for Windows 10/11 x64.`n`nPortable: RusticCalc.exe. Installer: RusticCalc-Setup.exe (per-user, no administrator required).`n`nNew in this release: WGS84 -> MSK (Ctrl+G) with 262 community PROJ templates covering 79 regions, editable parameters, custom definitions, strict offline conversion and selectable X/Y results. Coordinate drafts last only for the current session. The bundled catalogue is Apache-2.0 and is not certified survey data; verify official parameters and independent control points for cadastral or legal work.`n`nThe application is MIT licensed. Binaries are currently unsigned; verify SHA256SUMS.txt and the download origin.`n`nLinux/macOS and native x86/ARM64 builds are not supported yet; PRs welcome.`n"
[IO.File]::WriteAllText((Join-Path $OutputDirectory 'RELEASE_NOTES.md'), $notes, (New-Object Text.UTF8Encoding($false)))
Write-Host "Release $version prepared: $OutputDirectory"
