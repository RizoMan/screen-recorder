; =========================================================================
; Screen Recorder Pro - Inno Setup Script
; High-performance, cross-platform screen recording utility for Windows
; Supports English & Spanish installation, brand icons, desktop,
; start menu, and clean uninstallation.
; =========================================================================

#define MyAppName "Screen Recorder Pro"
#define MyAppVersion "0.1.27"
#define MyAppPublisher "Screen Recorder Team"
#define MyAppURL "https://github.com/screenrecorder/screen-recorder"
#define MyAppExeName "rec-app.exe"

[Setup]
; Unique application GUID for updates & clean uninstall
AppId={{D37F28B0-6A09-4C9C-8A91-8D9C08D7901A}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppVerName={#MyAppName} v{#MyAppVersion}
AppPublisher={#MyAppPublisher}
AppPublisherURL={#MyAppURL}
AppSupportURL={#MyAppURL}
AppUpdatesURL={#MyAppURL}

; Destination installation folder
DefaultDirName={autopf}\{#MyAppName}
DefaultGroupName={#MyAppName}
AllowNoIcons=yes

; Output installer settings
OutputDir=.
OutputBaseFilename=ScreenRecorderPro_Setup_v{#MyAppVersion}
SetupIconFile=..\assets\installer.ico
UninstallDisplayIcon={app}\app.ico

; Classic Step-by-Step Wizard Configuration
WizardStyle=modern
DisableWelcomePage=no
DisableDirPage=no
DisableProgramGroupPage=no
DisableReadyPage=no
AlwaysShowDirOnReadyPage=yes
AlwaysShowGroupOnReadyPage=yes

; Maximum compression and architecture
Compression=lzma2/ultra64
SolidCompression=yes
ArchitecturesInstallIn64BitMode=x64compatible
PrivilegesRequiredOverridesAllowed=dialog
CloseApplications=yes
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"
Name: "spanish"; MessagesFile: "compiler:Languages\Spanish.isl"

[CustomMessages]
english.LaunchProgram=Launch Screen Recorder Pro now
spanish.LaunchProgram=Ejecutar Screen Recorder Pro ahora
english.AutoStart=Start Screen Recorder Pro automatically on Windows logon
spanish.AutoStart=Iniciar Screen Recorder Pro automáticamente al iniciar Windows

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"
Name: "autostart"; Description: "{cm:AutoStart}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\app.ico"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\assets\app_icon.png"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
; Start Menu shortcut
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app.ico"
; Desktop shortcut (optional task)
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app.ico"; Tasks: desktopicon
; Windows Startup shortcut (optional task)
Name: "{autostartup}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; IconFilename: "{app}\app.ico"; Tasks: autostart
; Uninstaller shortcut
Name: "{group}\{cm:UninstallProgram,{#MyAppName}}"; Filename: "{uninstallexe}"; IconFilename: "{app}\app.ico"

[UninstallDelete]
Type: filesandordirs; Name: "{userappdata}\Screen Recorder Pro"
Type: filesandordirs; Name: "{userappdata}\ScreenRecorderPro"

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "{cm:LaunchProgram}"; Flags: nowait postinstall skipifsilent

[Code]
// Before installing or upgrading, ensure running instance is terminated cleanly
function InitializeSetup(): Boolean;
var
  ResultCode: Integer;
begin
  Result := True;
  Exec('taskkill.exe', '/F /IM {#MyAppExeName}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
end;

// Before uninstalling, ensure running instance is terminated cleanly
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  ResultCode: Integer;
begin
  if CurUninstallStep = usUninstall then
  begin
    Exec('taskkill.exe', '/F /IM {#MyAppExeName}', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  end;
end;
