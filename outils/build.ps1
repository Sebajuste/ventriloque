# Bâtit Ventriloque en release et dépose l'exécutable portable dans dist\.
#
# L'ORDRE N'EST PAS NÉGOCIABLE. `cargo` embarque `ui\dist` dans le binaire au moment de la
# compilation : si l'interface n'a pas été bâtie avant, l'exécutable emporte la version
# précédente sans rien signaler.
#
# Usage :
#   powershell -File outils\build.ps1              tout : interface, tests, binaire
#   powershell -File outils\build.ps1 -SansTests   saute les tests
#   powershell -File outils\build.ps1 -Bac         copie aussi vers D:\tmp\ventriloque-portable

param(
    [switch]$SansTests,
    [switch]$Bac
)

$ErrorActionPreference = "Stop"
$racine = Split-Path -Parent $PSScriptRoot
Set-Location $racine

$exe = Join-Path $racine "src-tauri\target\release\ventriloque.exe"

# Un Ventriloque ouvert tient son propre fichier : cargo échoue alors sur « failed to remove
# file », qui ne dit pas quoi faire. On le dit ici, avant de perdre une minute de compilation.
$ouverts = @(Get-Process -Name "ventriloque" -ErrorAction SilentlyContinue)
if ($ouverts.Count -gt 0) {
    $pids = ($ouverts | ForEach-Object { $_.Id }) -join ", "
    throw "Ventriloque tourne (PID $pids) et verrouille son exécutable. Ferme la fenêtre, puis relance."
}

Write-Output "== interface (typescript + vite) =="
npm --prefix ui run build
if ($LASTEXITCODE -ne 0) { throw "la construction de l'interface a échoué" }

if (-not $SansTests) {
    Write-Output ""
    Write-Output "== tests =="
    cargo test --release --manifest-path src-tauri\Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw "des tests échouent" }
}

Write-Output ""
Write-Output "== binaire =="
cargo build --release --manifest-path src-tauri\Cargo.toml
if ($LASTEXITCODE -ne 0) { throw "la compilation a échoué" }

# Le binaire est portable : il porte son moteur et son interface, et pose le premier à côté de
# lui au premier lancement. Tout le reste — modèles, voix, fiches — arrive par des paquets.
$dist = Join-Path $racine "dist"
if (-not (Test-Path $dist)) { New-Item -ItemType Directory $dist | Out-Null }
Copy-Item $exe $dist -Force

if ($Bac) {
    # Pas $bac : PowerShell ne distingue pas la casse, et ce nom EST celui du commutateur.
    $dossierBac = "D:/tmp/ventriloque-portable"
    if (Test-Path $dossierBac) {
        Copy-Item $exe $dossierBac -Force
        Write-Output "copie aussi dans $dossierBac"
    }
}

$poids = [math]::Round((Get-Item $exe).Length / 1MB, 1)
Write-Output ""
Write-Output "dist\ventriloque.exe — $poids Mo, portable"
