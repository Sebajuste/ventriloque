# Ventriloque

Un atelier qui clone des voix de PNJ, un player qui les joue en séance.

Vous préparez les voix de vos personnages avant la partie ; à la table, vous tapez une
réplique et le PNJ la dit, avec sa voix.

## Ce que ça fait

- **Cloner une voix** depuis quelques secondes de référence, et l'entendre tout de suite.
- **Tenir des fiches** : un personnage, sa voix, ses répliques, son débit de parole.
- **Jouer en séance** : une file de répliques, un transport, et « redis-le » qui rend
  exactement le même son sans refaire le calcul.
- **Fabriquer des paquets** depuis une copie d'un jeu que vous possédez — Cyberpunk 2077
  et StarCraft II aujourd'hui. Les voix extraites restent sur votre machine.

Tout tourne **hors ligne**, sur votre processeur. Rien n'est envoyé nulle part.

## Installer

Windows 10 ou 11, 64 bits. L'installateur est sur la [page des
publications](https://github.com/Sebajuste/ventriloque/releases/latest) ; il s'installe
pour l'utilisateur courant, sans droits administrateur.

Au premier lancement, Ventriloque n'a pas de modèles et le dit. Ils s'installent en
paquet, depuis l'onglet **Packs** — comptez un demi-gigaoctet.

Les mises à jour se cherchent depuis l'onglet **Réglages**, sur demande. Voir
[docs/publication.md](docs/publication.md).

## Ce qu'il faut savoir

**Les voix de jeu ne sont pas livrées.** Un paquet-recette dit *quel jeu chercher et quoi
en extraire* ; l'extraction tourne chez vous, sur votre copie. Rien qui vienne d'un jeu ne
transite par ce dépôt.

**Le moteur vit dans un processus à côté.** Il met ~2,5 s à charger ses modèles au
lancement, et se relance quand on applique un réglage. Il fabrique environ trois fois plus
vite qu'on n'écoute : le son part ~150 ms après la demande.

## Développer

```
npm install && npm --prefix ui install
npm run dev            # tauri dev
npm test               # tests d'interface + tests Rust
npm run build          # tsc, vitest, vite, puis cargo, sans installateur
```

`tools/prepare.ps1` enchaîne ce qui doit être vrai avant que cargo compile, dans un ordre
qui compte — lisez son en-tête avant d'y toucher. L'intégration continue rejoue le même
ordre : voir [docs/publication.md](docs/publication.md).

Le travail se fait sur `develop` ou sur une branche qui en part. `main` ne reçoit que des
fusions, et chacune peut publier une release.

Le fabricant de paquets a besoin de deux dépôts tiers, clonés sur place :

```
git clone --depth 1 https://github.com/ladislav-zezula/CascLib.git tools/pack-builder/vendor/CascLib
git clone --depth 1 https://github.com/hcs64/ww2ogg.git tools/pack-builder/vendor/ww2ogg
```

## Licence

MIT — voir [LICENSE](LICENSE).

Les modèles de synthèse, les bibliothèques tierces et tout contenu extrait d'un jeu ont
leurs propres conditions, qui ne sont pas celles-ci.
