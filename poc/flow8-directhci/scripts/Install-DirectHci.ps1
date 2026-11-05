#Requires -RunAsAdministrator
<#
.SYNOPSIS
  Silent DirectHCI install + start service + Prepare Controller + wait for named pipe.

.DESCRIPTION
  Same flow a future FLOW 8 MIDI installer would run. Place this script next to
  DirectHCI-Setup-*.exe (or pass -SetupPath). Default points at the sibling
  DirectHCI release build: ../../../../DirectHCI/target/release/build/DirectHCI-Setup-V2.0.1.exe

  Prepare Controller is NOT exposed by directhci.exe CLI. This script calls
  flow8-directhci-prepare.exe (Rust helper using DirectHCI IPC), built from
  ../ with: cargo build --release --bin flow8-directhci-prepare

.PARAMETER SetupPath
  Path to DirectHCI Inno Setup installer.

.PARAMETER PrepareExe
  Path to flow8-directhci-prepare.exe. Default: sibling release build.

.PARAMETER ControllerId
  Optional DirectHCI controller id. If omitted and exactly one controller exists, it is used.

.PARAMETER SkipInstall
  Skip running the Setup EXE (service already installed).

.PARAMETER SkipPrepare
  Only install + start + wait for pipe (no Prepare Controller).

.PARAMETER PipeTimeoutSec
  Max seconds to wait for \\.\pipe\DirectHCI\v1 (default 60).
#>
[CmdletBinding()]
param(
    [string] $SetupPath = (Join-Path $PSScriptRoot '..\..\..\..\DirectHCI\target\release\build\DirectHCI-Setup-V2.0.1.exe'),
    [string] $PrepareExe = (Join-Path $PSScriptRoot '..\target\release\flow8-directhci-prepare.exe'),
    [string] $ControllerId,
    [switch] $SkipInstall,
    [switch] $SkipPrepare,
    [int] $PipeTimeoutSec = 60
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$ServiceName = 'DirectHCI'
$PipeName = '\\.\pipe\DirectHCI\v1'
$DirectHciExe = Join-Path ${env:ProgramFiles} 'DirectHCI\directhci.exe'
$DirectHcidExe = Join-Path ${env:ProgramFiles} 'DirectHCI\directhcid.exe'

function Write-Step([string] $Message) {
    Write-Host "[DirectHCI] $Message" -ForegroundColor Cyan
}

function Test-NamedPipe([string] $Path) {
    # Test-Path is unreliable for named pipes on some hosts; probe via Win32 open.
    try {
        $fs = [System.IO.File]::Open(
            $Path,
            [System.IO.FileMode]::Open,
            [System.IO.FileAccess]::ReadWrite,
            [System.IO.FileShare]::ReadWrite
        )
        $fs.Dispose()
        return $true
    } catch {
        return $false
    }
}

function Wait-DirectHciPipe([int] $TimeoutSec) {
    Write-Step "Waiting for named pipe $PipeName (timeout ${TimeoutSec}s)..."
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    $attempt = 0
    while ((Get-Date) -lt $deadline) {
        $attempt++
        if (Test-NamedPipe $PipeName) {
            Write-Step "Pipe ready after ${attempt} attempt(s)."
            return
        }
        Start-Sleep -Milliseconds 500
    }
    throw "Timed out waiting for $PipeName. Is service '$ServiceName' running? Try: Get-Service $ServiceName"
}

function Install-DirectHciSetup([string] $Installer) {
    if (-not (Test-Path -LiteralPath $Installer)) {
        throw "Installer not found: $Installer"
    }
    Write-Step "Running silent setup: $Installer"
    # Inno Setup: /VERYSILENT skips wizard; /SUPPRESSMSGBOXES avoids prompts;
    # /NORESTART never reboots mid-script; /SP- skips "This will install..." prompt.
    $args = @(
        '/VERYSILENT',
        '/SUPPRESSMSGBOXES',
        '/NORESTART',
        '/SP-',
        '/LOG=' + (Join-Path $env:TEMP 'DirectHCI-Setup.log')
    )
    $p = Start-Process -FilePath $Installer -ArgumentList $args -Wait -PassThru
    if ($p.ExitCode -ne 0) {
        throw "Setup failed with exit code $($p.ExitCode). See $env:TEMP\DirectHCI-Setup.log"
    }
    if (-not (Test-Path -LiteralPath $DirectHcidExe)) {
        throw "Setup finished but missing $DirectHcidExe"
    }
    Write-Step "Installed under $env:ProgramFiles\DirectHCI"
}

function Start-DirectHciService {
    $svc = Get-Service -Name $ServiceName -ErrorAction SilentlyContinue
    if (-not $svc) {
        # Fallback: register via daemon (same as Inno PostInstall)
        if (-not (Test-Path -LiteralPath $DirectHcidExe)) {
            throw "Service '$ServiceName' not registered and $DirectHcidExe missing"
        }
        Write-Step "Registering service via directhcid install-service..."
        & $DirectHcidExe install-service
        if ($LASTEXITCODE -ne 0) {
            throw "directhcid install-service failed (exit $LASTEXITCODE)"
        }
        $svc = Get-Service -Name $ServiceName -ErrorAction Stop
    }

    if ($svc.Status -ne 'Running') {
        Write-Step "Starting service $ServiceName..."
        # Prefer SCM API; sc.exe is the fallback for older hosts
        try {
            Start-Service -Name $ServiceName -ErrorAction Stop
        } catch {
            Write-Step "Start-Service failed ($($_.Exception.Message)); trying sc.exe..."
            & sc.exe start $ServiceName | Out-Host
            if ($LASTEXITCODE -ne 0 -and $LASTEXITCODE -ne 1056) {
                # 1056 = already starting/running
                throw "sc.exe start $ServiceName failed (exit $LASTEXITCODE)"
            }
        }
        $svc.Refresh()
        $svc.WaitForStatus('Running', [TimeSpan]::FromSeconds(30))
    } else {
        Write-Step "Service already Running."
    }
}

function Invoke-PrepareController {
    if (-not (Test-Path -LiteralPath $PrepareExe)) {
        throw @"
Prepare helper not found: $PrepareExe

Build it (Windows native):
  cd ..
  cargo build --release --bin flow8-directhci-prepare

Why a helper? directhci.exe has NO 'prepare' subcommand - only the Control Panel
calls DirectHciClient::prepare_controller over the named pipe. This helper does the same.
"@
    }

    Write-Step "Prepare Controller via $PrepareExe"
    $prepArgs = @('--trust-certificate')
    if ($ControllerId) {
        $prepArgs += @('--controller', $ControllerId)
    }
    & $PrepareExe @prepArgs
    if ($LASTEXITCODE -ne 0) {
        throw "flow8-directhci-prepare failed (exit $LASTEXITCODE)"
    }
}

function Show-Status {
    if (Test-Path -LiteralPath $DirectHciExe) {
        Write-Step "directhci status:"
        & $DirectHciExe status
        Write-Step "directhci controllers:"
        & $DirectHciExe controllers
    }
}

# --- main ---
Write-Step "Elevated=$([Security.Principal.WindowsIdentity]::GetCurrent().Name)"

if (-not $SkipInstall) {
    Install-DirectHciSetup -Installer $SetupPath
} else {
    Write-Step "SkipInstall: not running Setup EXE"
}

Start-DirectHciService
Wait-DirectHciPipe -TimeoutSec $PipeTimeoutSec

if (-not $SkipPrepare) {
    Invoke-PrepareController
} else {
    Write-Step "SkipPrepare: controller not prepared"
}

Show-Status
Write-Step "DONE - pipe is up. Next: run flow8-directhci-poc.exe (still elevated)."
exit 0
