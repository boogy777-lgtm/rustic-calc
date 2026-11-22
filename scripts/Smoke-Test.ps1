[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string] $BinaryPath,
    [string] $OutputDirectory = 'artifacts\screenshots',
    [switch] $IncludeSystemTheme
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing
$BinaryPath = [IO.Path]::GetFullPath($BinaryPath)
$OutputRoot = [IO.Path]::GetFullPath($OutputDirectory)
if (-not (Test-Path -LiteralPath $BinaryPath -PathType Leaf)) { throw "Binary not found: $BinaryPath" }
$RunName = 'smoke-' + [DateTime]::UtcNow.ToString('yyyyMMddTHHmmssfffZ') + '-' + [guid]::NewGuid().ToString('N')
$RunDirectory = Join-Path $OutputRoot $RunName
if (Test-Path -LiteralPath $RunDirectory) { throw "Refusing to reuse smoke output directory: $RunDirectory" }
[IO.Directory]::CreateDirectory($RunDirectory) | Out-Null
$modes = @('standard', 'scientific', 'programmer', 'statistics', 'unit', 'date', 'mortgage', 'lease', 'mpg', 'l100', 'coordinates')
$themes = @('white', 'dark')
if ($IncludeSystemTheme) { $themes += 'system' }
$captureCount = 0
foreach ($theme in $themes) {
    foreach ($language in @('ru', 'en')) {
        foreach ($mode in $modes) {
            $suffix = if ($language -eq 'ru') { '' } else { '-' + $language }
            $image = Join-Path $RunDirectory ($theme + '-' + $mode + $suffix + '.png')
            $errorPath = $image + '.error'
            if ((Test-Path -LiteralPath $image) -or (Test-Path -LiteralPath $errorPath)) {
                throw "Refusing to reuse screenshot output: $image"
            }
            $startedUtc = [DateTime]::UtcNow
            $p = Start-Process -FilePath $BinaryPath -ArgumentList @('--mode', $mode, '--theme', $theme, '--lang', $language, '--screenshot', ('"' + $image + '"')) -PassThru
            try {
                if (-not $p.WaitForExit(15000)) {
                    try { $p.Kill() } catch { }
                    throw "Screenshot timed out after 15 seconds: $mode/$theme/$language"
                }
                if (Test-Path -LiteralPath $errorPath) {
                    throw "Screenshot failed: $mode/$theme/$language. $([IO.File]::ReadAllText($errorPath))"
                }
                if ($p.ExitCode -ne 0) { throw "Screenshot process failed with exit code $($p.ExitCode): $mode/$theme/$language" }
            } finally {
                $p.Dispose()
            }
            if (-not (Test-Path -LiteralPath $image -PathType Leaf)) { throw "Screenshot was not created: $mode/$theme/$language" }
            if ((Get-Item -LiteralPath $image).LastWriteTimeUtc -lt $startedUtc.AddSeconds(-2)) { throw "Screenshot is stale: $image" }
            $bytes = [IO.File]::ReadAllBytes($image)
            if ($bytes.Length -lt 100 -or [BitConverter]::ToString($bytes[0..7]) -ne '89-50-4E-47-0D-0A-1A-0A') { throw "Invalid PNG: $image" }
            if ([Text.Encoding]::ASCII.GetString($bytes, 12, 4) -ne 'IHDR') { throw "PNG has no IHDR header: $image" }
            foreach ($offset in @(16, 20)) {
                if (0 -eq ($bytes[$offset] -bor $bytes[($offset + 1)] -bor $bytes[($offset + 2)] -bor $bytes[($offset + 3)])) {
                    throw "PNG has invalid dimensions: $image"
                }
            }
            # A desktop compositor can return a valid but blank PNG. Sample
            # within the content so title bars and window borders do not count.
            $bitmap = [Drawing.Bitmap]::new($image)
            try {
                $colors = [Collections.Generic.HashSet[int]]::new()
                for ($row = 1; $row -le 15; $row++) {
                    for ($column = 1; $column -le 15; $column++) {
                        $x = [int][Math]::Floor($column * ($bitmap.Width - 1) / 16)
                        $y = [int][Math]::Floor($row * ($bitmap.Height - 1) / 16)
                        [void]$colors.Add($bitmap.GetPixel($x, $y).ToArgb())
                    }
                }
                if ($colors.Count -lt 3) { throw "Screenshot is nearly uniform: $image" }
            } finally {
                $bitmap.Dispose()
            }
            $captureCount++
        }
    }
}
Write-Host "PASS: $captureCount screenshots (eleven modes, $($themes -join '/'), RU/EN). Artifacts: $RunDirectory"
