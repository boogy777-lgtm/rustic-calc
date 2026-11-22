[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string] $OutputDirectory,
    [string] $CompilerPath,
    [string] $TargetDirectory
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$root = Split-Path -Parent $PSScriptRoot
$target = 'x86_64-pc-windows-msvc'
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
if ($CompilerPath) { $CompilerPath = [IO.Path]::GetFullPath($CompilerPath) }
if (-not $TargetDirectory) { $TargetDirectory = Join-Path $root 'target\editions' }
$TargetDirectory = [IO.Path]::GetFullPath($TargetDirectory)
if (Test-Path -LiteralPath $OutputDirectory) {
    throw "Choose a new output directory; refusing to replace $OutputDirectory."
}

$editions = @(
    @{ Name = 'Win32'; Manifest = 'Cargo.toml'; Bin = 'calc1' },
    @{ Name = 'Slint'; Manifest = 'editions\slint\Cargo.toml'; Bin = 'calc1' },
    @{ Name = 'GPUI'; Manifest = 'editions\gpui\Cargo.toml'; Bin = 'RusticCalc-GPUI' }
)

$stage = $OutputDirectory + '.staging-' + [guid]::NewGuid().ToString('N')
[IO.Directory]::CreateDirectory($stage) | Out-Null
Push-Location -LiteralPath $root
try {
    & python tools/check_project.py
    if ($LASTEXITCODE -ne 0) { throw 'Project contracts failed.' }
    foreach ($edition in $editions) {
        $manifest = Join-Path $root $edition.Manifest
        $editionTarget = Join-Path $TargetDirectory $edition.Name
        & cargo fmt --all --check --manifest-path $manifest
        if ($LASTEXITCODE -ne 0) { throw "$($edition.Name) formatting failed." }
        & cargo fetch --locked --manifest-path $manifest --target $target
        if ($LASTEXITCODE -ne 0) { throw "$($edition.Name) dependency fetch failed." }
        & cargo test --release --locked --manifest-path $manifest --target $target --target-dir $editionTarget
        if ($LASTEXITCODE -ne 0) { throw "$($edition.Name) tests failed." }
        & "$PSScriptRoot\Build-Portable.ps1" -ManifestPath $manifest -BinaryName $edition.Bin -TargetDirectory $editionTarget
        $exe = Join-Path $editionTarget "$target\release\$($edition.Bin).exe"
        & "$PSScriptRoot\Smoke-Test.ps1" -BinaryPath $exe -OutputDirectory (Join-Path $stage "ui\$($edition.Name)") -IncludeSystemTheme
        & "$PSScriptRoot\Build-Release.ps1" -Edition $edition.Name -BinaryPath $exe -CompilerPath $CompilerPath -OutputDirectory $stage
    }

    & "$PSScriptRoot\Verify-EditionArtifacts.ps1" -Directory $stage
    [IO.Directory]::Move($stage, $OutputDirectory)
    Write-Host "Three locally built editions: $OutputDirectory"
} catch {
    Write-Warning "No complete bundle was staged. Available build evidence: $stage"
    throw
} finally {
    Pop-Location
}
