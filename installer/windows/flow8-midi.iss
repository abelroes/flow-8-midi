; FLOW 8 MIDI Controller — single Windows end-user Setup (GUI).
; Components: FLOW 8 (required) + DirectHCI Bluetooth (optional, default on).
;
; Built by: scripts\windows\build-installer.ps1
;   iscc /DStageDir=...\dist\windows\release installer\windows\flow8-midi.iss
;
; StageDir:
;   app\          flow-8-midi.exe, flow8-ble-helper.exe
;   directhci\    directhcid.exe, directhci.exe, control-panel, ble
;   libwdi\       libwdi.dll

#ifdef StageDir
  #define Flow8BinaryDir StageDir + "\app"
  #define DirectHciBinaryDir StageDir + "\directhci"
  #define DirectHciLibwdiDir StageDir + "\libwdi"
#endif

#ifndef Flow8BinaryDir
  #error Define StageDir (dist\windows\<profile>) or Flow8BinaryDir.
#endif
#ifndef DirectHciBinaryDir
  #error Define StageDir or DirectHciBinaryDir.
#endif
#ifndef DirectHciLibwdiDir
  #error Define StageDir or DirectHciLibwdiDir (folder with libwdi.dll).
#endif

#define MyAppName "FLOW 8 MIDI Controller"
#define MyAppVersion "2.0.0"
#define MyAppPublisher "Abel Rocha Espinosa"
#define MyAppExeName "flow-8-midi.exe"
#define DirectHciDir "{autopf}\DirectHCI"

[Setup]
AppId={{A7E3F2C1-8B94-4D6E-9F11-2C5D8E0A1B73}
AppName={#MyAppName}
AppVersion={#MyAppVersion}
AppPublisher={#MyAppPublisher}
DefaultDirName={autopf}\FLOW 8 MIDI Controller
DefaultGroupName={#MyAppName}
DisableDirPage=yes
PrivilegesRequired=admin
ArchitecturesAllowed=x64os
ArchitecturesInstallIn64BitMode=x64os
OutputBaseFilename=FLOW8-MIDI-Setup-{#MyAppVersion}
Compression=lzma
SolidCompression=yes
WizardStyle=modern
CloseApplications=no
RestartApplications=no
SetupIconFile=..\..\resources\icon.ico
UninstallDisplayIcon={app}\{#MyAppExeName}
LicenseFile=THIRD_PARTY_LICENSES.txt
InfoBeforeFile=WELCOME.txt

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Types]
Name: "full"; Description: "Full installation (app + Bluetooth)"
Name: "apponly"; Description: "FLOW 8 MIDI only (USB MIDI; no DirectHCI)"
Name: "custom"; Description: "Custom"; Flags: iscustom

[Components]
Name: "flow8"; Description: "FLOW 8 MIDI Controller (app + Bluetooth helper)"; Types: full apponly custom; Flags: fixed
Name: "directhci"; Description: "DirectHCI (Windows Bluetooth runtime)"; Types: full custom

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop icon"; GroupDescription: "Additional icons:"; Flags: unchecked; Components: flow8

[Files]
; --- App ---
Source: "{#Flow8BinaryDir}\{#MyAppExeName}"; DestDir: "{app}"; Flags: ignoreversion; Components: flow8
Source: "{#Flow8BinaryDir}\flow8-ble-helper.exe"; DestDir: "{app}"; Flags: ignoreversion; Components: flow8
Source: "licenses\*"; DestDir: "{app}\licenses"; Flags: ignoreversion recursesubdirs; Components: flow8
Source: "THIRD_PARTY_LICENSES.txt"; DestDir: "{app}"; Flags: ignoreversion; Components: flow8
Source: "WELCOME.txt"; DestDir: "{app}"; Flags: ignoreversion; Components: flow8

; --- DirectHCI (preflight always extracted when component selected; also used if already installed) ---
Source: "{#DirectHciBinaryDir}\directhcid.exe"; DestName: "directhcid-preflight.exe"; Flags: dontcopy noencryption
Source: "{#DirectHciBinaryDir}\directhcid.exe"; DestDir: "{#DirectHciDir}"; Flags: ignoreversion; Components: directhci
Source: "{#DirectHciBinaryDir}\directhci.exe"; DestDir: "{#DirectHciDir}"; Flags: ignoreversion; Components: directhci
Source: "{#DirectHciBinaryDir}\directhci-control-panel.exe"; DestDir: "{#DirectHciDir}"; Flags: ignoreversion; Components: directhci
Source: "{#DirectHciBinaryDir}\directhci-ble.exe"; DestDir: "{#DirectHciDir}"; Flags: ignoreversion; Components: directhci
Source: "{#DirectHciLibwdiDir}\libwdi.dll"; DestDir: "{#DirectHciDir}"; Flags: ignoreversion; Components: directhci
Source: "licenses\DirectHCI-GPL-3.0.txt"; DestDir: "{#DirectHciDir}"; DestName: "LICENSE.txt"; Flags: ignoreversion; Components: directhci

[Icons]
Name: "{group}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Components: flow8
Name: "{group}\DirectHCI Control Panel"; Filename: "{#DirectHciDir}\directhci-control-panel.exe"; Components: directhci
Name: "{group}\Uninstall {#MyAppName}"; Filename: "{uninstallexe}"; Components: flow8
Name: "{autodesktop}\{#MyAppName}"; Filename: "{app}\{#MyAppExeName}"; Tasks: desktopicon; Components: flow8

[Run]
Filename: "{app}\{#MyAppExeName}"; Description: "Launch {#MyAppName}"; Flags: nowait postinstall skipifsilent; Components: flow8

[Code]
#include "process-guard.iss"

function LastNonEmptyLine(const Lines: TArrayOfString): String;
var
  I: Integer;
begin
  Result := '';
  for I := 0 to GetArrayLength(Lines) - 1 do
    if Trim(Lines[I]) <> '' then
      Result := Lines[I];
end;

function RunDaemon(const Exe, Args: String; var ResultCode: Integer;
  var Details: String): Boolean;
var
  Output: TExecOutput;
begin
  Details := '';
  try
    Result := ExecAndCaptureOutput(Exe, Args, '', SW_SHOWNORMAL,
      ewWaitUntilTerminated, ResultCode, Output);
    if Result then begin
      Details := LastNonEmptyLine(Output.StdErr);
      if Details = '' then
        Details := LastNonEmptyLine(Output.StdOut);
      if Output.Error then
        Details := Details + ' (output capture incomplete; inspect Setup log)';
      if Details <> '' then Log(Exe + ' ' + Args + ': ' + Details);
    end else
      Details := SysErrorMessage(ResultCode);
  except
    Result := False;
    Details := GetExceptionMessage;
  end;
  if Length(Details) > 600 then Details := Copy(Details, 1, 600) + '...';
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
var
  ResultCode: Integer;
  Helper: String;
  Details: String;
begin
  Result := '';
  if not WizardIsComponentSelected('directhci') then
    Exit;
  Result := TryBeginDirectHciMaintenance;
  if Result <> '' then Exit;
  if not RegKeyExists(HKEY_LOCAL_MACHINE,
    'SYSTEM\CurrentControlSet\Services\DirectHCI') and
    not FileExists(ExpandConstant(
      '{commonappdata}\DirectHCI\ownership-journal-v1.json')) then
    Exit;
  ExtractTemporaryFile('directhcid-preflight.exe');
  Helper := ExpandConstant('{tmp}\directhcid-preflight.exe');
  if not RunDaemon(Helper, 'uninstall-service', ResultCode, Details) then
    Result := 'Cannot check existing DirectHCI service: ' + Details
  else if ResultCode <> 0 then
    Result := 'DirectHCI service/recovery preflight failed (exit ' +
      IntToStr(ResultCode) + '). No files were replaced. ' + Details +
      ' Close Setup before running standalone recovery commands.';
end;

procedure CurStepChanged(CurStep: TSetupStep);
var
  ResultCode: Integer;
  Exe: String;
  Details: String;
begin
  if not WizardIsComponentSelected('directhci') then
    Exit;
  if CurStep = ssInstall then begin
    Details := DirectHciRunningProblem;
    if Details <> '' then RaiseException(Details);
  end;
  if CurStep = ssPostInstall then begin
    Exe := ExpandConstant('{#DirectHciDir}\directhcid.exe');
    if not RunDaemon(Exe, 'install-service', ResultCode, Details) then
      RaiseException('Could not register DirectHCI service: ' + Details);
    if ResultCode <> 0 then
      RaiseException('DirectHCI service registration failed (exit ' +
        IntToStr(ResultCode) + '): ' + Details +
        '. Files remain for repair; no driver was changed.');
  end;
end;

function InitializeUninstall: Boolean;
var
  ResultCode: Integer;
  Exe: String;
  Details: String;
begin
  Exe := ExpandConstant('{#DirectHciDir}\directhcid.exe');
  if not FileExists(Exe) then begin
    Result := True;
    Exit;
  end;
  Result := BeginDirectHciMaintenance;
  if not Result then Exit;
  if not RunDaemon(Exe, 'uninstall-service', ResultCode, Details) then begin
    MsgBox('Could not stop DirectHCI or check Windows Bluetooth recovery: ' +
      Details, mbError, MB_OK);
    Result := False;
  end else if ResultCode <> 0 then begin
    MsgBox('Windows Bluetooth recovery was not confirmed: ' + Details +
      '. Uninstall is cancelled; run DirectHCI recovery as Administrator and retry.',
      mbError, MB_OK);
    Result := False;
  end;
end;
