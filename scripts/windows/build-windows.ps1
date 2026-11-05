#Requires -Version 5.1
<#
.SYNOPSIS
  Build Flow 8 + DirectHCI into a shared cargo output and stage tree (no Setup).

.DESCRIPTION
  Sibling layout:
    parent\flow-8-midi
    parent\DirectHCI

  Shared CARGO_TARGET_DIR → target\<triple>\<profile>\.
  Stage → dist\windows\<profile>\{app,directhci}.

  To produce the end-user GUI installer, use build-installer.ps1 instead.
#>
[CmdletBinding()]
param(
    [ValidateSet('debug', 'release')]
    [string] $Profile = 'release',

    [ValidateSet('x86_64-pc-windows-msvc', 'x86_64-pc-windows-gnu')]
    [string] $Target = 'x86_64-pc-windows-msvc',

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

function Invoke-Cargo {
    param([string] $WorkDir, [string[]] $CargoArgs)
    Write-Host "==> [$WorkDir] cargo $($CargoArgs -join ' ')" -ForegroundColor Cyan
    Push-Location $WorkDir
    try {
        & cargo @CargoArgs
        if ($LASTEXITCODE -ne 0) { throw "cargo failed ($LASTEXITCODE)" }
    } finally {
        Pop-Location
    }
}

$repo = Get-NativePath (Join-Path $PSScriptRoot '..\..')
$directHci = Get-NativePath (Join-Path $repo '..\DirectHCI')
$env:CARGO_TARGET_DIR = Join-Path $repo 'target'

$cargoProfile = @()
if ($Profile -eq 'release') { $cargoProfile = @('--release') }

Write-Host "CARGO_TARGET_DIR=$env:CARGO_TARGET_DIR"
Write-Host "DirectHCI=$directHci"

Invoke-Cargo $repo (@('build') + $cargoProfile + @('--target', $Target, '--bin', 'flow-8-midi'))
Invoke-Cargo (Join-Path $repo 'helper\flow8-ble-helper') (@('build') + $cargoProfile + @('--target', $Target))
Invoke-Cargo $directHci (@('build') + $cargoProfile + @(
        '--target', $Target,
        '-p', 'directhci',
        '-p', 'directhcid',
        '-p', 'directhci-control-panel',
        '-p', 'directhci-ble-cli'
    ))

$out = Join-Path $env:CARGO_TARGET_DIR "$Target\$Profile"
foreach ($name in @(
        'flow-8-midi.exe',
        'flow8-ble-helper.exe',
        'directhci.exe',
        'directhcid.exe',
        'directhci-control-panel.exe',
        'directhci-ble.exe'
    )) {
    if (-not (Test-Path -LiteralPath (Join-Path $out $name) -PathType Leaf)) {
        throw "Missing $out\$name after unified build"
    }
}

$stage = Join-Path $repo "dist\windows\$Profile"
$stageApp = Join-Path $stage 'app'
$stageDhci = Join-Path $stage 'directhci'
New-Item -ItemType Directory -Force -Path $stageApp, $stageDhci | Out-Null
Copy-Item -Force (Join-Path $out 'flow-8-midi.exe') $stageApp
Copy-Item -Force (Join-Path $out 'flow8-ble-helper.exe') $stageApp
foreach ($name in @(
        'directhci.exe',
        'directhcid.exe',
        'directhci-control-panel.exe',
        'directhci-ble.exe'
    )) {
    Copy-Item -Force (Join-Path $out $name) $stageDhci
}

if (-not $LibwdiDir) {
    $LibwdiDir = Join-Path $env:LOCALAPPDATA 'DirectHCI\native'
}
$libwdiDll = Join-Path $LibwdiDir 'libwdi.dll'
if (Test-Path -LiteralPath $libwdiDll -PathType Leaf) {
    $stageLibwdi = Join-Path $stage 'libwdi'
    New-Item -ItemType Directory -Force -Path $stageLibwdi | Out-Null
    Copy-Item -Force $libwdiDll $stageLibwdi
    Copy-Item -Force $libwdiDll $out
}

Write-Host "Unified cargo output: $out" -ForegroundColor Green
Write-Host "Stage: $stage\{app,directhci}" -ForegroundColor Green
Write-Host "GUI Setup: .\scripts\windows\build-installer.ps1 -Profile $Profile -Target $Target"
