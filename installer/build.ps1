# Construire l'installation Windows (bpm-caddy.iss) : Inno Setup 6, installé
# s'il manque sur la machine, puis le fichier rangé à la racine du dépôt
# sous bpm-caddy-setup-windows-x86_64.exe. Pour la publication et pour la
# tâche Windows de la CI.
param(
    [Parameter(Mandatory = $true)][string]$Version,
    [Parameter(Mandatory = $true)][string]$Launcher
)
$ErrorActionPreference = "Stop"
$iscc = Join-Path ${env:ProgramFiles(x86)} "Inno Setup 6\ISCC.exe"
if (-not (Test-Path $iscc)) {
    choco install innosetup -y --no-progress
    if ($LASTEXITCODE -ne 0) { throw "Inno Setup ne s'installe pas" }
}
if (-not (Test-Path $iscc)) { throw "ISCC introuvable : $iscc" }
& $iscc "/DAppVersion=$Version" "/DLauncher=$Launcher" (Join-Path $PSScriptRoot "bpm-caddy.iss")
if ($LASTEXITCODE -ne 0) { throw "ISCC a échoué ($LASTEXITCODE)" }
$root = Split-Path $PSScriptRoot -Parent
Copy-Item (Join-Path $root "dist-installer\bpm-caddy-setup-windows-x86_64.exe") $root
