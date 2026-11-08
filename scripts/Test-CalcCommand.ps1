[CmdletBinding()]
param([string] $CalculatorPath, [string] $WorkDirectory)
Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'
if (-not $CalculatorPath) { $CalculatorPath = Join-Path $PSScriptRoot '..\target\x86_64-pc-windows-msvc\release\calc1.exe' }
if (-not $WorkDirectory) { $WorkDirectory = Join-Path ([IO.Path]::GetTempPath()) ('rustic-calc-tests-' + [guid]::NewGuid().ToString('N')) }
$root = Join-Path ([IO.Path]::GetFullPath($WorkDirectory)) ([guid]::NewGuid().ToString('N'))
$keyRoot = 'Software\RusticCalc\InstallerTests\' + [guid]::NewGuid().ToString('N')
$profileKey = $keyRoot + '\CommandProcessor'
$install = Join-Path $root 'folder (spaces) & command'
$script = Join-Path $PSScriptRoot 'Set-CalcCommand.ps1'
$key = [Microsoft.Win32.Registry]::CurrentUser.CreateSubKey($profileKey)
$original = '@echo EXISTING_PROFILE'
$key.SetValue('AutoRun', $original, [Microsoft.Win32.RegistryValueKind]::ExpandString)
$key.Dispose()
function Assert($condition, [string] $message) { if (-not $condition) { throw $message } }
function Invoke-Setup([string] $action) {
    & $script $action -CalculatorPath $CalculatorPath -InstallDirectory $install -RegistrySubKey $profileKey
}
try {
    & $script Install -CalculatorPath $CalculatorPath -InstallDirectory $install -RegistrySubKey $profileKey -WhatIf
    Assert (-not (Test-Path -LiteralPath $install)) 'WhatIf created files.'
    Invoke-Setup Install
    $state = [IO.File]::ReadAllText((Join-Path $install 'install-state.json')) | ConvertFrom-Json
    Assert ($state.PreviousProfile.Value -ceq $original) 'Original AutoRun was not preserved.'
    Assert ($state.PreviousProfile.Kind -eq 'ExpandString') 'Registry type was not preserved.'
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($profileKey)
    $first = $key.GetValue('AutoRun', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    $key.Dispose()
    Invoke-Setup Install
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($profileKey)
    Assert ($key.GetValue('AutoRun', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) -ceq $first) 'Repeated install duplicated the hook.'
    $key.Dispose()
    $init = Join-Path $install 'cmd-init.cmd'
    $command = 'call "' + $init + '" & call "' + $init + '" & where calc'
    $resolved = @(& $env:COMSPEC /d /c $command)
    Assert ($LASTEXITCODE -eq 0) 'CMD command resolution failed.'
    Assert ($resolved[0] -eq (Join-Path $install 'calc.cmd')) 'calc did not resolve to the managed launcher first.'
    # Test the real profile body without touching the user's actual AutoRun key.
    $profileOutput = @(& $env:COMSPEC /d /c ($first + ' & where calc') | ForEach-Object { $_.Trim() })
    Assert ($LASTEXITCODE -eq 0) 'Combined AutoRun body failed in CMD.'
    Assert ($profileOutput -contains 'EXISTING_PROFILE') 'Existing profile was skipped.'
    Assert ($profileOutput -contains (Join-Path $install 'calc.cmd')) 'Profile body did not activate calc.'
    Invoke-Setup Uninstall
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($profileKey)
    Assert ($key.GetValue('AutoRun', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) -ceq $original) 'Uninstall failed to restore AutoRun.'
    Assert ($key.GetValueKind('AutoRun').ToString() -eq 'ExpandString') 'Uninstall changed the registry type.'
    $key.Dispose()
    Invoke-Setup Install
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($profileKey, $true)
    $current = $key.GetValue('AutoRun', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
    $key.SetValue('AutoRun', $current + ' & @echo USER_ADDITION', [Microsoft.Win32.RegistryValueKind]::ExpandString)
    $key.Dispose()
    Invoke-Setup Uninstall
    $key = [Microsoft.Win32.Registry]::CurrentUser.OpenSubKey($profileKey)
    Assert ($key.GetValue('AutoRun', '', [Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames) -ceq ($original + ' & @echo USER_ADDITION')) 'User addition was lost.'
    $key.Dispose()
    Invoke-Setup Install
    & $script Uninstall -InstallDirectory $install -RegistrySubKey $profileKey -ExpectedSourcePath (Join-Path $root 'another-source.exe')
    Assert (Test-Path -LiteralPath (Join-Path $install 'calc.cmd')) 'Uninstall detached another source.'
    & $script Uninstall -InstallDirectory $install -RegistrySubKey $profileKey -ExpectedSourcePath ([IO.Path]::GetFullPath($CalculatorPath))
    Assert (-not (Test-Path -LiteralPath (Join-Path $install 'calc.cmd'))) 'Owned-source uninstall failed.'
    Write-Host 'PASS: isolated install/update/uninstall, registry backup, CMD resolution, WhatIf, user edits and source ownership.'
} finally {
    [Microsoft.Win32.Registry]::CurrentUser.DeleteSubKeyTree($keyRoot, $false)
}
