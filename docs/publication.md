# Publier et mettre à jour

Ventriloque s'installe chez des gens qui jouent avec, pas qui suivent le dépôt. Sans canal
de mise à jour, la seule version qui compte est celle du jour de l'installation — et un
correctif n'atteint personne.

Ce document décrit le chemin complet : ce qui déclenche une publication, ce que la CI
vérifie avant, et ce qu'il faut avoir renseigné une fois pour toutes.

## Ce qui déclenche une publication

**Le numéro de version de [`tauri.conf.json`](../src-tauri/tauri.conf.json), pas le
commit.** [`release.yml`](../.github/workflows/release.yml) se déclenche à chaque poussée
sur `master`, mais s'arrête aussitôt si le tag `v<version>` existe déjà.

Publier revient donc à un seul geste :

```
# bumper "version" dans src-tauri/tauri.conf.json, puis
git commit -am "v0.2.0" && git push
```

Publier à chaque commit aurait deux coûts : une compilation Rust complète pour une
correction de typo, et une notification de mise à jour à tout le monde pour la même.

## L'intégration, avant

[`ci.yml`](../.github/workflows/ci.yml) tourne sur **toutes les branches sauf `master`**,
et sur les pull requests qui la visent. Sur `windows-latest`, parce que Ventriloque lit le
registre, interroge Battle.net et pilote un moteur `.exe` : il n'y a rien à vérifier
ailleurs.

Elle rejoue ce que [`tools/prepare.ps1`](../tools/prepare.ps1) fait localement, dans le
même ordre, plus trois contrôles que la compilation seule ne ferait pas :

| Étape | Ce qu'elle attrape |
|---|---|
| `cargo fmt --check` | un format qui dérive du reste du dépôt |
| `cargo clippy --all-targets -D warnings` | du code mort, y compris dans les tests |
| `cargo test` (src-tauri) | 99 tests, et l'écriture de `bindings.ts` |
| `git diff --exit-code ui/src/ipc/bindings.ts` | **le contrat qui a bougé sans être relu** |
| `npm --prefix ui run build` | types, 123 tests d'interface, compilation Vite |
| `cargo check` + `cargo test` (tools) | une rupture du fabricant |

Le quatrième est le seul qui ne soit pas une évidence. `ui/src/ipc/bindings.ts` est écrit
par `cargo test bindings` depuis les commandes Rust, **et il est versionné**. S'il diffère
après que les tests l'ont réécrit, c'est qu'une commande a changé sans que le lien vers
l'interface soit régénéré et relu : l'interface d'ici compilerait contre la bonne version,
et celle du dépôt casserait en séance. Voir l'en-tête de
[`ipc/mod.rs`](../src-tauri/src/ipc/mod.rs), qui pose déjà la règle.

### CascLib et ww2ogg

Le fabricant compile deux dépôts tiers sur place, et ils ne sont pas versionnés ici — la
seule `listfile` de CascLib pèse 91 Mo. Son `build.rs` s'arrête avec la marche à suivre
s'ils manquent ; les deux workflows la suivent avant tout appel à cargo :

```
git clone --depth 1 https://github.com/ladislav-zezula/CascLib.git tools/pack-builder/vendor/CascLib
git clone --depth 1 https://github.com/hcs64/ww2ogg.git tools/pack-builder/vendor/ww2ogg
```

### Le fabricant, toujours avant

`embedded.rs` avale `pack-builder.exe` par `include_bytes!`. Compilé après l'application,
l'exécutable emporterait celui d'avant **sans rien signaler** — c'est la panne la plus
coûteuse du projet, parce qu'elle ne se voit qu'à l'exécution et ressemble à un bug du
code qu'on vient d'écrire. `release.yml` le compile et le copie explicitement avant
d'appeler `tauri build`, qui le refera via `prepare.ps1`. Le refaire coûte quelques
secondes ; l'oublier coûte une release.

## Une version de Rust, épinglée

[`rust-toolchain.toml`](../rust-toolchain.toml) fixe le compilateur pour le poste de
développement, la CI et la release. Deux raisons, dans cet ordre : le binaire publié
devient reproductible, et `stable` cesse de désigner une version différente selon le jour
et la machine — ce qui fait rougir clippy sur du code que personne n'a touché.

C'est la version sur laquelle `fmt --check` et `clippy -D warnings` ont été mis au vert.
La bumper est un geste explicite : changer le `channel`, relancer les deux, et les
nouvelles règles arrivent d'un coup plutôt qu'un matin au hasard.

## Le canal de mise à jour

L'application interroge, quand on le lui demande :

```
https://github.com/Sebajuste/ventriloque/releases/latest/download/latest.json
```

GitHub redirige `latest` vers la release la plus récente ; l'URL n'a donc pas à connaître
les numéros de version. **Cela suppose un dépôt public** : les assets d'un dépôt privé ne
sont pas téléchargeables sans jeton, et l'application n'en a pas.

### Rien ne se cherche au démarrage

La recherche coûte un aller-retour réseau, et hors ligne elle met plusieurs secondes à
échouer. Le lancement n'a pas à porter ça, et une séance sans internet ne doit rien voir
clignoter : c'est un bouton, dans l'onglet Réglages. Son erreur s'affiche, puisqu'on la
lui a demandée.

### Le moteur est tué avant l'installateur

C'est le seul point délicat du branchement, et il est particulier à ce projet. Tauri
termine le processus **sans dérouler la pile** : le `Drop` qui tue le moteur ne part pas,
et il survivrait à la sortie avec un demi-gigaoctet de modèles en mémoire et son port
pris. La relance retomberait alors sur un port occupé par un orphelin que plus personne ne
pilote. `Updates::install` appelle donc `AppState::shutdown` avant de télécharger quoi que
ce soit — voir [`update.rs`](../src-tauri/src/update.rs).

Conséquence assumée, dite dans la fenêtre avant le clic : une réplique en cours s'arrête
net. Les voix, les fiches et les paquets, eux, vivent dans le dossier de données et ne
sont pas touchés.

### La fenêtre sonde, on n'émet rien

Même règle que les paquets : la commande ne rend la main qu'à la fin, et
`update_progress` se lit toutes les 200 ms pendant ce temps. Un événement par morceau reçu
saturerait le pont pour dessiner cent positions de barre.

## La signature de mise à jour

Un paquet téléchargé est vérifié contre la clé publique inscrite dans `tauri.conf.json`.
Sans elle, n'importe qui pouvant servir un `latest.json` à la place de GitHub servirait
aussi un exécutable qui s'installerait tout seul.

À ne pas confondre avec une signature Authenticode : celle-ci authentifie la **mise à
jour** auprès des installations existantes, l'autre authentifierait l'**éditeur** auprès
de Windows. La seconde manque ; elles ne se remplacent pas.

### Générer la paire, une seule fois

```powershell
npx tauri signer generate -w "$env:USERPROFILE\.tauri\ventriloque.key"
```

Par `npx` et non `npm run` : `npm run -- -w` ne transmet pas `-w` au script, npm
l'interceptant comme son propre `--workspace`.

La clé privée est **la** chose à ne pas perdre : la remplacer casserait la mise à jour de
toutes les installations existantes, qui refuseraient un paquet signé par une inconnue.
Elles devraient être réinstallées à la main.

Trois endroits à renseigner :

| Où | Quoi |
|---|---|
| `tauri.conf.json` → `plugins.updater.pubkey` | contenu de `ventriloque.key.pub` |
| Secret GitHub `TAURI_SIGNING_PRIVATE_KEY` | contenu de `ventriloque.key` |
| Secret GitHub `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | le mot de passe saisi |

Tant que `pubkey` est vide, l'application démarre et cherche normalement : la clé ne sert
qu'au moment de vérifier le paquet téléchargé. L'échec n'arriverait donc qu'à
l'installation, au pire moment. À renseigner **avant** la première release.

### Le mot de passe vide, et un message trompeur

La clé livrée avec ce dépôt n'a pas de mot de passe. Le secret
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` doit donc porter une **chaîne vide** — et non ne pas
exister : le workflow résout un secret absent en chaîne vide et pose la variable malgré
tout, si bien que supprimer le secret ne change rien. Pour forcer le vide :

```
printf '' | gh secret set TAURI_SIGNING_PRIVATE_KEY_PASSWORD --repo Sebajuste/ventriloque
```

La saisie interactive de `gh secret set` lit l'entrée standard telle quelle : y répondre
par une touche Entrée seule peut y laisser un caractère, et un mot de passe d'un caractère
suffit à tout casser.

Le message rendu alors par Tauri est **failed to decode secret key: incorrect updater
private key password**. Il désigne le mot de passe, mais recouvre en réalité tout échec de
déchiffrement de la clé — un fichier altéré en transit donne le même. Le départ se fait en
deux essais locaux, qui coûtent une seconde :

```powershell
npx tauri signer sign -f "$env:USERPROFILE\.tauri\ventriloque.key" -p "" fichier
npx tauri signer sign -f "$env:USERPROFILE\.tauri\ventriloque.key" -p "x" fichier
```

Si le premier passe et que le second rend le message du CI, la clé est saine et c'est le
mot de passe qui est en cause.

Enfin, depuis PowerShell, envoyer un fichier à `gh secret set` demande `-Raw` : sans lui,
`Get-Content` découpe en lignes et le pipeline les recompose, ce qui suffit à casser le
checksum minisign.

```powershell
Get-Content -Raw "$env:USERPROFILE\.tauri\ventriloque.key" | gh secret set TAURI_SIGNING_PRIVATE_KEY --repo Sebajuste/ventriloque
```

## Le piège : `createUpdaterArtifacts`

`bundle.createUpdaterArtifacts` vaut **`false`** par défaut. Sans lui, le bundler produit
l'installateur et s'arrête là : ni signatures, ni paquets de mise à jour. Le build
réussit, la release se crée, l'installateur s'y attache — et `latest.json` manque, sans
qu'aucune étape n'ait échoué. Le seul indice tient à une ligne de log : « Signature not
found for the updater JSON. Skipping upload... ».

Le symptôme à distance est muet aussi : les installations existantes interrogent un
`latest.json` absent, reçoivent un 404, et concluent qu'elles sont à jour.

## L'installateur : NSIS, `currentUser`

`bundle.targets` vaut `["nsis"]` et non `"all"`. NSIS est le format que l'updater sait
poser ; un MSI produit à côté ne serait désigné par aucun `latest.json` et personne
n'irait le chercher.

`currentUser` écrit dans `%LOCALAPPDATA%`. Ce n'est pas un détail d'emplacement : un
installateur `perMachine` écrit dans `Program Files`, ce qui demande l'élévation **à
chaque mise à jour**. Une invite UAC par correctif suffirait à faire cliquer « plus tard »
indéfiniment.

## Ce que la mise à jour ne touche pas

Les modèles — un demi-gigaoctet — ne sont pas dans l'installateur : ils s'installent en
paquet, depuis l'onglet Packs, et vivent dans le dossier de données. Une mise à jour ne
les retélécharge pas.

Le moteur et le fabricant, eux, sont embarqués dans l'exécutable et reposés à côté au
lancement suivant — mais seulement si leur taille a changé, voir `write_if_stale` dans
[`embedded.rs`](../src-tauri/src/embedded.rs). Deux versions d'un même binaire qui
pèseraient exactement pareil ne seraient pas redéployées. C'est un contrôle bon marché et
grossier, assumé en release où ces fichiers ne changent qu'avec une version de
Ventriloque.

## Ce qui reste désagréable

Le binaire n'est pas signé Authenticode. Chaque mise à jour rejoue donc l'invite
SmartScreen « Éditeur inconnu ». L'auto-mise à jour rend le certificat **plus** utile, pas
moins : on passe d'une alerte à l'installation à une alerte récurrente.
