[CmdletBinding()]
param([string] $InstallerPath = 'dist\RusticCalc-Setup.exe', [string] $PortablePath = 'dist\RusticCalc.exe')
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'Run full installer smoke tests only on a disposable GitHub Actions Windows runner.' }
function Profile-Snapshot {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Microsoft\Command Processor')
    try {
        if ($null -eq $key -or @($key.GetValueNames()) -notcontains 'AutoRun') { return '<absent>' }
        return $key.GetValueKind('AutoRun').ToString() + ':' + [string]$key.GetValue('AutoRun', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    } finally { if ($null -ne $key) { $key.Dispose() } }
}
function Wait-Child($process, [string] $label) {
    if (-not $process.WaitForExit(120000)) { $process.Kill(); throw "$label timed out." }
    if ($process.ExitCode -ne 0) { throw "$label failed with exit code $($process.ExitCode)." }
}
$before = Profile-Snapshot
$dir = Join-Path $env:RUNNER_TEMP ('RusticCalc-install-' + [guid]::NewGuid().ToString('N'))
$log = Join-Path $env:RUNNER_TEMP 'rustic-installer.log'
$setup = Start-Process -FilePath ([IO.Path]::GetFullPath($InstallerPath)) -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/NOICONS', '/TASKS=', ('/DIR="' + $dir + '"'), ('/LOG="' + $log + '"')) -PassThru
Wait-Child $setup 'Install'
$installed = Join-Path $dir 'RusticCalc.exe'
if (-not (Test-Path -LiteralPath $installed)) { throw 'Installed executable is missing.' }
if ((Get-FileHash -LiteralPath $installed).Hash -ne (Get-FileHash -LiteralPath $PortablePath).Hash) { throw 'Installer payload differs from the portable executable.' }
if ((Profile-Snapshot) -cne $before) { throw 'Unchecked CMD task changed AutoRun.' }
$uninstaller = Join-Path $dir 'unins000.exe'
$uninstall = Start-Process -FilePath $uninstaller -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART') -PassThru
Wait-Child $uninstall 'Uninstall'
if (Test-Path -LiteralPath $installed) { throw 'Uninstall left the application executable.' }
if ((Profile-Snapshot) -cne $before) { throw 'Uninstall changed an unowned CMD profile.' }
Write-Host 'PASS: per-user installer/uninstaller, payload equality, and no unsolicited CMD integration.'
