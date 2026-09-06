# extraire-casc — sortir les voix d'une installation Blizzard

Deux outils et un script, qui vont ensemble :

| | |
|---|---|
| `extraire-casc.exe` | lit le stockage CASC du jeu, liste et extrait des fichiers |
| `..\assemble-voice\` | décode les `.ogg`, en assemble une référence de clonage `.wav` |
| `paquet-sc2.py` | enchaîne les deux et fabrique le paquet Ventriloque |

Tout se fait **hors du jeu**, **en lecture seule**, **sans réseau**. Le résultat reste sur la
machine : ce sont des enregistrements d'acteurs tirés d'un jeu commercial.

---

## Le lancer

```powershell
# une seule fois : bâtir les deux outils
git clone --depth 1 https://github.com/ladislav-zezula/CascLib.git tools\extract-casc\vendor\CascLib
powershell -File tools\extract-casc\build.ps1
cargo build --release --manifest-path tools\assemble-voice\Cargo.toml

# puis, à chaque fois
python tools\extract-casc\paquet-sc2.py
```

Environ trois minutes, dont deux pour les deux parcours du stockage. `--reprendre` les saute si
le dossier de travail les porte déjà ; `--jeu` et `--travail` déplacent l'installation lue et le
dossier de travail (`D:\Jeux\StarCraft II` et `D:\tmp\sc2-voix` par défaut).

Le paquet sort dans `packs-a-distribuer\ventriloque-starcraft-2-USAGE-PERSONNEL.zip` : vingt
voix, vingt fiches, une cinquantaine de mégaoctets. Il s'installe par l'onglet Paquets.

`extraire-casc.exe` sert aussi seul, pour fouiller :

```powershell
extraire-casc lister   "D:\Jeux\StarCraft II" "*frfr.sc2assets*.ogg" --sortie liste.txt
extraire-casc extraire "D:\Jeux\StarCraft II" "*_kerrigan_*.ogg" D:\tmp\kerrigan --plat
extraire-casc extraire-liste "D:\Jeux\StarCraft II" choix.txt D:\tmp\choix --plat
```

`lister` écrit « taille‹TAB›nom » ; `extraire-liste` relit ce format. C'est ce couple qui évite
de reparcourir 780 000 entrées pour extraire quarante fichiers.

---

## Ce qui a été tranché

### CascLib se bâtit avec `cl.exe`

C'était la question qui décidait du budget, et la réponse est oui. CascLib porte
`sources-c.c` et `sources-cpp.cpp`, deux fichiers de compilation unifiée qui incluent tout le
reste : trois appels à `cl.exe` et un `link` suffisent, ce que fait `build.ps1`.

```
cl /nologo /c /O2 /MT /EHsc /D_CRT_SECURE_NO_WARNINGS /DCASCLIB_NO_AUTO_LINK_LIBRARY sources-c.c
cl ... sources-cpp.cpp
cl ... /I vendor\CascLib\src src\extraire.cpp
link /nologo /OUT:extraire-casc.exe sources-c.obj sources-cpp.obj extraire.obj advapi32.lib ws2_32.lib
```

Sorti en 0,67 Mo, sans dépendance à l'exécution. Le brief annonçait CascLib sans CMake : c'est
faux, le dépôt en porte un — mais `cmake` n'est pas installé sur cette machine, et la voie
`cl.exe` reste la plus courte.

**MNDX, l'obstacle annoncé, n'en a pas été un.** CascLib le traite ; `CascOpenStorage` sur la
racine du jeu rend 779 862 entrées nommées, dont 353 385 `.ogg`. Aucune clé de déchiffrement
n'a manqué : tout ce qui a été demandé est sorti.

### Où sont réellement les voix

Pas sous `Assets/Sounds/VO/` comme le supposait le brief. La disposition est :

```
<campagne ou mod>\<locale>.sc2assets\localizeddata\sounds\vo\<scène>_<locuteur>_<numéro>.ogg
```

Par exemple `campaigns\swarmstory.sc2campaign\frfr.sc2assets\localizeddata\sounds\vo\zbriefing_char03_kerrigan_007.ogg`.

Le doublage est une branche par langue : `frfr.sc2assets` porte la voix française,
`enus.sc2assets` l'anglaise, et les deux existent côte à côte — 28 348 `.ogg` en frFR. Les
sous-titres, eux, vivent ailleurs, dans `<locale>.sc2data` : ce sont bien deux réglages
distincts.

Les grands gisements de voix française :

| dossier | fichiers |
|---|---|
| `campaigns\void.sc2campaign` | 8 098 |
| `mods\liberty.sc2mod` | 5 666 |
| `campaigns\liberty.sc2campaign` | 3 702 |
| `campaigns\swarmstory.sc2campaign` | 3 697 |
| `campaigns\voidstory.sc2campaign` | 3 335 |

Le locuteur est le jeton qui précède le numéro. Kerrigan pèse 1 797 répliques françaises,
Artanis 963, Nova 521, Raynor 363.

### Le texte de chaque prise se retrouve

C'est la trouvaille qui fait la qualité du paquet. Les tables de sous-titres,
`<locale>.sc2data\localizeddata\conversationstrings.txt`, portent des clés de la forme
`Conversation/<scène>/Line<numéro>` — **la même scène et le même numéro que le nom du `.ogg`** :

```
zbriefing_char03_kerrigan_007.ogg
Conversation/zBriefing_Char03/Line00007=Soldats, l'Essaim de Kerrigan est sur le plateau. […]
```

Aucun format binaire à ouvrir, aucun `.SC2Conv` à décoder : une jointure sur un nom de fichier.
16 887 sous-titres français sont récupérés ainsi, ce qui permet de **ne retenir que des prises
dont on sait qu'elles sont des phrases** — pas un cri, pas un râle de mort, pas un « oui,
commandant » — et de remplir les répliques des fiches avec le texte exact du jeu.

Quatre clés apparaissent avec deux textes différents, deux mods partageant un nom de scène ;
elles sont écartées plutôt que risquer de coller la mauvaise phrase sur une prise.

---

## Comment la référence est fabriquée

`assemble-voice` décode chaque `.ogg` (symphonia, en Rust pur), replie en mono, coupe les silences
de bord à −42 dBFS avec 30 ms de marge, puis retient les prises dont la durée tombe entre
`--min` et `--max`. Il en empile jusqu'à `--duree` secondes **en alternant les scènes** : une
référence tirée d'un seul briefing ne porte qu'une humeur. Sortie en WAV mono 16 bits à la
fréquence des sources — 44 100 Hz pour StarCraft II — normalisée à −1 dBFS.

`paquet-sc2.py` vise 32 secondes en prises de 1 à 4 secondes, ce qui donne neuf à quinze
répliques par personnage. Les fichiers sont pré-filtrés sur leur taille (20 à 70 ko, le doublage
tournant autour de 18 ko la seconde) pour ne pas décoder des milliers de `.ogg` inutilement.

Un personnage qui ne parle jamais court — Amon n'a pas une réplique sous les quatre secondes —
bascule sur une fenêtre large plutôt que d'être écarté.

**Les voix sont préfixées `sc2_`.** Le moteur met en cache l'état de clonage sous le seul radical
du fichier : un `kerrigan.wav` d'un autre univers hériterait de celui-ci sans avertissement.

---

## Ce qui reste ouvert

- **Le paquet pèse 51 Mo** parce que les références sont en 44,1 kHz. Ventriloque ramène tout en
  24 kHz mono à la lecture ; écrire directement à 24 kHz diviserait le poids par deux, au prix
  d'un rééchantillonneur dans `assemble-voice`.
- **Aucune écoute n'a été faite** — le critère qui compte, « on reconnaît le personnage », ne se
  mesure pas ici. Les prises sont propres par construction (pistes de dialogue, sans musique),
  mais rien ne garantit qu'aucune ne porte un effet radio ou un traitement de voix zerg.
- **Vingt personnages** sont au casting dans `paquet-sc2.py`. La liste est une table en tête de
  fichier ; en ajouter un revient à écrire son nom et ses jetons de locuteur.
- **Les autres jeux Blizzard** passeraient par le même `extraire-casc.exe` sans modification —
  c'est CascLib qui fait le travail. Seule la partie « où sont les voix, comment s'appellent les
  fichiers » est propre à StarCraft II, et elle tient dans `paquet-sc2.py`.
