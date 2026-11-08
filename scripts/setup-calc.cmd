@powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0Set-CalcCommand.ps1" %*
@if errorlevel 1 exit /b %errorlevel%
@if /i "%~1"=="Uninstall" exit /b 0
@if /i "%~1"=="Status" exit /b 0
@rem For the standard install, activate the launcher in this CMD immediately too.
@if "%~1"=="" if exist "%LOCALAPPDATA%\Calc1\Command\cmd-init.cmd" call "%LOCALAPPDATA%\Calc1\Command\cmd-init.cmd"
@if /i "%~1"=="Install" if "%~2"=="" if exist "%LOCALAPPDATA%\Calc1\Command\cmd-init.cmd" call "%LOCALAPPDATA%\Calc1\Command\cmd-init.cmd"
@exit /b 0
