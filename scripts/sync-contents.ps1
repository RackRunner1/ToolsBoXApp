#Requires -Version 5.1
<#
.SYNOPSIS
    Syncs the contents/ folder from ToolsBoXMachine.

.DESCRIPTION
    Fetches the source repository content (shallow clone, or a local clone
    through -SourcePath) then mirrors the known folders/files into contents/
    with robocopy /MIR (files deleted upstream are removed locally).
    Writes contents/.sync-info.json (source SHA + timestamp).

.PARAMETER SourceRepo
    URL of the source repository.

.PARAMETER Branch
    Branch of the source repository.

.PARAMETER Dest
    Destination folder (default: <repo>/contents).

.PARAMETER SourcePath
    Already-cloned local folder (skips the network clone).

.PARAMETER KeepClone
    Keeps the temporary clone (debugging).

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

# Folders synced as mirrors
$SyncDirs  = @('.assets', 'og', 'pages', 'public', 'tools', 'ui')
# Files synced at the root
$SyncFiles = @('index.html', 'style.css', 'llms.txt', 'robots.txt', 'sitemap.xml')

if (-not (Test-Path $Dest)) {
    New-Item -ItemType Directory -Path $Dest -Force | Out-Null
}

# ---------------------------------------------------------------------------
# 1. Get the source
# ---------------------------------------------------------------------------
$tmp = $null
if ($SourcePath) {
    if (-not (Test-Path (Join-Path $SourcePath 'index.html'))) {
        throw "Invalid SourcePath: index.html is missing from '$SourcePath'."
    }
    $src = (Resolve-Path $SourcePath).Path
    Write-Host "Local source: $src"
}
else {
    $tmp = Join-Path ([IO.Path]::GetTempPath()) ('tbxm-src-' + [Guid]::NewGuid().ToString('N').Substring(0, 8))
    Write-Host "Cloning $SourceRepo ($Branch) -> $tmp"
    git clone --depth 1 --branch $Branch $SourceRepo $tmp 2>&1 | Out-Null
    if ($LASTEXITCODE -ne 0) { throw "git clone failed (code $LASTEXITCODE)" }
    if (-not (Test-Path (Join-Path $tmp 'index.html'))) {
        throw "The clone looks invalid: index.html is missing."
    }
    $src = $tmp
}

try {
    # -----------------------------------------------------------------------
    # 2. Source SHA
    # -----------------------------------------------------------------------
    $sha = 'unknown'
    if (Test-Path (Join-Path $src '.git')) {
        $out = (git -C $src rev-parse HEAD) 2>$null
        if ($LASTEXITCODE -eq 0 -and $out) { $sha = $out.Trim() }
    }
    Write-Host "Source commit: $sha"

    # -----------------------------------------------------------------------
    # 3. Mirror the folders
    # -----------------------------------------------------------------------
    foreach ($d in $SyncDirs) {
        $s = Join-Path $src $d
        if (-not (Test-Path $s)) {
            Write-Host "  (folder missing from source, skipped) $d"
            continue
        }
        $dst = Join-Path $Dest $d
        $null = robocopy $s $dst /MIR /NFL /NDL /NJH /NJS /NP /XF .gitkeep
        if ($LASTEXITCODE -ge 8) { throw "robocopy failed on '$d' (code $LASTEXITCODE)" }
        # robocopy returns 1-7 on success (files copied): reset the code to
        # 0 or the CI step is reported as failed.
        $global:LASTEXITCODE = 0
    }

    # -----------------------------------------------------------------------
    # 4. Root files
    # -----------------------------------------------------------------------
    foreach ($f in $SyncFiles) {
        $s = Join-Path $src $f
        if (Test-Path $s) {
            Copy-Item $s (Join-Path $Dest $f) -Force
        }
        else {
            Write-Host "  (file missing from source, skipped) $f"
        }
    }

    # -----------------------------------------------------------------------
    # 5. Sync metadata (idempotent: only rewritten when the source changed,
    #    to avoid a daily commit with no changes)
    # -----------------------------------------------------------------------
    $infoPath = Join-Path $Dest '.sync-info.json'
    $unchanged = $false
    if (Test-Path $infoPath) {
        try {
            $existing = Get-Content $infoPath -Raw | ConvertFrom-Json
            $unchanged = ($existing.sourceSha -eq $sha) -and
                         ($existing.branch -eq $Branch) -and
                         ($existing.sourceRepo -eq $SourceRepo)
        }
        catch { $unchanged = $false }
    }

    if ($unchanged) {
        Write-Host ".sync-info.json unchanged (same source commit)."
    }
    else {
        $info = [ordered]@{
            source      = 'RackRunner1/ToolsBoXMachine'
            sourceRepo  = $SourceRepo
            sourcePath  = if ($SourcePath) { (Resolve-Path $SourcePath).Path } else { $null }
            sourceSha   = $sha
            branch      = $Branch
            syncedAtUtc = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ')
        } | ConvertTo-Json
        [IO.File]::WriteAllText($infoPath, $info)
    }

    $fileCount = (Get-ChildItem $Dest -Recurse -File -Force).Count
    Write-Host "OK: contents/ synced ($fileCount files, source @ $sha)"
    # Safety net: make sure the script exits with code 0 (no native
    # command may leave a non-zero $LASTEXITCODE behind).
    $global:LASTEXITCODE = 0
}
finally {
    if ($tmp -and -not $KeepClone) {
        Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
    }
}
