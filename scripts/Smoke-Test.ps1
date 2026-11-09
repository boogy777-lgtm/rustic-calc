[CmdletBinding()]
param([Parameter(Mandatory = $true)][string] $BinaryPath, [string] $OutputDirectory = 'artifacts\screenshots')
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
$BinaryPath = [IO.Path]::GetFullPath($BinaryPath)
$OutputDirectory = [IO.Path]::GetFullPath($OutputDirectory)
[IO.Directory]::CreateDirectory($OutputDirectory) | Out-Null
foreach ($theme in @('white', 'dark')) {
    foreach ($mode in @('standard', 'scientific', 'programmer', 'statistics', 'unit', 'date', 'mortgage', 'lease', 'mpg', 'l100', 'coordinates')) {
      $languages = if ($mode -eq 'coordinates') { @('ru', 'en') } else { @('ru') }
      foreach ($language in $languages) {
        $suffix = if ($language -eq 'ru') { '' } else { '-' + $language }
        $image = Join-Path $OutputDirectory ($theme + '-' + $mode + $suffix + '.png')
        $p = Start-Process -FilePath $BinaryPath -ArgumentList @('--mode', $mode, '--theme', $theme, '--lang', $language, '--screenshot', ('"' + $image + '"')) -PassThru
        if (-not $p.WaitForExit(15000)) { $p.Kill(); throw "Screenshot timed out: $mode/$theme" }
        if ($p.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $image) -or (Test-Path -LiteralPath ($image + '.error'))) { throw "Screenshot failed: $mode/$theme" }
        $bytes = [IO.File]::ReadAllBytes($image)
        if ($bytes.Length -lt 100 -or [BitConverter]::ToString($bytes[0..7]) -ne '89-50-4E-47-0D-0A-1A-0A') { throw "Invalid PNG: $image" }
      }
    }
}
Write-Host 'PASS: all eleven modes rendered in White and Dark; coordinates in RU and EN.'
