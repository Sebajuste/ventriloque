# Ce qui doit être vrai AVANT que `cargo` compile le binaire.
#
# APPELÉ PAR TAURI, pas à la main : c'est le `beforeBuildCommand` de `tauri.conf.json`. Le point
# d'entrée est `npm run build`, qui lance `tauri build --no-bundle`.
#
# L'ORDRE N'EST PAS NÉGOCIABLE, et c'est toute la raison d'être de ce script. Deux choses sont
# figées dans l'exécutable au moment de la compilation :
#
#   src-tauri\binaries\fabriquer.exe   par `include_bytes!`, dans embarque.rs
#   ui\dist                            par tauri-build
#
# Si l'une n'a pas été refaite avant, l'exécutable emporte la version précédente **sans rien
# signaler**. C'est la panne la plus coûteuse du projet : elle ne se voit qu'à l'exécution, et
# elle ressemble à un bug du code qu'on vient d'écrire.
#
# POURQUOI DU POWERSHELL ICI. Ce script ne décide de rien : il enchaîne quatre commandes dans un
# ordre qui compte, et vérifie deux choses que Tauri ne sait pas vérifier. Tout ce qui est un
# vrai choix — quoi bâtir, où le déposer — est dans `package.json` et `tauri.conf.json`.
#
#   $env:VENTRILOQUE_SANS_TESTS = "1"   saute les tests Rust (npm run build:rapide le pose)

$ErrorActionPreference = "Stop"
$racine = Split-Path -Parent $PSScriptRoot
Set-Location $racine

# Un Ventriloque ouvert tient son propre fichier : cargo échoue alors sur « failed to remove
# file », qui ne dit pas quoi faire. On le dit ici, avant de perdre une minute de compilation.
$ouverts = @(Get-Process -Name "ventriloque" -ErrorAction SilentlyContinue)
if ($ouverts.Count -gt 0) {
    $pids = ($ouverts | ForEach-Object { $_.Id }) -join ", "
    throw "Ventriloque tourne (PID $pids) et verrouille son exécutable. Ferme la fenêtre, puis relance."
}

# LE FABRICANT D'ABORD. Il porte CascLib et le moteur de script, et fait tourner les recettes
# des paquets hors du processus de l'application. Voir `recette.rs`.
Write-Output "== fabricant (casclib + rhai) =="
cargo build --release --manifest-path outils\fabriquer\Cargo.toml
if ($LASTEXITCODE -ne 0) { throw "la compilation du fabricant a échoué" }
Copy-Item outils\fabriquer\target\release\fabriquer.exe src-tauri\binaries\fabriquer.exe -Force

# LE LIEN ENSUITE, AVANT DE BÂTIR L'INTERFACE CONTRE LUI.
#
# `ui\src\lien.ts` porte les commandes de Rust et la forme de leurs retours ; il est écrit par
# `cargo test lien`. Le regénérer après avoir bâti l'interface ne servirait à rien : elle aurait
# déjà compilé contre la version d'avant.
Write-Output ""
Write-Output "== lien rust <-> interface =="

# ON REGARDE AVANT ET APRÈS, et c'est toute la finesse de ce contrôle.
#
# Le lien est versionné. Le savoir « modifié » ne dit rien en soi : pendant qu'on travaille sur
# une commande, il l'est en permanence, et bloquer là-dessus interdirait de bâtir pour essayer
# sa propre modification. Ce qui mérite d'être signalé, c'est qu'il bouge *pendant cette
# compilation* alors qu'il était propre : là, le contrat vient de changer, et l'interface qu'on
# s'apprête à bâtir ne compile plus contre ce qui a été relu.
#
# Dans les deux cas on avertit et on continue. Le seul juge est celui qui lit le diff.
$avant = git status --porcelain -- ui/src/lien.ts 2>$null

cargo test --manifest-path src-tauri\Cargo.toml lien
if ($LASTEXITCODE -ne 0) { throw "le lien vers l'interface n'a pas pu être écrit" }

$apres = git status --porcelain -- ui/src/lien.ts 2>$null
if ($apres -and -not $avant) {
    Write-Warning "le contrat vient de bouger : une commande Rust a changé et ui\src\lien.ts a été réécrit. Relis « git diff ui/src/lien.ts » avant de commiter."
} elseif ($apres) {
    Write-Output "  (ui\src\lien.ts est modifié et non commité — travail en cours)"
}

Write-Output ""
Write-Output "== interface (typescript + tests + vite) =="
npm --prefix ui run build
if ($LASTEXITCODE -ne 0) { throw "la construction de l'interface a échoué" }

# Les tests Rust en release : la compilation est partagée avec celle qui suit, donc ils ne
# coûtent presque que leur exécution. Rien ne part en binaire sans être passé par là.
if ($env:VENTRILOQUE_SANS_TESTS -ne "1") {
    Write-Output ""
    Write-Output "== tests rust =="
    cargo test --release --manifest-path src-tauri\Cargo.toml
    if ($LASTEXITCODE -ne 0) { throw "des tests échouent" }
}
