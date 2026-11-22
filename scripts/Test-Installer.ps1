[CmdletBinding()]
param(
    [string] $InstallerPath = 'dist\RusticCalc-Setup.exe',
    [string] $PortablePath = 'dist\RusticCalc.exe',
    [string] $EditionsDirectory,
    [switch] $WindowsSandbox,
    [switch] $VerifyLaunch
)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if ($WindowsSandbox) {
    $account = [Security.Principal.WindowsIdentity]::GetCurrent().Name
    if ($account -ne "$env:COMPUTERNAME\WDAGUtilityAccount") {
        throw 'The -WindowsSandbox option must run inside Windows Sandbox under its disposable account.'
    }
    $testBase = $env:TEMP
} elseif ($env:GITHUB_ACTIONS -eq 'true' -and $env:RUNNER_ENVIRONMENT -eq 'github-hosted') {
    $testBase = $env:RUNNER_TEMP
} else {
    throw 'Run full installer smoke tests only in Windows Sandbox or on a GitHub-hosted Windows runner.'
}
function Profile-Snapshot {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey('Software\Microsoft\Command Processor')
    try {
        if ($null -eq $key -or @($key.GetValueNames()) -notcontains 'AutoRun') { return '<absent>' }
        return $key.GetValueKind('AutoRun').ToString() + ':' + [string]$key.GetValue('AutoRun', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    } finally { if ($null -ne $key) { $key.Dispose() } }
}
function Wait-Child($process, [string] $label, [int] $TimeoutMs = 120000) {
    try {
        if (-not $process.WaitForExit($TimeoutMs)) { $process.Kill(); throw "$label timed out." }
        if ($process.ExitCode -ne 0) { throw "$label failed with exit code $($process.ExitCode)." }
    } finally {
        $process.Dispose()
    }
}
$before = Profile-Snapshot
$testRoot = Join-Path $testBase ('RusticCalc-install-' + [guid]::NewGuid().ToString('N'))
[IO.Directory]::CreateDirectory($testRoot) | Out-Null
$editions = if ($EditionsDirectory) {
    @(
        @{ Name = 'Win32'; Base = 'RusticCalc'; Id = '{2C85D442-432B-42CD-987B-1738EAE56F13}' },
        @{ Name = 'Slint'; Base = 'RusticCalc-Slint'; Id = '{A3CD8BF1-B96F-4FDB-9C1B-EFC1D55D8260}' },
        @{ Name = 'GPUI'; Base = 'RusticCalc-GPUI'; Id = '{8C701C5E-58BC-4E19-9A16-B8A04FF4B610}' }
    ) | ForEach-Object {
        $_.Installer = Join-Path $EditionsDirectory "$($_.Base)-Setup.exe"
        $_.Portable = Join-Path $EditionsDirectory "$($_.Base).exe"
        $_.Notice = Join-Path $EditionsDirectory "THIRD-PARTY-NOTICES-$($_.Name).txt"
        $_
    }
} else {
    @{
        Name = 'Win32'; Installer = $InstallerPath; Portable = $PortablePath
        Id = '{2C85D442-432B-42CD-987B-1738EAE56F13}'
        Notice = Join-Path (Split-Path -Parent ([IO.Path]::GetFullPath($PortablePath))) 'THIRD-PARTY-NOTICES-Win32.txt'
    }
}
$active = [Collections.Generic.List[object]]::new()

function Assert-Payload($edition) {
    if (-not (Test-Path -LiteralPath $edition.Installed -PathType Leaf)) {
        throw "$($edition.Name) installed executable is missing."
    }
    if ((Get-FileHash -LiteralPath $edition.Installed).Hash -ne $edition.ExpectedHash) {
        throw "$($edition.Name) installed payload differs from the portable executable."
    }
    if ((Get-FileHash -LiteralPath $edition.InstalledNotice).Hash -ne $edition.ExpectedNoticeHash) {
        throw "$($edition.Name) installed notices differ from the bundled notices."
    }
    if (-not (Test-Path -LiteralPath $edition.RegistryKey)) {
        throw "$($edition.Name) did not register its own per-user uninstall entry."
    }
}

function Assert-Launch($edition) {
    $frame = Join-Path $testRoot "$($edition.Name).png"
    $process = Start-Process -FilePath $edition.Installed -ArgumentList @(
        '--mode', 'standard', '--lang', 'en', '--theme', 'white', '--screenshot', ('"' + $frame + '"')
    ) -PassThru
    Wait-Child $process "Launch $($edition.Name)" 15000
    if (Test-Path -LiteralPath "$frame.error") {
        throw "Launch $($edition.Name) did not render: $([IO.File]::ReadAllText("$frame.error"))"
    }
    $bytes = [IO.File]::ReadAllBytes($frame)
    if ($bytes.Length -lt 100 -or [BitConverter]::ToString($bytes[0..7]) -ne '89-50-4E-47-0D-0A-1A-0A') {
        throw "Launch $($edition.Name) did not produce a PNG."
    }
}

foreach ($edition in $editions) {
    $dir = Join-Path $testRoot $edition.Name
    $log = Join-Path $testRoot "$($edition.Name).log"
    $edition.ExpectedHash = (Get-FileHash -LiteralPath $edition.Portable).Hash
    $edition.ExpectedNoticeHash = (Get-FileHash -LiteralPath $edition.Notice).Hash
    $edition.Installed = Join-Path $dir 'RusticCalc.exe'
    $edition.InstalledNotice = Join-Path $dir 'licenses\THIRD-PARTY-NOTICES.txt'
    $edition.Uninstaller = Join-Path $dir 'unins000.exe'
    $edition.RegistryKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$($edition.Id)_is1"
    if (Test-Path -LiteralPath $edition.RegistryKey) {
        throw "The disposable test profile already has $($edition.Name) installed."
    }
    $setup = Start-Process -FilePath ([IO.Path]::GetFullPath($edition.Installer)) -ArgumentList @(
        '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/NOICONS', '/TASKS=', ('/DIR="' + $dir + '"'), ('/LOG="' + $log + '"')
    ) -PassThru
    Wait-Child $setup "Install $($edition.Name)"
    $active.Add($edition)
    foreach ($survivor in $active) { Assert-Payload $survivor }
    if ((Profile-Snapshot) -cne $before) { throw 'Unchecked CMD task changed AutoRun.' }
}
$launchFailures = @()
if ($VerifyLaunch) {
    foreach ($edition in $editions) {
        try { Assert-Launch $edition }
        catch { $launchFailures += $_.Exception.Message }
    }
}
foreach ($edition in $editions) {
    $uninstall = Start-Process -FilePath $edition.Uninstaller -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART') -PassThru
    Wait-Child $uninstall "Uninstall $($edition.Name)"
    if (Test-Path -LiteralPath $edition.Installed) { throw "Uninstall left the $($edition.Name) executable." }
    if (Test-Path -LiteralPath $edition.InstalledNotice) { throw "Uninstall left the $($edition.Name) notices." }
    if (Test-Path -LiteralPath $edition.RegistryKey) { throw "Uninstall left the $($edition.Name) registry entry." }
    [void]$active.Remove($edition)
    foreach ($survivor in $active) { Assert-Payload $survivor }
    if ((Profile-Snapshot) -cne $before) { throw 'Uninstall changed an unowned CMD profile.' }
}
Write-Host "PASS: $(@($editions).Count) per-user installer(s), payload equality, independent removal, and no unsolicited CMD integration."
if ($launchFailures.Count -gt 0) {
    throw ($launchFailures -join "`n")
}
if ($VerifyLaunch) { Write-Host 'PASS: installed executables rendered a CLI screenshot.' }
