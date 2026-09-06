# fabriquer — faire tourner la recette d'un paquet sur une copie installée d'un jeu

Un binaire, embarqué dans Ventriloque et posé dans `moteur\` au premier lancement. Il porte les
trois choses qu'une recette réclame et que l'application n'a pas à porter elle-même :

| | |
|---|---|
| **CascLib** | le lecteur d'archives Blizzard, compilé par `build.rs` |
| **symphonia** | le décodage Ogg Vorbis, et le montage de la référence |
| **Rhai** | le moteur de script, et son bac à sable |

```powershell
fabriquer --script <recette.rhai> --jeu <racine du jeu> --sortie <dossier de travail>
fabriquer --lister <fragment> --jeu <racine du jeu>      # pour fouiller à la main
```

---

## Pourquoi un processus séparé

Le C++ de CascLib et le script d'un paquet sont deux codes qui ne viennent pas de l'application.
Aucun des deux ne tourne dans Ventriloque : le processus est lancé sous *job object*, exactement
comme le moteur de parole (`engine.rs`), il meurt avec la fenêtre, et il écrit dans un dossier de
travail jetable. Ce qui en sort passe ensuite par le même contrôle qu'une entrée de zip —
`recette.rs` refuse ce qui n'est pas un nom de fichier ordinaire avec la bonne extension.

Le dossier du jeu est ouvert en lecture seule. Rien ne va sur le réseau : le CDN nommé dans
`.build.info` n'est jamais contacté.

## Ce qu'un script peut faire, et rien d'autre

**Le script n'a aucune entrée-sortie.** Il ne reçoit jamais un chemin, il ne peut pas en
fabriquer un, et aucun verbe n'en accepte. Le bac à sable est donc presque vide par
construction : il n'y a rien à retirer, parce qu'il n'y a rien eu à ajouter.

```rhai
produit()                     le nom de code du jeu ouvert : "s2", "fenris"
lister(motif)                 les entrées dont le nom concorde, en minuscules
taille(nom)                   la taille d'une entrée, en octets
texte(nom)                    le contenu texte d'une entrée
dire(ligne)                   une ligne de journal, que la fenêtre affiche
voix(#{ nom, prises, duree, min, max })        monte et écrit une référence
fiche(#{ id, nom, univers, voix, repliques })  écrit une fiche de personnage
```

Ce qu'il ne peut pas faire : ouvrir un fichier, lancer un processus, joindre le réseau, sortir du
dossier de travail, appeler `eval`, ni tourner sans fin — les compteurs de Rhai l'arrêtent.

Rhai plutôt que Lua pour une raison de sens du défaut : Lua arrive avec `io`, `os`, `package`,
`require` et `debug`, et le bac à sable consiste à *retirer*, en n'en oubliant aucun. Rhai est du
Rust pur, sans bibliothèque d'entrée-sortie du tout : le bac à sable consiste à *ajouter*. Une
DLL a été écartée d'emblée — chargée en processus, elle a tous les droits de l'utilisateur.

**Un piège de syntaxe, qui a coûté une heure :** `trim()` modifie sur place et ne rend rien.
`valeur = valeur.trim()` met l'unité dans la variable, et une jointure échoue ensuite sans dire
un mot. Écrire `valeur.trim();`.

---

## Ce qui a été mesuré sur StarCraft II

Relevé le 2026-09-06, version 5.0.16.97563, `D:\Jeux\StarCraft II`.

### CascLib se bâtit sans CMake

Elle porte `sources-c.c` et `sources-cpp.cpp`, deux fichiers de compilation unifiée qui incluent
tout le reste : deux appels à `cc` dans `build.rs`, et c'est fini. Elle est bâtie en **Unicode**,
pour que les chemins d'installation accentués ouvrent comme les autres.

**MNDX, le format racine annoncé comme l'obstacle, n'en a pas été un** : CascLib le traite.
`CascOpenStorage` sur la racine du jeu rend **779 862 entrées nommées**, dont 353 385 `.ogg`.
Aucune clé de déchiffrement n'a manqué.

### Où sont les voix

Pas sous `Assets/Sounds/VO/`. La disposition réelle est :

```
<campagne ou mod>\<locale>.sc2assets\localizeddata\sounds\vo\<scène>_<locuteur>_<numéro>.ogg
```

Le doublage est une branche par langue : `frfr.sc2assets` porte la voix — **28 348 `.ogg`** —,
`frfr.sc2data` le texte. Ce sont bien deux réglages distincts dans le jeu. Le locuteur est le
jeton qui précède le numéro : Kerrigan pèse 1 797 répliques françaises, Artanis 963, Nova 521.

### Le texte de chaque prise se retrouve

C'est la trouvaille qui fait la qualité du paquet. Les tables
`<locale>.sc2data\localizeddata\conversationstrings.txt` portent des clés
`Conversation/<scène>/Line<numéro>` — **la même scène et le même numéro que le nom du `.ogg`** :

```
zbriefing_char03_kerrigan_007.ogg
Conversation/zBriefing_Char03/Line00007=Soldats, l'Essaim de Kerrigan est sur le plateau. […]
```

Aucun format binaire à ouvrir, aucun `.SC2Conv` à décoder : une jointure sur un nom de fichier.
**16 887 sous-titres** français en sortent, ce qui permet de ne retenir que des prises dont on
sait qu'elles sont des phrases, et de remplir les répliques des fiches avec le texte du jeu.
Quatre clés apparaissent avec deux textes différents, deux mods partageant un nom de scène :
elles sont écartées.

### Comment la référence est montée

Chaque `.ogg` est décodé en mémoire — aucun n'est écrit sur le disque —, replié en mono, ébarbé
de ses silences à −42 dBFS avec 30 ms de marge. Les prises dont la durée tombe dans la fenêtre
sont empilées **en alternant les scènes**, jusqu'à la durée visée : une référence tirée d'un seul
briefing ne porte qu'une humeur. Sortie en WAV mono 16 bits à la fréquence des sources
(44 100 Hz), normalisée à −1 dBFS.

La recette vise 32 s en prises de 1 à 4 s, ce qui donne 9 à 15 répliques par personnage. Les
fichiers sont pré-filtrés sur leur taille (20 à 70 ko, le doublage tournant autour de 18 ko la
seconde) pour ne pas décoder des milliers de `.ogg` inutilement. Un personnage qui ne parle
jamais court — Amon n'a pas une réplique sous quatre secondes — bascule sur une fenêtre large
plutôt que d'être écarté.

**Les voix sont préfixées `sc2_`.** Le moteur met en cache l'état de clonage sous le seul radical
du fichier : un `kerrigan.wav` d'un autre univers hériterait de celui-ci sans avertissement.

---

## Les autres jeux

Le lecteur est ce qui décide, pas la recette.

| jeu | conteneur | état |
|---|---|---|
| StarCraft II | CASC + MNDX | **fait** — 20 voix |
| Warcraft III | *dans* le stockage de StarCraft II | atteignable : 2 128 `.ogg` frFR sous `mods\war3.sc2mod\`, rangés par héros — il manque une recette « personnage = dossier » |
| Diablo IV | CASC + TVFS | **bloqué** — le stockage ouvre (`fenris`, 1 855 234 entrées) mais 291 seulement sont nommées ; `frFR.speech` est un index de 229 ko, pas l'audio |
| Cyberpunk 2077 | RDAR + Oodle + Wwise | recette connue (`^judy_`, `^panam_`, `^sobchak_`) mais lecteur entier à écrire, plus le dictionnaire de chemins — les archives ne stockent que le FNV1a64 du chemin |

## Ce qui reste ouvert

- **Aucune écoute n'a été faite.** Les WAV sont sains par la mesure — crête, RMS, part de
  silence — mais « on reconnaît le personnage » ne se mesure pas ici.
- **Le journal n'arrive qu'à la fin.** La fabrication bloque deux à quatre minutes ; c'est la
  même façon de faire que `forger` et `prechauffer`, mais un fil de progression serait mieux.
- **La fréquence reste celle des sources.** Ventriloque ramène tout en 24 kHz mono à la lecture ;
  écrire directement à 24 kHz diviserait le poids par deux, au prix d'un rééchantillonneur.
- **`outils\extraire-casc\` et `outils\monter-voix\` sont absorbés** par ce binaire, qui fait les
  deux. Ils restent en place, et `--lister` remplace le premier.
