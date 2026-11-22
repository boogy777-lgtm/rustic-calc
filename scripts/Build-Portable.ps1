[CmdletBinding()]
param(
    [string] $ManifestPath,
    [string] $BinaryName = 'calc1',
    [string] $TargetDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
if (-not $ManifestPath) { $ManifestPath = Join-Path $root 'Cargo.toml' }
$cargoArgs = @(
    'rustc', '--release', '--locked', '--target', 'x86_64-pc-windows-msvc',
    '--manifest-path', [IO.Path]::GetFullPath($ManifestPath), '--bin', $BinaryName
)
if ($TargetDirectory) { $cargoArgs += @('--target-dir', [IO.Path]::GetFullPath($TargetDirectory)) }

Push-Location -LiteralPath $root
try {
    # Set the CRT mode on the executable link; developer/test builds and
    # the dependency build cache can still use the regular Cargo settings.
    & cargo @cargoArgs -- -C target-feature=+crt-static
    if ($LASTEXITCODE -ne 0) { throw "Portable build failed: $BinaryName." }
} finally {
    Pop-Location
}
