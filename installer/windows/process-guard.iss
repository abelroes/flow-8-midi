{ Included inside [Code]. Keep these names in sync with process_lifecycle.rs.
  Setup holds the startup gate through its entire lifetime, including recovery
  preflight. Normal apps hold it only while publishing ActiveApplications.
  Neither recovery helper command acquires this gate. }
const
  DhciStartupGate = 'Global\DirectHCI.StartupGate.v1';
  DhciActiveApplications = 'Global\DirectHCI.ActiveApplications.v1';
  DhciWaitObject = 0;
  DhciWaitAbandoned = $80;
  DhciSynchronize = $00100000;

var
  DhciMaintenanceHandle: THandle;

function DhciCreateMutex(Attributes: DWORD_PTR; InitialOwner: BOOL;
  const Name: String): THandle;
  external 'CreateMutexW@kernel32.dll stdcall';
function DhciOpenMutex(Access: DWORD; Inherit: BOOL;
  const Name: String): THandle;
  external 'OpenMutexW@kernel32.dll stdcall';
function DhciWaitForSingleObject(Handle: THandle; Milliseconds: DWORD): DWORD;
  external 'WaitForSingleObject@kernel32.dll stdcall';
function DhciReleaseMutex(Handle: THandle): BOOL;
  external 'ReleaseMutex@kernel32.dll stdcall';
function DhciCloseHandle(Handle: THandle): BOOL;
  external 'CloseHandle@kernel32.dll stdcall';
function DhciGetLastError: DWORD;
  external 'GetLastError@kernel32.dll stdcall';

procedure EndDirectHciMaintenance;
begin
  if DhciMaintenanceHandle <> 0 then begin
    DhciReleaseMutex(DhciMaintenanceHandle);
    DhciCloseHandle(DhciMaintenanceHandle);
    DhciMaintenanceHandle := 0;
  end;
end;

function DirectHciRunningProblem: String;
var
  Locator, Services, Items, Item: Variant;
  ProcessName, ServiceState: String;
  ProcessId: Integer;
  Handle: THandle;
  ErrorCode: DWORD;
begin
  Result := '';
  try
    { Machine-wide inspection, including other sessions and old builds that do
      not publish the activity marker. Exact executable names, never substrings.
      Do not inspect/kill arbitrary consumer applications: a running daemon is
      enough to block maintenance while any consumer session could be active. }
    Locator := CreateOleObject('WbemScripting.SWbemLocator');
    Services := Locator.ConnectServer('', 'root\CIMV2');
    Items := Services.ExecQuery(
      'SELECT Name, ProcessId FROM Win32_Process WHERE ' +
      'Name="directhci-control-panel.exe" OR Name="directhcid.exe" OR ' +
      'Name="directhci.exe" OR Name="directhci-ble.exe"');
    if Items.Count > 0 then begin
      Item := Items.ItemIndex(0);
      ProcessName := Item.Name;
      ProcessId := Item.ProcessId;
      Result := 'DirectHCI is still running: ' + ProcessName +
        ' (PID ' + IntToStr(ProcessId) + '). Close consumers and the ' +
        'Control Panel normally, stop the DirectHCI service, and retry. ' +
        'Minimizing the panel to the tray does not exit it. ' +
        'No process will be terminated automatically.';
      Exit;
    end;
    Items := Services.ExecQuery('SELECT State FROM Win32_Service WHERE Name="DirectHCI"');
    if Items.Count > 0 then begin
      Item := Items.ItemIndex(0);
      ServiceState := Item.State;
      if CompareText(ServiceState, 'Stopped') <> 0 then begin
        Result := 'The DirectHCI service is ' + ServiceState +
          '. Stop it normally and wait for it to exit before installing or uninstalling.';
        Exit;
      end;
    end;
  except
    Result := 'Cannot verify DirectHCI running processes/service: ' +
      GetExceptionMessage + '. No installation files will be changed.';
    Exit;
  end;

  { Also detects renamed current-build executables, in every Windows session. }
  Handle := DhciOpenMutex(DhciSynchronize, False, DhciActiveApplications);
  if Handle <> 0 then begin
    DhciCloseHandle(Handle);
    Result := 'A DirectHCI application is still active. Close it normally and retry.';
  end else begin
    ErrorCode := DhciGetLastError;
    if ErrorCode <> 2 then
      Result := 'Cannot verify DirectHCI application activity: ' + SysErrorMessage(ErrorCode);
  end;
end;

function TryBeginDirectHciMaintenance: String;
var
  Handle: THandle;
  WaitResult: DWORD;
begin
  Result := '';
  if DhciMaintenanceHandle = 0 then begin
    Handle := DhciCreateMutex(0, False, DhciStartupGate);
    if Handle = 0 then begin
      Result := 'Cannot open DirectHCI installation guard: ' +
        SysErrorMessage(DhciGetLastError);
      Exit;
    end;
    WaitResult := DhciWaitForSingleObject(Handle, 2000);
    if (WaitResult <> DhciWaitObject) and (WaitResult <> DhciWaitAbandoned) then begin
      DhciCloseHandle(Handle);
      Result := 'Another DirectHCI setup/uninstall or startup is in progress. Wait for it to finish and retry.';
      Exit;
    end;
    DhciMaintenanceHandle := Handle;
  end;
  Result := DirectHciRunningProblem;
  if Result <> '' then EndDirectHciMaintenance;
end;

function BeginDirectHciMaintenance: Boolean;
var
  Details: String;
begin
  Result := False;
  repeat
    Details := TryBeginDirectHciMaintenance;
    if Details = '' then begin
      Result := True;
      Exit;
    end;
    { The gate has been released before prompting: the user can still perform
      normal service stop/recovery. Silent setup cancels instead of waiting. }
    Log(Details);
    if IsUninstaller then begin
      if UninstallSilent then Exit;
    end else begin
      if WizardSilent then Exit;
    end;
    if SuppressibleMsgBox(Details, mbError, MB_RETRYCANCEL, IDCANCEL) <> IDRETRY then Exit;
  until False;
end;

function InitializeSetup: Boolean;
begin
  Result := BeginDirectHciMaintenance;
end;

procedure DeinitializeSetup;
begin
  EndDirectHciMaintenance;
end;

procedure DeinitializeUninstall;
begin
  EndDirectHciMaintenance;
end;

procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Details: String;
begin
  if CurUninstallStep = usUninstall then begin
    { Recheck legacy processes immediately before file removal. Legacy binaries
      do not cooperate with the startup gate; never treat their stop as implicit. }
    Details := DirectHciRunningProblem;
    if Details <> '' then RaiseException(Details);
  end;
end;
