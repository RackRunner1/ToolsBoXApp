#Requires -Version 5.1
[CmdletBinding()]
param(
  [string]$BaseUrl = "https://github.com/RackRunner1/ToolsBoXApp/releases/latest/download"
)

$ErrorActionPreference = "Stop"

$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)

if (-not $isAdmin) {
    Write-Debug "Élévation administrateur requise pour installer le certificat..."
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

Write-Debug "1/3 Téléchargement de ToolsBoXApp.cer..."
Invoke-WebRequest "$BaseUrl/ToolsBoXApp.cer" -OutFile $cer -UseBasicParsing

Write-Debug "2/3 Installation du certificat dans 'Trusted People' (Ordinateur local)..."
try {
    Import-Certificate -FilePath $cer -CertStoreLocation "Cert:\LocalMachine\TrustedPeople" | Out-Null
    Write-Debug "    Certificat installé."
}
catch {
    Write-Debug "Import du certificat échoué : $_"
    Write-Debug "Vous pouvez aussi le faire à la main : double-cliquez ToolsBoXApp.cer -> Installer le certificat -> Ordinateur local -> Trusted People."
}

Write-Debug "3/3 Ouverture de l'installeur d'applications..."
Invoke-WebRequest "$BaseUrl/ToolsBoXApp.appinstaller" -OutFile $ai -UseBasicParsing
Start-Process $ai

Write-Debug ""
Write-Debug "Si l'installation échoue avec une erreur de confiance :"
Write-Debug "  Paramètres Windows -> Mise à jour & sécurité -> Pour les développeurs -> activer 'Mode développeur'"
Write-Debug "  ou réinstallez le certificat comme indiqué ci-dessus."