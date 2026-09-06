# Brief : sortir les voix de StarCraft II, et des jeux Blizzard en général

Une commande de travail pour un agent qui travaille seul. Tout ce dont il a besoin est ici ; il
n'a rien de la conversation d'où cela vient.

---

## Ce qu'il faut produire

Un outil, lancé hors du jeu, qui tire des **répliques d'un personnage** de la copie installée du
joueur et les dépose dans un dossier que Ventriloque sait consommer.

```
outils\extraire-casc\        l'extracteur
    README.md                ce qu'il fait, comment on le lance, ce qui a été mesuré
```

La sortie n'a pas besoin d'être fine : un dossier de fichiers audio par personnage suffit.
Ventriloque fait le reste — assemblage, repli mono, décalage éventuel, normalisation.

```
<sortie>/kerrigan/  0001.ogg  0002.ogg  ...
<sortie>/raynor/    0001.ogg  ...
```

---

## Ce qui est déjà mesuré — ne pas le rechercher

Relevé le 2026-09-06 sur la machine du projet.

### L'installation

| | |
|---|---|
| Racine | `D:\Jeux\StarCraft II` |
| Version | `5.0.16.97563`, branche `eu`, produit `sc2` |
| Stockage | `SC2Data\` — **25 Go** |
| Sous-dossiers | `config\` `data\` `ecache\` `indices\` `s2\` |
| Index | 276 fichiers `.idx` dans `data\`, 154 `.index` dans `indices\` |
| Build Key | `5bc8dcc1fade3a320c12936586b7c0ed` (dans `.build.info`) |
| Doublage | **frFR présent** — `.build.info` porte le tag `frFR speech` |

**Aucun fichier audio en clair.** Vérifié : `find` sur `*.ogg`, `*.wav`, `*.SC2Assets`,
`*.SC2Mod` dans toute l'arborescence ne rend rien. `Versions\` ne contient que deux exécutables,
`Interfaces\` que des `.SC2Interface`. Tout passe par CASC.

**Aucun outil disponible.** Ni `CascLib.dll`, ni `CascView.exe` sur la machine. Aucun paquet
Python : `casc` et `casc-tools` n'existent pas sur PyPI.

### L'obstacle, et c'est le seul qui compte

**Le format racine de StarCraft II est MNDX** — un trie compressé, propre aux jeux Blizzard de
cette génération. Ce n'est pas la table plate qu'on trouve chez d'autres titres : dans CascLib,
son implémentation occupe un fichier entier et une bonne part de la complexité du projet.

Une chaîne CASC complète demande, dans l'ordre :

1. `.build.info` → la Build Key
2. `SC2Data\config\5b\c8\<build key>` → la configuration de build, qui nomme `encoding`, `root`,
   `install`
3. le fichier `encoding` → correspondance CKey (contenu) vers EKey (chiffré/encodé)
4. les `.idx` locaux → EKey vers (archive, décalage, taille)
5. **BLTE** → décompression par trames (`N` brut, `Z` zlib, `F` récursif, `E` chiffré)
6. le fichier `root` → **MNDX**, nom de fichier vers CKey

Les étapes 1 à 5 sont abordables. L'étape 6 est celle qui décide du budget.

---

## À trancher en premier, et s'arrêter si ça échoue

**Est-ce que CascLib se compile avec `cl.exe` ?**

> **Répondu le 2026-09-06 : oui.** CascLib porte `sources-c.c` et `sources-cpp.cpp`, deux
> fichiers de compilation unifiée ; trois `cl.exe` et un `link` suffisent. Elle porte aussi un
> CMakeLists, contrairement à ce qui est écrit plus bas — mais `cmake` n'est pas installé ici et
> n'a pas eu à l'être. MNDX n'a demandé aucun travail : CascLib le traite.

Le projet n'a **aucun CMake**, pas même dans Visual Studio — c'est mesuré, et c'est pourquoi
`src-tauri\build.rs` et le `build.ps1` du mod `ai_npc-holo` pilotent `cl.exe` directement. Si
CascLib peut être bâtie de la même façon, la voie est ouverte et le reste est de la plomberie.
Si elle réclame CMake, dis-le et arrête-toi : le repli est un autre design.

Répondre avec la preuve : la commande qui a bâti, ou l'erreur qui a arrêté.

**Le repli, s'il faut l'employer :** `CascView`, l'outil graphique du même auteur que CascLib,
exporte une branche entière à la main. Le `StarCraft II Editor`, déjà installé à la racine du
jeu, a aussi un navigateur d'archives qui sait exporter. Une extraction manuelle est un résultat
acceptable — mais alors, écris dans le README ce que tu as fait, pour que ce ne soit pas à
refaire de tête la prochaine fois.

---

## Où sont les voix, une fois l'archive ouverte

**Corrigé le 2026-09-06, une fois l'archive ouverte.** Ce n'est pas `Assets/Sounds/VO/`. La
disposition réelle est :

```
<campagne ou mod>\<locale>.sc2assets\localizeddata\sounds\vo\<scène>_<locuteur>_<numéro>.ogg
```

Le doublage est bien une branche par langue, distincte des sous-titres : `frfr.sc2assets` porte
la voix, `frfr.sc2data` le texte. Le locuteur est le jeton qui précède le numéro. Le détail est
dans `outils\extraire-casc\README.md`, qui porte aussi la jointure sous-titre ↔ prise.

---

## Ce qu'une bonne référence de voix demande

Ventriloque clone à partir d'un seul fichier de référence qu'il assemble lui-même. L'extracteur
n'a donc qu'à fournir de la matière propre.

- **20 à 40 secondes** par personnage au total. Les références qui fonctionnent aujourd'hui en
  font 30 à 35. Au-delà, le temps de clonage augmente sans que la voix gagne.
- **Un seul locuteur**, sans musique ni bruit de fond, sans effet radio si possible.
- **Dix à vingt répliques courtes** valent mieux qu'un long monologue : la référence porte une
  manière de parler, et une seule phrase n'en montre qu'une facette.
- Le format n'a pas d'importance : `wav`, `mp3`, `flac`, `ogg`, `m4a` sont tous lus. La fréquence
  et le nombre de canaux non plus — le moteur ramène tout en mono 24 kHz avec un Lanczos à seize
  lobes, puis normalise.

---

## Le format d'un paquet Ventriloque

Si l'extracteur va jusqu'au paquet, voici le contrat exact. Il est appliqué par
`src-tauri\src\packs.rs`, qui a des tests.

```
mon-paquet.zip
    pack.json            OBLIGATOIRE, à la racine du zip
    voix/*.wav           les références
    pnj/*.json           les fiches de personnages
    modeles/             réservé au paquet des modèles ; pas d'usage ici
```

`pack.json` :

```json
{
  "nom": "StarCraft II",
  "version": "1.0",
  "auteur": "voix extraites d'une copie du jeu",
  "description": "Les voix de Koprulu."
}
```

Seul `nom` est obligatoire. Les champs `fichiers` et `installe_le` sont ajoutés par
l'installateur ; ne pas les écrire.

Une fiche, `pnj/<id>.json` :

```json
{
  "id": "sarah_kerrigan",
  "nom": "Sarah Kerrigan",
  "univers": "StarCraft II",
  "voix": "kerrigan.wav",
  "repliques": [
    "Je ne suis plus celle que tu as connue.",
    "Tu perds ton temps.",
    "Parle."
  ]
}
```

**`id` doit être exactement le nom du fichier, sans l'extension.** À la lecture, c'est le nom du
fichier qui fait foi : une fiche dont l'`id` diverge changera d'identité au premier
redémarrage. La règle est celle de `fiches::identifiant` — minuscules, tout ce qui n'est pas
alphanumérique devient un souligné, les soulignés de tête et de queue sont retirés.

`voix` nomme un fichier de `voix/`. S'il ne finit pas par `.wav`, le moteur le comprend comme une
**voix de catalogue** et le cherche dans les modèles, pas dans les références.

### Ce que l'installateur refuse

- un zip sans `pack.json` à sa racine — c'est ce qui empêche un zip quelconque de se répandre
  dans les dossiers de l'application ;
- toute entrée hors de `voix/`, `pnj/`, `modeles/` — ignorée et comptée ;
- tout chemin qui remonte (`..`), absolu, ou porteur d'une lettre de lecteur.

---

## Un piège à connaître : les collisions entre paquets

Le moteur met en cache l'état de clonage de chaque voix sous
`voix\.cache\<radical>.emb` et `.kv`. **Le radical seul** : le dossier est retiré, et la validité
se juge sur la date du `.wav`.

Conséquence : deux paquets qui livrent chacun un `narrator.wav` visent le même cache, et le
second peut hériter de la voix du premier **sans le moindre avertissement**. Sous-dossiers
compris — `sc2/narrator.wav` et `cp77/narrator.wav` donnent le même `narrator.kv`.

Tant que ce n'est pas corrigé côté application, **donne des noms uniques entre univers** :
`sc2_kerrigan.wav` plutôt que `kerrigan.wav`.

---

## Ce qui n'est pas demandé

**Rien ne se distribue.** L'outil lit la copie du joueur, sur sa machine, et produit des paquets
qui restent chez lui. Ce sont des enregistrements d'acteurs tirés d'un jeu commercial : légitimes
pour un usage personnel, pas pour un partage. Ne rien publier, ne rien téléverser, ne pas ajouter
de fonction de partage.

**Aucun réseau.** Tout se lit sur le disque local. Le CDN nommé dans `.build.info` n'a pas à être
contacté.

**Ne pas toucher au jeu.** Lecture seule sur `D:\Jeux\StarCraft II`. Aucun fichier n'y est écrit,
déplacé ni renommé.

---

## Comment on saura que c'est fini

1. Une commande produit un dossier de sons pour au moins deux personnages nommés.
2. Ventriloque, onglet Atelier, accepte ces fichiers et fabrique une voix.
3. Cette voix parle dans le player, et **on reconnaît le personnage à l'oreille**.

Le troisième critère est le seul qui compte, et il ne se mesure pas : il s'écoute.
