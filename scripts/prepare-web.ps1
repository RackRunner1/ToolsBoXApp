#Requires -Version 5.1
<#
.SYNOPSIS
    Prepares the web assets embedded in the Tauri app.

.DESCRIPTION
    Copies contents/ to app/web/ (the frontendDist configured in
    app/src-tauri/tauri.conf.json). Every file found there is embedded into
    the binary at build time: the produced exe is standalone.

    Run after scripts/sync-contents.ps1 and before `tauri build`.

.PARAMETER ContentsDir
    Source folder (default: <repo>/contents).

.PARAMETER WebDir
    Destination folder (default: <repo>/app/web).

.EXAMPLE
    .\scripts\prepare-web.ps1
#>
[CmdletBinding()]
param(
    [string]$ContentsDir = '',
    [string]$WebDir      = ''
)

$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path $PSScriptRoot -Parent
if (-not $ContentsDir) { $ContentsDir = Join-Path $RepoRoot 'contents' }
if (-not $WebDir)      { $WebDir      = Join-Path $RepoRoot 'app\web' }

if (-not (Test-Path (Join-Path $ContentsDir 'index.html'))) {
    throw "index.html is missing from '$ContentsDir' - run first: .\scripts\sync-contents.ps1"
}

# ---------------------------------------------------------------------------
# Full rebuild of app/web/
# ---------------------------------------------------------------------------
if (Test-Path $WebDir) { Remove-Item $WebDir -Recurse -Force }
New-Item -ItemType Directory -Path $WebDir -Force | Out-Null

$null = robocopy $ContentsDir $WebDir /E /NFL /NDL /NJH /NJS /NP
if ($LASTEXITCODE -ge 8) { throw "robocopy failed (code $LASTEXITCODE)" }
# robocopy returns 1-7 on success (files copied): reset the code to 0 or
# the CI step is reported as failed.
$global:LASTEXITCODE = 0

# ---------------------------------------------------------------------------
# Minimal checks
# ---------------------------------------------------------------------------
foreach ($f in @('index.html', 'style.css')) {
    if (-not (Test-Path (Join-Path $WebDir $f))) { throw "$f is missing from app/web/ - incomplete content?" }
}

$files = Get-ChildItem $WebDir -Recurse -File -Force
$sizeMb = [math]::Round((($files | Measure-Object Length -Sum).Sum / 1MB), 2)
Write-Host "OK: app/web/ ready ($($files.Count) files, $sizeMb MB) - embedded into the binary at build time"
# Safety net: make sure the script exits with code 0.
$global:LASTEXITCODE = 0
