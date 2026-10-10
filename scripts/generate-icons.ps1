#Requires -Version 5.1
<#
.SYNOPSIS
    Génère les icônes de l'app Tauri (PNG + ICO multi-résolutions).

.DESCRIPTION
    Source par défaut : contents/.assets/logo/logo.png (le logo du contenu
    synchronisé), avec repli sur fav.png. Si aucune source n'existe, une
    icône de secours est dessinée (le build ne casse jamais).

    Sortie : app/src-tauri/icons/
        32x32.png, 128x128.png, 128x128@2x.png (256), icon.png (512)
        icon.ico   (16/24/32/48/64/128/256, PNG compressé, Vista+)

.PARAMETER Source
    Chemin vers un PNG source (carré de préférence).

.PARAMETER OutDir
    Dossier de sortie (par défaut : <repo>/app/src-tauri/icons).

.EXAMPLE
    .\scripts\generate-icons.ps1
#>
[CmdletBinding()]
param(
    [string]$Source  = '',
    [string]$OutDir  = ''
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Drawing

$RepoRoot = Split-Path $PSScriptRoot -Parent
if (-not $OutDir) { $OutDir = Join-Path $RepoRoot 'app\src-tauri\icons' }

if (-not $Source) {
    foreach ($c in @(
        (Join-Path $RepoRoot 'contents\.assets\logo\logo.png'),
        (Join-Path $RepoRoot 'contents\.assets\logo\fav.png')
    )) {
        if (Test-Path $c) { $Source = $c; break }
    }
}

New-Item -ItemType Directory -Path $OutDir -Force | Out-Null

# ---------------------------------------------------------------------------
# Bitmap source (ou icône de secours dessinée)
# ---------------------------------------------------------------------------
$srcBitmap = $null
if ($Source -and (Test-Path $Source)) {
    Write-Host "Source de l'icône : $Source"
    $srcBitmap = New-Object System.Drawing.Bitmap $Source
}
else {
    Write-Host "Aucun logo trouvé dans contents/.assets/logo/ -> icône de secours générée."
    $srcBitmap = New-Object System.Drawing.Bitmap 512, 512
    $g = [System.Drawing.Graphics]::FromImage($srcBitmap)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.Clear([System.Drawing.Color]::Transparent)
    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $r = 96
    $path.AddArc(0, 0, $r, $r, 180, 90)
    $path.AddArc(412, 0, $r, $r, 270, 90)
    $path.AddArc(412, 412, $r, $r, 0, 90)
    $path.AddArc(0, 412, $r, $r, 90, 90)
    $path.CloseFigure()
    $brush = New-Object System.Drawing.Drawing2D.LinearGradientBrush(
        (New-Object System.Drawing.Point 0, 0),
        (New-Object System.Drawing.Point 512, 512),
        [System.Drawing.Color]::FromArgb(255, 37, 99, 235),
        [System.Drawing.Color]::FromArgb(255, 14, 165, 233))
    $g.FillPath($brush, $path)
    $font = New-Object System.Drawing.Font('Segoe UI', 300, [System.Drawing.FontStyle]::Bold, [System.Drawing.GraphicsUnit]::Pixel)
    $sf = New-Object System.Drawing.StringFormat
    $sf.Alignment = [System.Drawing.StringAlignment]::Center
    $sf.LineAlignment = [System.Drawing.StringAlignment]::Center
    $g.DrawString('T', $font, [System.Drawing.Brushes]::White, (New-Object System.Drawing.RectangleF 0, 0, 512, 512), $sf)
    $g.Dispose()
}

try {
    # -----------------------------------------------------------------------
    # Redimensionnement "contain" sur carré transparent
    # -----------------------------------------------------------------------
    function New-SquareIcon {
        param([int]$Size)
        $bmp = New-Object System.Drawing.Bitmap($Size, $Size)
        $g = [System.Drawing.Graphics]::FromImage($bmp)
        $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
        $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality
        $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
        $g.Clear([System.Drawing.Color]::Transparent)
        $ratio = [Math]::Min($Size / $srcBitmap.Width, $Size / $srcBitmap.Height)
        $w = [int]($srcBitmap.Width  * $ratio)
        $h = [int]($srcBitmap.Height * $ratio)
        $x = [int](($Size - $w) / 2)
        $y = [int](($Size - $h) / 2)
        $g.DrawImage($srcBitmap, $x, $y, $w, $h)
        $g.Dispose()
        return $bmp
    }

    function Save-Png {
        param([System.Drawing.Bitmap]$Bitmap, [string]$Path)
        $ms = New-Object System.IO.MemoryStream
        $Bitmap.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        [IO.File]::WriteAllBytes($Path, $ms.ToArray())
        $ms.Dispose()
    }

    # -----------------------------------------------------------------------
    # PNG pour Tauri
    # -----------------------------------------------------------------------
    $pngs = [ordered]@{
        '32x32.png'          = 32
        '128x128.png'        = 128
        '128x128@2x.png'     = 256
        'icon.png'           = 512
    }
    foreach ($name in $pngs.Keys) {
        $size = $pngs[$name]
        $bmp = New-SquareIcon -Size $size
        Save-Png -Bitmap $bmp -Path (Join-Path $OutDir $name)
        $bmp.Dispose()
    }

    # -----------------------------------------------------------------------
    # ICO multi-résolutions (entrées PNG, supportées depuis Vista)
    # -----------------------------------------------------------------------
    $icoSizes = @(16, 24, 32, 48, 64, 128, 256)
    $entries = @()
    foreach ($size in $icoSizes) {
        $bmp = New-SquareIcon -Size $size
        $ms = New-Object System.IO.MemoryStream
        $bmp.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
        $entries += ,@($size, $ms.ToArray())
        $ms.Dispose()
        $bmp.Dispose()
    }

    $icoPath = Join-Path $OutDir 'icon.ico'
    $ms = New-Object System.IO.MemoryStream
    $bw = New-Object System.IO.BinaryWriter($ms)
    $bw.Write([UInt16]0)                  # réservé
    $bw.Write([UInt16]1)                  # 1 = icône
    $bw.Write([UInt16]$entries.Count)     # nombre d'images

    $offset = 6 + (16 * $entries.Count)
    foreach ($e in $entries) {
        $size = $e[0]; $data = $e[1]
        $dim = if ($size -ge 256) { 0 } else { $size }
        $bw.Write([Byte]$dim)             # largeur (0 = 256)
        $bw.Write([Byte]$dim)             # hauteur
        $bw.Write([Byte]0)                # nombre de couleurs
        $bw.Write([Byte]0)                # réservé
        $bw.Write([UInt16]1)              # plans de couleur
        $bw.Write([UInt16]32)             # bits par pixel
        $bw.Write([UInt32]$data.Length)   # taille des données
        $bw.Write([UInt32]$offset)        # offset des données
        $offset += $data.Length
    }
    foreach ($e in $entries) { $bw.Write($e[1]) }
    $bw.Flush()
    [IO.File]::WriteAllBytes($icoPath, $ms.ToArray())
    $bw.Close(); $ms.Dispose()

    Write-Host "OK : icônes générées dans $OutDir"
    Get-ChildItem $OutDir -File | ForEach-Object { "  $($_.Name) ($([math]::Round($_.Length/1KB,1)) Ko)" }
}
finally {
    if ($srcBitmap) { $srcBitmap.Dispose() }
}
