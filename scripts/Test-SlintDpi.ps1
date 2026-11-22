[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string] $BinaryPath,
    [string] $OutputDirectory = 'artifacts\screenshots'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$BinaryPath = [IO.Path]::GetFullPath($BinaryPath)
$run = Join-Path ([IO.Path]::GetFullPath($OutputDirectory)) ('slint-dpi-' + [guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($run) | Out-Null
$previous = $env:SLINT_SCALE_FACTOR
$logicalSize = $null
$count = 0
try {
    foreach ($dpi in @(96, 72, 120, 144, 168, 192, 240, 288, 384)) {
        $factor = $dpi / 96.0
        $env:SLINT_SCALE_FACTOR = $factor.ToString([Globalization.CultureInfo]::InvariantCulture)
        foreach ($mode in @('standard', 'coordinates')) {
            $file = Join-Path $run "$dpi-$mode.png"
            $p = Start-Process -FilePath $BinaryPath -ArgumentList @(
                '--mode', $mode, '--theme', 'dark', '--lang', 'ru', '--screenshot', ('"' + $file + '"')
            ) -PassThru
            try {
                if (-not $p.WaitForExit(15000)) { $p.Kill(); throw "Slint DPI capture timed out: $dpi/$mode" }
                if (Test-Path -LiteralPath "$file.error") {
                    throw "Slint DPI capture failed: $dpi/$mode. $([IO.File]::ReadAllText("$file.error"))"
                }
                if ($p.ExitCode -ne 0) { throw "Slint DPI capture failed: $dpi/$mode, exit $($p.ExitCode)." }
            } finally {
                $p.Dispose()
            }
            $bitmap = [Drawing.Bitmap]::new($file)
            try {
                if ($null -eq $logicalSize) { $logicalSize = @($bitmap.Width, $bitmap.Height) }
                $width = [Math]::Round($logicalSize[0] * $factor)
                $height = [Math]::Round($logicalSize[1] * $factor)
                if ($bitmap.Width -ne $width -or $bitmap.Height -ne $height) {
                    throw "Slint DPI geometry $dpi/${mode}: expected ${width}x$height, got $($bitmap.Width)x$($bitmap.Height)."
                }
            } finally {
                $bitmap.Dispose()
            }
            $count++
        }
    }
} finally {
    $env:SLINT_SCALE_FACTOR = $previous
}
Write-Host "PASS: $count synthetic Slint captures at nine backend scales. No physical DPI change tested. Artifacts: $run"
