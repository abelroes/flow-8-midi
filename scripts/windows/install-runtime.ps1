#Requires -RunAsAdministrator
#Requires -Version 5.1
<#
.SYNOPSIS
  DEV ONLY — silent Program Files copy (no Start Menu, no wizard).

.DESCRIPTION
  End users must use FLOW8-MIDI-Setup-*.exe (GUI). Build it with package-setup.ps1.

  This script is a maintainer shortcut: copy stage → Program Files + register
  DirectHCI service. It does not create Start Menu shortcuts.
#>
[CmdletBinding()]
param(
    [ValidateSet('debug', 'release')]
    [string] $Profile = 'debug',

    [string] $StageDir = '',

    [string] $LibwdiDir = ''
)

$ErrorActionPreference = 'Stop'

function Get-NativePath([string] $Path) {
    $full = [System.IO.Path]::GetFullPath($Path)
    if (Test-Path -LiteralPath $full) {
        $resolved = Resolve-Path -LiteralPath $full
        if ($resolved.ProviderPath) { return $resolved.ProviderPath }
        return $resolved.Path
    }
    return $full
}

$repo = Get-NativePath (Join-Path $PSScriptRoot '..\..')

if (-not $StageDir) {
    $StageDir = Join-Path $repo "dist\windows\$Profile"
}
$StageDir = Get-NativePath $StageDir
$stageApp = Join-Path $StageDir 'app'
$stageDhci = Join-Path $StageDir 'directhci'

foreach ($dir in @($stageApp, $stageDhci)) {
    if (-not (Test-Path -LiteralPath $dir -PathType Container)) {
        throw "Stage incomplete: missing $dir. Build first (build-windows.ps1 or build-dev-wsl.sh)."
    }
}

$appDir = Join-Path $env:ProgramFiles 'FLOW 8 MIDI Controller'
$dhciDir = Join-Path $env:ProgramFiles 'DirectHCI'

function Stop-DirectHciService {
    $svc = Get-Service -Name 'DirectHCI' -ErrorAction SilentlyContinue
    if (-not $svc) { return }
    if ($svc.Status -ne 'Stopped') {
        Write-Host 'Stopping DirectHCI service...' -ForegroundColor Cyan
        Stop-Service -Name 'DirectHCI' -Force -ErrorAction Stop
        $svc.WaitForStatus('Stopped', '00:00:30')
    }
}

function Install-DirectHciService([string] $Daemon) {
    $svc = Get-Service -Name 'DirectHCI' -ErrorAction SilentlyContinue
    if ($svc) {
        Write-Host 'DirectHCI service already registered.' -ForegroundColor Green
        return
    }
    Write-Host "Registering service: $Daemon install-service" -ForegroundColor Cyan
    & $Daemon install-service
    if ($LASTEXITCODE -ne 0) {
        throw "directhcid install-service failed ($LASTEXITCODE)"
    }
}

$libwdiDll = $null
$libwdiCandidates = @()
if ($LibwdiDir) {
    $libwdiCandidates += (Join-Path $LibwdiDir 'libwdi.dll')
}
$libwdiCandidates += @(
    (Join-Path $StageDir 'libwdi\libwdi.dll'),
    (Join-Path $stageDhci 'libwdi.dll'),
    (Join-Path $env:LOCALAPPDATA 'DirectHCI\native\libwdi.dll'),
    (Join-Path $env:ProgramFiles 'DirectHCI\libwdi.dll')
)
foreach ($cand in $libwdiCandidates) {
    if ($cand -and (Test-Path -LiteralPath $cand -PathType Leaf)) {
        $libwdiDll = $cand
        break
    }
}
if (-not $libwdiDll) {
    Write-Warning @"
libwdi.dll not found — installing app + DirectHCI service anyway.
Bluetooth prepare will fail until libwdi is present (DirectHCI build-libwdi.ps1
under EWDK, or copy libwdi.dll into $env:ProgramFiles\DirectHCI\).
"@
}

Stop-DirectHciService

Write-Host "Installing app → $appDir" -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path $appDir | Out-Null
Copy-Item -Force (Join-Path $stageApp 'flow-8-midi.exe') $appDir
Copy-Item -Force (Join-Path $stageApp 'flow8-ble-helper.exe') $appDir

Write-Host "Installing DirectHCI → $dhciDir" -ForegroundColor Cyan
New-Item -ItemType Directory -Force -Path $dhciDir | Out-Null
foreach ($name in @(
        'directhcid.exe',
        'directhci.exe',
        'directhci-control-panel.exe',
        'directhci-ble.exe'
    )) {
    Copy-Item -Force (Join-Path $stageDhci $name) $dhciDir
}
if ($libwdiDll) {
    Copy-Item -Force $libwdiDll $dhciDir
    Write-Host "libwdi.dll ← $libwdiDll" -ForegroundColor Green
}

$daemon = Join-Path $dhciDir 'directhcid.exe'
Install-DirectHciService -Daemon $daemon

Write-Host ""
Write-Host 'OK — runtime installed (same paths as end-user Setup).' -ForegroundColor Green
Write-Host "  App:      $appDir\flow-8-midi.exe"
Write-Host "  DirectHCI: $dhciDir"
Write-Host 'Run the app from Program Files (Start Menu / path above).'
