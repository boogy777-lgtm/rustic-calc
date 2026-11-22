#ifndef AppVersion
  #error AppVersion must come from Cargo.toml via Build-Release.ps1
#endif
#ifndef AppEdition
  #error AppEdition must be explicitly set by Build-Release.ps1
#endif
#ifndef AppSourceExe
  #error AppSourceExe must point to the verified portable binary
#endif
#ifndef AppNoticeBundlePath
  #error AppNoticeBundlePath must point to the generated edition notice bundle
#endif
#ifndef AppOutputDir
  #define AppOutputDir "..\dist"
#endif

#if AppEdition == "Win32"
  #define EditionAppId "{{2C85D442-432B-42CD-987B-1738EAE56F13}"
  #define EditionAppName "Rustic Calc"
  #define EditionDirectoryName "Rustic Calc"
  #define EditionInstallerName "RusticCalc-Setup"
#elif AppEdition == "Slint"
  #define EditionAppId "{{A3CD8BF1-B96F-4FDB-9C1B-EFC1D55D8260}"
  #define EditionAppName "Rustic Calc (Slint)"
  #define EditionDirectoryName "Rustic Calc Slint"
  #define EditionInstallerName "RusticCalc-Slint-Setup"
#elif AppEdition == "GPUI"
  #define EditionAppId "{{8C701C5E-58BC-4E19-9A16-B8A04FF4B610}"
  #define EditionAppName "Rustic Calc (GPUI)"
  #define EditionDirectoryName "Rustic Calc GPUI"
  #define EditionInstallerName "RusticCalc-GPUI-Setup"
#else
  #error AppEdition must be Win32, Slint, or GPUI
#endif

[Setup]
AppId={#EditionAppId}
AppName={#EditionAppName}
AppVersion={#AppVersion}
AppPublisher=Rustic Calc contributors
AppPublisherURL=https://github.com/boogy777-lgtm/rustic-calc
AppSupportURL=https://github.com/boogy777-lgtm/rustic-calc/issues
AppUpdatesURL=https://github.com/boogy777-lgtm/rustic-calc/releases
DefaultDirName={localappdata}\Programs\{#EditionDirectoryName}
DefaultGroupName={#EditionAppName}
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#AppOutputDir}
OutputBaseFilename={#EditionInstallerName}
SetupIconFile=..\assets\calc.ico
UninstallDisplayIcon={app}\RusticCalc.exe
LicenseFile=..\LICENSE
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
VersionInfoVersion={#AppVersion}.0
CloseApplications=yes
RestartApplications=no
DisableProgramGroupPage=yes

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "russian"; MessagesFile: "compiler:Languages\Russian.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked
#if AppEdition == "Win32"
Name: "cmdalias"; Description: "Use calc in CMD (current user, reversible)"; Flags: unchecked
#endif

[Files]
Source: "{#AppSourceExe}"; DestDir: "{app}"; DestName: "RusticCalc.exe"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; DestName: "LICENSE.txt"; Flags: ignoreversion
Source: "..\NOTICE.md"; DestDir: "{app}"; DestName: "NOTICE.txt"; Flags: ignoreversion
Source: "..\third_party\geo-ts-msk-LICENSE.txt"; DestDir: "{app}\licenses"; DestName: "geo-ts-msk-LICENSE.txt"; Flags: ignoreversion
Source: "{#AppNoticeBundlePath}"; DestDir: "{app}\licenses"; DestName: "THIRD-PARTY-NOTICES.txt"; Flags: ignoreversion
#if AppEdition == "Win32"
Source: "..\scripts\Set-CalcCommand.ps1"; DestDir: "{app}\tools"; Flags: ignoreversion
Source: "..\scripts\setup-calc.cmd"; DestDir: "{app}\tools"; Flags: ignoreversion
Source: "..\scripts\README.md"; DestDir: "{app}\tools"; Flags: ignoreversion
#endif

[Icons]
Name: "{group}\{#EditionAppName}"; Filename: "{app}\RusticCalc.exe"
Name: "{autodesktop}\{#EditionAppName}"; Filename: "{app}\RusticCalc.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\RusticCalc.exe"; Description: "Launch {#EditionAppName}"; Flags: nowait postinstall skipifsilent

#if AppEdition == "Win32"
[UninstallRun]
Filename: "{sys}\WindowsPowerShell\v1.0\powershell.exe"; Parameters: "-NoLogo -NoProfile -ExecutionPolicy Bypass -File ""{app}\tools\Set-CalcCommand.ps1"" Uninstall -ExpectedSourcePath ""{app}\RusticCalc.exe"""; Check: OwnsCmdAlias; RunOnceId: "RusticCalcCmdAlias"; Flags: runhidden

[Code]
procedure CurStepChanged(CurStep: TSetupStep);
var ResultCode: Integer; Parameters: String;
begin
  if (CurStep = ssPostInstall) and WizardIsTaskSelected('cmdalias') then begin
    Parameters := '-NoLogo -NoProfile -ExecutionPolicy Bypass -File "' +
      ExpandConstant('{app}\tools\Set-CalcCommand.ps1') + '" Install -CalculatorPath "' +
      ExpandConstant('{app}\RusticCalc.exe') + '"';
    if Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'), Parameters,
      '', SW_HIDE, ewWaitUntilTerminated, ResultCode) and (ResultCode = 0) then
      RegWriteStringValue(HKCU, 'Software\RusticCalc\Installer', 'CmdAliasSource', ExpandConstant('{app}\RusticCalc.exe'))
    else
      MsgBox('Rustic Calc was installed, but CMD integration failed. Run tools\Set-CalcCommand.ps1 manually to inspect the error.', mbError, MB_OK);
  end;
end;

function OwnsCmdAlias: Boolean;
var Source: String;
begin
  Result := RegQueryStringValue(HKCU, 'Software\RusticCalc\Installer', 'CmdAliasSource', Source) and
    (CompareText(Source, ExpandConstant('{app}\RusticCalc.exe')) = 0);
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
begin
  if (CurUninstallStep = usPostUninstall) and OwnsCmdAlias then begin
    RegDeleteValue(HKCU, 'Software\RusticCalc\Installer', 'CmdAliasSource');
    RegDeleteKeyIfEmpty(HKCU, 'Software\RusticCalc\Installer');
  end;
end;
#endif
