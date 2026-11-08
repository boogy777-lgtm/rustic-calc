[CmdletBinding(SupportsShouldProcess = $true)]
param(
    [Parameter(Position = 0)]
    [ValidateSet('Install', 'Uninstall', 'Status')]
    [string] $Action = 'Install',
    [string] $CalculatorPath,
    [string] $InstallDirectory,
    [string] $ExpectedSourcePath,
    [Parameter(DontShow = $true)]
    [string] $RegistrySubKey = 'Software\Microsoft\Command Processor'
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $CalculatorPath) { $CalculatorPath = Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\release\calc1.exe' }
if (-not $InstallDirectory) { $InstallDirectory = Join-Path $env:LOCALAPPDATA 'Calc1\Command' }
$owner = 'Calc1.CmdIntegration.v1'
$InstallDirectory = [IO.Path]::GetFullPath($InstallDirectory).TrimEnd('\')
$statePath = Join-Path $InstallDirectory 'install-state.json'
$initPath = Join-Path $InstallDirectory 'cmd-init.cmd'
$hook = 'if exist "' + $initPath + '" call "' + $initPath + '"'
$ownedNames = @('calc1.exe', 'calc.cmd', 'cmd-init.cmd')
$utf8 = New-Object Text.UTF8Encoding($false)

function Get-Profile {
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($RegistrySubKey)
    try {
        if ($null -eq $key -or @($key.GetValueNames()) -notcontains 'AutoRun') {
            return [pscustomobject]@{ Exists = $false; Kind = 'String'; Value = '' }
        }
        return [pscustomobject]@{
            Exists = $true
            Kind = $key.GetValueKind('AutoRun').ToString()
            Value = [string]$key.GetValue('AutoRun', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
        }
    } finally { if ($null -ne $key) { $key.Dispose() } }
}

function Set-Profile($profile) {
    if ($profile.Exists) {
        if ($profile.Kind -notin @('String', 'ExpandString')) { throw 'Unsupported AutoRun registry type.' }
        $key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($RegistrySubKey)
        try { $key.SetValue('AutoRun', [string]$profile.Value, [Enum]::Parse([Microsoft.Win32.RegistryValueKind], [string]$profile.Kind)) }
        finally { $key.Dispose() }
    } else {
        $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($RegistrySubKey, $true)
        if ($null -ne $key) {
            try { $key.DeleteValue('AutoRun', $false) } finally { $key.Dispose() }
        }
    }
}

function Get-State {
    if (-not [IO.File]::Exists($statePath)) { return $null }
    $state = [IO.File]::ReadAllText($statePath, [Text.Encoding]::UTF8) | ConvertFrom-Json
    if ($state.Owner -ne $owner -or $state.RegistrySubKey -ne $RegistrySubKey -or $state.InstallDirectory -ne $InstallDirectory) {
        throw 'Installer state does not belong to this location/profile. Nothing was changed.'
    }
    return $state
}

function Get-Hash([string] $path) { return (Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() }
function Get-TextHash([string] $text) {
    $sha = [Security.Cryptography.SHA256]::Create()
    try { return ([BitConverter]::ToString($sha.ComputeHash($utf8.GetBytes($text)))).Replace('-', '').ToLowerInvariant() }
    finally { $sha.Dispose() }
}

function Get-KnownHashes($state, [string] $name) {
    if ($null -eq $state) { return @() }
    $property = $state.KnownFileHashes.PSObject.Properties[$name]
    if ($null -eq $property) { throw "Missing ownership record for $name." }
    return @($property.Value)
}

function Test-HasHook([string] $value) {
    return $value.IndexOf($hook, [StringComparison]::OrdinalIgnoreCase) -ge 0
}

function Remove-Hook([string] $value) {
    if ($value.Equals($hook, [StringComparison]::OrdinalIgnoreCase)) { return '' }
    $suffix = ' & ' + $hook
    $prefix = $hook + ' & '
    if ($value.EndsWith($suffix, [StringComparison]::OrdinalIgnoreCase)) { return $value.Substring(0, $value.Length - $suffix.Length) }
    if ($value.StartsWith($prefix, [StringComparison]::OrdinalIgnoreCase)) { return $value.Substring($prefix.Length) }
    $middle = ' & ' + $hook + ' & '
    $index = $value.IndexOf($middle, [StringComparison]::OrdinalIgnoreCase)
    if ($index -ge 0) { return $value.Remove($index, $suffix.Length) }
    throw 'AutoRun was edited and its Calc1 hook cannot be safely detached. Leave files in place and inspect AutoRun manually.'
}

try {
    $state = Get-State
    $profile = Get-Profile
    $registered = Test-HasHook $profile.Value
    if ($Action -eq 'Status') {
        [pscustomobject]@{
            Registered = $registered
            LauncherExists = [IO.File]::Exists((Join-Path $InstallDirectory 'calc.cmd'))
            CalculatorExists = [IO.File]::Exists((Join-Path $InstallDirectory 'calc1.exe'))
            InstallDirectory = $InstallDirectory
            Profile = 'HKCU\' + $RegistrySubKey + '\AutoRun'
            BackupExists = $null -ne $state
        }
        return
    }

    if ($Action -eq 'Uninstall') {
        if ($null -eq $state) {
            if ($registered) { throw 'Hook exists but the ownership/backup file is missing. Nothing was removed.' }
            Write-Host 'Calc1 command integration is not installed.'
            return
        }
        if ($ExpectedSourcePath) {
            $sourceProperty = $state.PSObject.Properties['SourceExecutable']
            if ($null -eq $sourceProperty -or $sourceProperty.Value -ne [IO.Path]::GetFullPath($ExpectedSourcePath)) {
                Write-Warning 'CMD integration belongs to another source; leaving it unchanged.'
                return
            }
        }
        if (-not $PSCmdlet.ShouldProcess($InstallDirectory, 'Detach Calc1 CMD hook and remove unchanged installer-owned files')) { return }
        if ($registered) {
            if ($profile.Kind -eq $state.InstalledProfile.Kind -and $profile.Value -ceq $state.InstalledProfile.Value) {
                Set-Profile $state.PreviousProfile
            } else {
                # Preserve commands added/edited by the user after installation.
                $remaining = Remove-Hook $profile.Value
                $exists = $remaining.Length -gt 0 -or [bool]$state.PreviousProfile.Exists
                Set-Profile ([pscustomobject]@{ Exists = $exists; Kind = $profile.Kind; Value = $remaining })
            }
        }
        $retained = $false
        foreach ($name in $ownedNames) {
            $path = Join-Path $InstallDirectory $name
            if (-not [IO.File]::Exists($path)) { continue }
            if ((Get-KnownHashes $state $name) -notcontains (Get-Hash $path)) {
                Write-Warning "Modified file retained: $path"
                $retained = $true
                continue
            }
            try { Remove-Item -LiteralPath $path -ErrorAction Stop }
            catch { Write-Warning "File retained (possibly running): $path"; $retained = $true }
        }
        if (-not $retained) {
            Remove-Item -LiteralPath $statePath
            if (@(Get-ChildItem -LiteralPath $InstallDirectory -Force).Count -eq 0) { Remove-Item -LiteralPath $InstallDirectory }
        }
        Write-Host 'Calc1 CMD hook removed. Open a new CMD window to use the original calc command.'
        return
    }

    $CalculatorPath = [IO.Path]::GetFullPath($CalculatorPath)
    if (-not [IO.File]::Exists($CalculatorPath)) { throw "Calculator not found: $CalculatorPath" }
    if ($CalculatorPath -eq (Join-Path $InstallDirectory 'calc1.exe')) { throw 'Use the project executable as the source, not the managed installed copy.' }
    if ($InstallDirectory -match '[%!^"\r\n]') { throw 'The install path contains characters unsupported by CMD AutoRun (% ! ^ or quotes). Choose a simple -InstallDirectory.' }
    if ($profile.Exists -and $profile.Kind -notin @('String', 'ExpandString')) { throw 'AutoRun is not a string. Nothing was changed.' }
    if ($null -eq $state -and $registered) { throw 'An existing Calc1 hook has no backup metadata. Nothing was overwritten.' }
    foreach ($name in $ownedNames) {
        $path = Join-Path $InstallDirectory $name
        if ([IO.File]::Exists($path)) {
            if ($null -eq $state -or (Get-KnownHashes $state $name) -notcontains (Get-Hash $path)) {
                throw "Refusing to overwrite an unowned/modified file: $path"
            }
            if ((Get-Item -LiteralPath $path).IsReadOnly) { throw "Managed file is read-only: $path" }
        }
    }

    $prefixLength = ($InstallDirectory + '\').Length
    # ASCII batch bodies: no code-page dependency for Unicode user directories.
    $init = @(
        '@rem Calc1: process-local PATH only. No system files or persistent PATH edits.',
        '@if not exist "%~dp0calc1.exe" exit /b 0',
        ('@if /i "%PATH:~0,' + $prefixLength + '%"=="%~dp0" exit /b 0'),
        '@set "PATH=%~dp0;%PATH%"',
        '@exit /b 0', ''
    ) -join "`r`n"
    $launcher = @(
        '@rem Calc1 CMD launcher. START keeps the prompt available.',
        '@start "" "%~dp0calc1.exe" %*',
        '@exit /b %errorlevel%', ''
    ) -join "`r`n"
    $previous = $profile
    if ($registered) { $previous = $state.PreviousProfile }
    $installed = $profile.Value
    if (-not $registered) {
        if ($installed.Length -gt 0) { $installed += ' & ' }
        $installed += $hook
    }
    $kind = 'String'
    if ($profile.Exists) { $kind = $profile.Kind }
    $newHashes = @{ 'calc1.exe' = (Get-Hash $CalculatorPath); 'calc.cmd' = (Get-TextHash $launcher); 'cmd-init.cmd' = (Get-TextHash $init) }
    $known = [ordered]@{}
    foreach ($name in $ownedNames) { $known[$name] = @(@(Get-KnownHashes $state $name) + $newHashes[$name] | Select-Object -Unique) }
    $newState = [ordered]@{
        Owner = $owner; InstallDirectory = $InstallDirectory; RegistrySubKey = $RegistrySubKey
        SourceExecutable = $CalculatorPath
        PreviousProfile = $previous
        InstalledProfile = [pscustomobject]@{ Exists = $true; Kind = $kind; Value = $installed }
        KnownFileHashes = $known
    }
    if (-not $PSCmdlet.ShouldProcess($InstallDirectory, 'Install user-local calc launcher and append its CMD AutoRun hook')) { return }
    [IO.Directory]::CreateDirectory($InstallDirectory) | Out-Null
    # Recovery/ownership state is saved first, even if a later copy is interrupted.
    [IO.File]::WriteAllText($statePath, ($newState | ConvertTo-Json -Depth 8), $utf8)
    $exePath = Join-Path $InstallDirectory 'calc1.exe'
    if (-not [IO.File]::Exists($exePath) -or (Get-Hash $exePath) -ne $newHashes['calc1.exe']) {
        Copy-Item -LiteralPath $CalculatorPath -Destination $exePath -Force
    }
    [IO.File]::WriteAllText((Join-Path $InstallDirectory 'calc.cmd'), $launcher, $utf8)
    [IO.File]::WriteAllText($initPath, $init, $utf8)
    foreach ($name in $ownedNames) {
        if ((Get-Hash (Join-Path $InstallDirectory $name)) -ne $newHashes[$name]) { throw "Installed file verification failed: $name" }
    }
    Set-Profile $newState.InstalledProfile
    Write-Host "Installed: $InstallDirectory"
    Write-Host 'Open a new CMD and run: calc'
    Write-Host 'System calc.exe, WindowsApps aliases and the persistent PATH were not modified.'
} catch {
    Write-Error $_
    exit 1
}
