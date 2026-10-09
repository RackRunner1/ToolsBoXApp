#Requires -Version 5.1
<#
.SYNOPSIS
  Synchronise le dossier contents/ de ToolsBoXApp avec le code du site
  publié dans ToolsBoXMachine (RackRunner1/ToolsBoXMachine).

  Ne copie QUE les fichiers qui constituent le site (pas les méta-fichiers
  du repo : README, LICENSE, .github, og/ simulations d'aperçu, etc.)

.EXAMPLE
  # Utilisation locale
  .\scripts\sync-contents.ps1

.EXAMPLE
  # La CI l'appelle tous les jours à minuit UTC (GitHub Actions), puis commit & push.
#>
[CmdletBinding()]
param(
  [string]$SourceRepo = 'https://github.com/RackRunner1/ToolsBoXMachine.git',
  [string]$Branch     = 'main',
  [string]$Dest       = '',
  [switch]$KeepClone
)

$ErrorActionPreference = 'Stop'

# Par défaut : <racine du repo>\contents
if (-not $Dest) {
  $Dest = Join-Path (Split-Path $PSScriptRoot -Parent) 'contents'
}

# --- Éléments du repo source considérés comme "le site" ---
$SyncDirs  = @('.assets', 'pages', 'public', 'tools', 'ui')
$SyncFiles = @('index.html', 'style.css', 'llms.txt', 'robots.txt', 'sitemap.xml')

if (-not (Test-Path $Dest)) {
  New-Item -ItemType Directory -Path $Dest | Out-Null
}

$Tmp = Join-Path ([System.IO.Path]::GetTempPath()) ("tbxm-src-" + [Guid]::NewGuid().ToString('N').Substring(0, 8))
if (Test-Path $Tmp) { Remove-Item $Tmp -Recurse -Force }

Write-Host "Cloning $SourceRepo ($Branch) -> $Tmp"
git clone --depth 1 --branch $Branch $SourceRepo $Tmp | Write-Host

try {
  if (-not (Test-Path (Join-Path $Tmp 'index.html'))) {
    throw "Le clone semble invalide : index.html est absent."
  }
  $sha = (git -C $Tmp rev-parse HEAD).Trim()
  Write-Host "Source commit : $sha"

  # --- Copie miroir des dossiers ---
  foreach ($d in $SyncDirs) {
    $src = Join-Path $Tmp $d
    if (-not (Test-Path $src)) { Write-Warning "Dossier absent de la source, ignoré : $d"; continue }
    $dst = Join-Path $Dest $d
    if (-not (Test-Path $dst)) { New-Item -ItemType Directory -Path $dst | Out-Null }
    # /MIR = miroir : les fichiers supprimés côté source sont aussi supprimés ici
    $null = robocopy $src $dst /MIR /NFL /NDL /NJH /NJS /NP /XF .gitkeep
    if ($LASTEXITCODE -ge 8) { throw "robocopy a échoué sur '$d' (code $LASTEXITCODE)" }
  }

  # --- Copie des fichiers racine ---
  foreach ($f in $SyncFiles) {
    $src = Join-Path $Tmp $f
    if (Test-Path $src) { Copy-Item $src (Join-Path $Dest $f) -Force }
  }

  # --- Métadonnées de synchro (affichées dans la barre d'état de l'app) ---
  $info = [ordered]@{
    source      = 'RackRunner1/ToolsBoXMachine'
    sourceRepo  = $SourceRepo
    sourceSha   = $sha
    branch      = $Branch
    syncedAtUtc = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
  } | ConvertTo-Json
  [System.IO.File]::WriteAllText((Join-Path $Dest '.sync-info.json'), $info)

  $fileCount = (Get-ChildItem $Dest -Recurse -File).Count
  Write-Host "OK: contents/ synchronisé ($fileCount fichiers, source @ $sha)"
}
finally {
  if ($KeepClone) { Write-Host "Clone conservé : $Tmp" }
  else { Remove-Item $Tmp -Recurse -Force -ErrorAction SilentlyContinue }
}
