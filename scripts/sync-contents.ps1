#Requires -Version 5.1
<#
.SYNOPSIS
    Synchronise le dossier contents/ depuis ToolsBoXMachine.

.DESCRIPTION
    Récupère le contenu du dépôt source (clone léger, ou clone local via
    -SourcePath) puis recopie les dossiers/fichiers connus dans contents/
    avec robocopy /MIR (miroir : les fichiers supprimés en amont sont
    retirés localement).
    Écrit contents/.sync-info.json (SHA source + horodatage).

.PARAMETER SourceRepo
    URL du dépôt source.

.PARAMETER Branch
    Branche du dépôt source.

.PARAMETER Dest
    Dossier de destination (par défaut : <repo>/contents).

.PARAMETER SourcePath
    Dossier local déjà cloné (évite le clone réseau).

.PARAMETER KeepClone
    Conserve le clone temporaire (débogage).

.EXAMPLE
    .\scripts\sync-contents.ps1

.EXAMPLE
    .\scripts\sync-contents.ps1 -SourcePath C:\Users\RackRunner\Documents\ToolsBoXMachine
#>
[CmdletBinding()]
param(
    [string]$SourceRepo  = 'https://github.com/RackRunner1/ToolsBoXMachine.git',
    [string]$Branch     = 'main',
    [string]$Dest       = '',
    [string]$SourcePath = '',
    [switch]$KeepClone
)

$ErrorActionPreference = 'Stop'

$RepoRoot = Split-Path $PSScriptRoot -Parent
if (-not $Dest) { $Dest = Join-Path $RepoRoot 'contents' }

# Dossiers synchronisés en miroir
$SyncDirs  = @('.assets', 'og', 'pages', 'public', 'tools', 'ui')
# Fichiers synchronisés à la racine
$SyncFiles = @('index.html', 'style.css', 'llms.txt', 'robots.txt', 'sitemap.xml')

if (-not (Test-Path $Dest)) {
    New-Item -ItemType Directory -Path $Dest -Force | Out-Null
}

# ---------------------------------------------------------------------------
# 1. Obtention de la source
# ---------------------------------------------------------------------------
$tmp = $null
if ($SourcePath) {
    if (-not (Test-Path (Join-Path $SourcePath 'index.html'))) {
        throw "SourcePath invalide : index.html est absent de '$SourcePath'."
    }
    $src = (Resolve-Path $SourcePath).Path
    Write-Host "Source locale : $src"
}
else {
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ('tbxm-src-' + [Guid]::NewGuid().ToString('N').Substring(0, 8))
    Write-Host "Clonage de $SourceRepo ($Branch) -> $tmp"
    git clone --depth 1 --branch $Branch $SourceRepo $tmp 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "git clone a échoué (code $LASTEXITCODE)" }
    if (-not (Test-Path (Join-Path $tmp 'index.html'))) {
        throw "Le clone semble invalide : index.html est absent."
    }
    $src = $tmp
}

try {
    # -----------------------------------------------------------------------
    # 2. SHA source
    # -----------------------------------------------------------------------
    $sha = 'inconnu'
    if (Test-Path (Join-Path $src '.git')) {
        $out = (git -C $src rev-parse HEAD) 2>$null
        if ($LASTEXITCODE -eq 0 -and $out) { $sha = $out.Trim() }
    }
    Write-Host "Commit source : $sha"

    # -----------------------------------------------------------------------
    # 3. Miroir des dossiers
    # -----------------------------------------------------------------------
    foreach ($d in $SyncDirs) {
        $s = Join-Path $src $d
        if (-not (Test-Path $s)) {
            Write-Host "  (dossier absent de la source, ignoré) $d"
            continue
        }
        $dst = Join-Path $Dest $d
        $null = robocopy $s $dst /MIR /NFL /NDL /NJH /NJS /NP /XF .gitkeep
        if ($LASTEXITCODE -ge 8) { throw "robocopy a échoué sur '$d' (code $LASTEXITCODE)" }
        # robocopy renvoie 1 à 7 en cas de succès (fichiers copiés) : on
        # remet le code à 0, sinon l'étape CI est marquée en échec.
        $global:LASTEXITCODE = 0
    }

    # -----------------------------------------------------------------------
    # 4. Fichiers racine
    # -----------------------------------------------------------------------
    foreach ($f in $SyncFiles) {
        $s = Join-Path $src $f
        if (Test-Path $s) {
            Copy-Item $s (Join-Path $Dest $f) -Force
        }
        else {
            Write-Host "  (fichier absent de la source, ignoré) $f"
        }
    }

    # -----------------------------------------------------------------------
    # 5. Métadonnées de synchro
    # -----------------------------------------------------------------------
    $info = [ordered]@{
        source      = 'RackRunner1/ToolsBoXMachine'
        sourceRepo  = $SourceRepo
        sourcePath  = if ($SourcePath) { (Resolve-Path $SourcePath).Path } else { $null }
        sourceSha   = $sha
        branch      = $Branch
        syncedAtUtc = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
    } | ConvertTo-Json
    [IO.File]::WriteAllText((Join-Path $Dest '.sync-info.json'), $info)

    $fileCount = (Get-ChildItem $Dest -Recurse -File -Force).Count
    Write-Host "OK : contents/ synchronisé ($fileCount fichiers, source @ $sha)"
    # Garde-fou : s'assurer que le script se termine avec un code 0
    # (aucune commande native ne doit laisser un $LASTEXITCODE non nul).
    $global:LASTEXITCODE = 0
}
finally {
    if ($tmp -and -not $KeepClone) {
        Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}
