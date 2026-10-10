#Requires -Version 5.1
[CmdletBinding()]
param(
  [string]$SourceRepo = 'https://github.com/RackRunner1/ToolsBoXMachine.git',
  [string]$Branch     = 'main',
  [string]$Dest       = '',
  [switch]$KeepClone
)

$ErrorActionPreference = 'Stop'

if (-not $Dest) {
  $Dest = Join-Path (Split-Path $PSScriptRoot -Parent) 'contents'
}

$SyncDirs  = @('.assets', 'pages', 'public', 'tools', 'ui')
$SyncFiles = @('index.html', 'style.css', 'llms.txt', 'robots.txt', 'sitemap.xml')

if (-not (Test-Path $Dest)) {
  New-Item -ItemType Directory -Path $Dest | Out-Null
}

$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("tbxm-src-" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
if (Test-Path $Tmp) { Remove-Item $Tmp -Recurse -Force }

Write-Debug "Cloning $SourceRepo ($Branch) -> $Tmp"
$null = git clone --depth 1 --branch $Branch $SourceRepo $Tmp 2>&1
if ($LASTEXITCODE -ne 0) { throw "git clone a échoué (code $LASTEXITCODE)" }

try {
  if (-not (Test-Path (Join-Path $Tmp 'index.html'))) {
    throw "Le clone semble invalide : index.html est absent."
  }
  $sha = (git -C $Tmp rev-parse HEAD).Trim()
  Write-Debug "Source commit : $sha"

  foreach ($d in $SyncDirs) {
    $src = Join-Path $Tmp $d
    if (-not (Test-Path $src)) { Write-Debug "Dossier absent de la source, ignoré : $d"; continue }
    $dst = Join-Path $Dest $d
    if (-not (Test-Path $dst)) { New-Item -ItemType Directory -Path $dst | Out-Null }
    $null = robocopy $src $dst /MIR /NFL /NDL /NJH /NJS /NP /XF .gitkeep
    if ($LASTEXITCODE -ge 8) { throw "robocopy a échoué sur '$d' (code $LASTEXITCODE)" }
  }

  foreach ($f in $SyncFiles) {
    $src = Join-Path $Tmp $f
    if (Test-Path $src) { Copy-Item $src (Join-Path $Dest $f) -Force }
  }

  $info = [ordered]@{
    source      = 'RackRunner1/ToolsBoXMachine'
    sourceRepo  = $SourceRepo
    sourceSha   = $sha
    branch      = $Branch
    syncedAtUtc = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
  } | ConvertTo-Json
  [System.IO.File]::WriteAllText((Join-Path $Dest '.sync-info.json'), $info)

  $fileCount = (Get-ChildItem $Dest -Recurse -File).Count
  Write-Debug "OK: contents/ synchronisé ($fileCount fichiers, source @ $sha)"
}
finally {
  if ($KeepClone) { Write-Debug "Clone conservé : $Tmp" }
  else { Remove-Item $Tmp -Recurse -Force -ErrorAction SilentlyContinue }
}