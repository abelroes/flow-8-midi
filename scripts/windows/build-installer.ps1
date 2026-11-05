#Requires -Version 5.1
<#
.SYNOPSIS
  Build Flow 8 + DirectHCI and produce the end-user GUI Setup (FLOW8-MIDI-Setup-*.exe).

.DESCRIPTION
  Canonical Windows packaging entry (local dev and GitHub Actions release):

    1. Sibling layout: parent\flow-8-midi + parent\DirectHCI
    2. Shared CARGO_TARGET_DIR — app, helper, DirectHCI EXEs in one folder
    3. Stage dist\windows\<profile>\{app,directhci,libwdi}
    4. Compile installer\windows\flow8-midi.iss with Inno Setup 6

  End users run only the generated Setup (wizard, component selection, Start Menu).

.PARAMETER Profile
  Cargo / stage profile (debug or release). Default: release.

.PARAMETER Target
  Rust target triple. Default: x86_64-pc-windows-msvc.

.PARAMETER SkipBuild
  Skip cargo; package an existing stage (e.g. after build-dev-wsl.sh).

.PARAMETER LibwdiDir
  Folder containing libwdi.dll.

.PARAMETER LibwdiUrl
  Optional URL to download libwdi.dll (also reads env LIBWDI_DLL_URL).

.PARAMETER OpenOutput
  Open Explorer on the generated Setup.exe.
#>
[CmdletBinding()]
param(
    [ValidateSet('debug', 'release')]
    [string] $Profile = 'release',

    [ValidateSet('x86_64-pc-windows-msvc', 'x86_64-pc-windows-gnu')]
    [string] $Target = 'x86_64-pc-windows-msvc',

    [switch] $SkipBuild,

    [string] $LibwdiDir = '',

    [string] $LibwdiUrl = '',

    [switch] $OpenOutput
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

function Find-Iscc {
    $fromPath = Get-Command ISCC.exe -ErrorAction SilentlyContinue |
        Select-Object -ExpandProperty Source
    $candidates = @(
        $fromPath,
        (Join-Path ${env:ProgramFiles(x86)} 'Inno Setup 6\ISCC.exe'),
        (Join-Path $env:ProgramFiles 'Inno Setup 6\ISCC.exe')
    ) | Where-Object { $_ -and (Test-Path -LiteralPath $_ -PathType Leaf) }
    return $candidates | Select-Object -First 1
}

function Resolve-LibwdiDll {
    param(
        [string] $LibwdiDir,
        [string] $LibwdiUrl,
        [string] $StageDir
    )

    if (-not $LibwdiUrl) { $LibwdiUrl = $env:LIBWDI_DLL_URL }

    $candidates = @()
    if ($LibwdiDir) {
        $candidates += (Join-Path $LibwdiDir 'libwdi.dll')
    }
    $candidates += @(
        (Join-Path $StageDir 'libwdi\libwdi.dll'),
        (Join-Path $env:LOCALAPPDATA 'DirectHCI\native\libwdi.dll'),
        (Join-Path $env:ProgramFiles 'DirectHCI\libwdi.dll')
    )

    foreach ($c in $candidates) {
        if ($c -and (Test-Path -LiteralPath $c -PathType Leaf)) {
            return (Get-NativePath $c)
        }
    }

    if ($LibwdiUrl) {
        $downloadDir = Join-Path $env:LOCALAPPDATA 'DirectHCI\native'
        New-Item -ItemType Directory -Force -Path $downloadDir | Out-Null
        $dest = Join-Path $downloadDir 'libwdi.dll'
        Write-Host "Downloading libwdi.dll from LIBWDI_DLL_URL..." -ForegroundColor Cyan
        Invoke-WebRequest -Uri $LibwdiUrl -OutFile $dest
        if (-not (Test-Path -LiteralPath $dest -PathType Leaf)) {
            throw "Download failed: $dest"
        }
        return (Get-NativePath $dest)
    }

    throw @"
libwdi.dll not found (required inside the Setup package).

Provide one of:
  - -LibwdiDir <folder with libwdi.dll>
  - %LOCALAPPDATA%\DirectHCI\native\libwdi.dll (DirectHCI build-libwdi.ps1 / EWDK)
  - $env:ProgramFiles\DirectHCI\libwdi.dll
  - -LibwdiUrl / env LIBWDI_DLL_URL (GitHub Actions secret)
"@
}

$repo = Get-NativePath (Join-Path $PSScriptRoot '..\..')
$directHci = Get-NativePath (Join-Path $repo '..\DirectHCI')
$env:CARGO_TARGET_DIR = Join-Path $repo 'target'
$stage = Join-Path $repo "dist\windows\$Profile"

if (-not $SkipBuild) {
    if (-not (Test-Path -LiteralPath $directHci -PathType Container)) {
        throw "DirectHCI sibling missing at $directHci"
    }

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
    Write-Host "Stage: $stage\{app,directhci}" -ForegroundColor Green
}

$stage = Get-NativePath $stage
foreach ($sub in @('app', 'directhci')) {
    $p = Join-Path $stage $sub
    if (-not (Test-Path -LiteralPath $p -PathType Container)) {
        throw "Stage incomplete: $p — build first or omit -SkipBuild."
    }
}

$libwdiDll = Resolve-LibwdiDll -LibwdiDir $LibwdiDir -LibwdiUrl $LibwdiUrl -StageDir $stage
$stageLibwdi = Join-Path $stage 'libwdi'
New-Item -ItemType Directory -Force -Path $stageLibwdi | Out-Null
Copy-Item -Force $libwdiDll (Join-Path $stageLibwdi 'libwdi.dll')
Write-Host "libwdi.dll <- $libwdiDll" -ForegroundColor Green

$iscc = Find-Iscc
if (-not $iscc) {
    throw 'Inno Setup 6 ISCC.exe not found. Install from https://jrsoftware.org/isinfo.php'
}

$iss = Join-Path $repo 'installer\windows\flow8-midi.iss'
Write-Host "Compiling GUI Setup: $iscc" -ForegroundColor Cyan
Write-Host "  StageDir=$stage"
& $iscc "/DStageDir=$stage" "/O$stage" $iss
if ($LASTEXITCODE -ne 0) { throw "ISCC failed ($LASTEXITCODE)" }

$setup = Get-ChildItem -LiteralPath $stage -Filter 'FLOW8-MIDI-Setup-*.exe' |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if (-not $setup) { throw "Setup EXE not found under $stage" }

Write-Host ""
Write-Host "OK — end-user GUI installer:" -ForegroundColor Green
Write-Host "  $($setup.FullName)"
Write-Host "Run that file (wizard + component selection + Start Menu)."

if ($OpenOutput) {
    Start-Process explorer.exe "/select,$($setup.FullName)"
}
