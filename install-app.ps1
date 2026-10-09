#Requires -Version 5.1
<#
.SYNOPSIS
  Installe ToolsBoX (RackRunner/ToolsBoXApp) sur ce PC :
    1. installe une fois le certificat de signature dans "Trusted People"
    2. télécharge le fichier .appinstaller depuis la dernière release GitHub
    3. ouvre l'installeur d'applications Windows

.EXAMPLE  (une seule commande, dans PowerShell)
  irm https://github.com/RackRunner1/ToolsBoXApp/releases/latest/download/install-app.ps1 | iex

.NOTES
  - Le protocole ms-appinstaller: est désactivé par défaut depuis décembre 2023 :
    on télécharge donc le fichier .appinstaller et on l'ouvre directement.
  - L'élévation administrateur est nécessaire pour installer le certificat
    au niveau de la machine (Trusted People).
#>
[CmdletBinding()]
param(
  [string]$BaseUrl = "https://github.com/RackRunner1/ToolsBoXApp/releases/latest/download"
)

$ErrorActionPreference = "Stop"

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()) `
             .IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

# ------------------------------------------------------------------
# Sans les droits admin, on relance tout le processus en élévation.
# (Le script peut être lancé via irm ... | iex, donc sans chemin de fichier :
#  on refait simplement le travail dans la fenêtre élevée.)
# ------------------------------------------------------------------
if (-not $isAdmin) {
    Write-Host "Élévation administrateur requise pour installer le certificat..."
    $tmp0 = Join-Path $env:TEMP "ToolsBoXApp-install"
    New-Item -ItemType Directory -Path $tmp0 -Force | Out-Null
    $ai0 = Join-Path $tmp0 "ToolsBoXApp.appinstaller"
    $cer0 = Join-Path $tmp0 "ToolsBoXApp.cer"
    $cmd = "Set-Location '$tmp0'; " +
           "Invoke-WebRequest '$BaseUrl/ToolsBoXApp.cer' -OutFile '$cer0' -UseBasicParsing; " +
           "Invoke-WebRequest '$BaseUrl/ToolsBoXApp.appinstaller' -OutFile '$ai0' -UseBasicParsing; " +
           "try { Import-Certificate -FilePath '$cer0' -CertStoreLocation 'Cert:\LocalMachine\TrustedPeople' } catch { }; " +
           "Start-Process '$ai0'"
    Start-Process powershell -Verb RunAs -ArgumentList "-NoProfile","-ExecutionPolicy","Bypass","-Command",$cmd
    return
}

$tmp = Join-Path $env:TEMP "ToolsBoXApp-install"
New-Item -ItemType Directory -Path $tmp -Force | Out-Null
$ai  = Join-Path $tmp "ToolsBoXApp.appinstaller"
$cer = Join-Path $tmp "ToolsBoXApp.cer"

Write-Host "1/3 Téléchargement de ToolsBoXApp.cer..."
Invoke-WebRequest "$BaseUrl/ToolsBoXApp.cer" -OutFile $cer -UseBasicParsing

Write-Host "2/3 Installation du certificat dans 'Trusted People' (Ordinateur local)..."
try {
    Import-Certificate -FilePath $cer -CertStoreLocation "Cert:\LocalMachine\TrustedPeople" | Out-Null
    Write-Host "    Certificat installé."
}
catch {
    Write-Warning "Import du certificat échoué : $_"
    Write-Warning "Vous pouvez aussi le faire à la main : double-cliquez ToolsBoXApp.cer -> Installer le certificat -> Ordinateur local -> Trusted People."
}

Write-Host "3/3 Ouverture de l'installeur d'applications..."
Invoke-WebRequest "$BaseUrl/ToolsBoXApp.appinstaller" -OutFile $ai -UseBasicParsing
Start-Process $ai

Write-Host ""
Write-Host "Si l'installation échoue avec une erreur de confiance :" -ForegroundColor Yellow
Write-Host "  Paramètres Windows -> Mise à jour & sécurité -> Pour les développeurs -> activer 'Mode développeur'" -ForegroundColor Yellow
Write-Host "  ou réinstallez le certificat comme indiqué ci-dessus." -ForegroundColor Yellow
