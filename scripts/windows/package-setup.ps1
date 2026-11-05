#Requires -Version 5.1
<#
.SYNOPSIS
  Package an existing stage into FLOW8-MIDI-Setup-*.exe (thin wrapper).

.DESCRIPTION
  Equivalent to: build-installer.ps1 -SkipBuild
  Prefer build-installer.ps1 for a full build + Setup in one step.
#>
[CmdletBinding()]
param(
    [ValidateSet('debug', 'release')]
    [string] $Profile = 'release',

    [string] $LibwdiDir = '',
    [string] $LibwdiUrl = '',
    [switch] $OpenOutput
)

$ErrorActionPreference = 'Stop'
$script = Join-Path $PSScriptRoot 'build-installer.ps1'
$forward = @('-SkipBuild', '-Profile', $Profile)
if ($LibwdiDir) { $forward += @('-LibwdiDir', $LibwdiDir) }
if ($LibwdiUrl) { $forward += @('-LibwdiUrl', $LibwdiUrl) }
if ($OpenOutput) { $forward += '-OpenOutput' }
& $script @forward
if ($LASTEXITCODE -ne 0) { throw "build-installer.ps1 failed ($LASTEXITCODE)" }
