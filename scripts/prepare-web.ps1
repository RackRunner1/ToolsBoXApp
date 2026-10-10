#Requires -Version 5.1
<#
.SYNOPSIS
    Prépare les assets web embarqués dans l'app Tauri.

.DESCRIPTION
    Copie contents/ vers app/web/ (le frontendDist configuré dans
    app/src-tauri/tauri.conf.json). Tous les fichiers trouvés sont embarqués
    dans le binaire au build : l'exe produit est autonome.

    À exécuter après scripts/sync-contents.ps1 et avant `tauri build`.

.PARAMETER ContentsDir
    Dossier source (par défaut : <repo>/contents).

.PARAMETER WebDir
    Dossier destination (par défaut : <repo>/app/web).

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
    throw "index.html absent de '$ContentsDir' - lance d'abord : .\scripts\sync-contents.ps1"
}

# ---------------------------------------------------------------------------
# Reconstruction complète de app/web/
# ---------------------------------------------------------------------------
if (Test-Path $WebDir) { Remove-Item $WebDir -Recurse -Force }
New-Item -ItemType Directory -Path $WebDir -Force | Out-Null

$null = robocopy $ContentsDir $WebDir /E /NFL /NDL /NJH /NJS /NP
if ($LASTEXITCODE -ge 8) { throw "robocopy a échoué (code $LASTEXITCODE)" }
# robocopy renvoie 1 à 7 en cas de succès (fichiers copiés) : on remet le
# code à 0, sinon l'étape CI est marquée en échec.
$global:LASTEXITCODE = 0

# ---------------------------------------------------------------------------
# Vérifications minimales
# ---------------------------------------------------------------------------
foreach ($f in @('index.html', 'style.css')) {
    if (-not (Test-Path (Join-Path $WebDir $f))) { throw "$f est absent de app/web/ - contenu incomplet ?" }
}

$files = Get-ChildItem $WebDir -Recurse -File -Force
$sizeMb = [math]::Round((($files | Measure-Object Length -Sum).Sum / 1MB), 2)
Write-Host "OK : app/web/ prêt ($($files.Count) fichiers, $sizeMb Mo) - embed dans le binaire au build"
# Garde-fou : s'assurer que le script se termine avec un code 0.
$global:LASTEXITCODE = 0
