# Fabrique le paquet StarCraft II : une voix et une fiche par personnage, depuis la copie
# installee du joueur.
#
# LES VOIX NE SONT PAS DANS CE DEPOT et ne peuvent pas y etre. Ce sont des enregistrements
# d'acteurs, tires du doublage francais du jeu. Le script les lit sur place, en lecture seule,
# et fabrique un paquet qui reste sur la machine du joueur.
#
# LES REPLIQUES, elles, SONT REPRISES DU JEU : le doublage porte un numero de ligne que la table
# de sous-titres porte aussi, si bien qu'on retrouve le texte exact de chaque prise. C'est ce
# qui permet de ne retenir que des repliques vraiment parlees -- pas un grognement, pas un cri.
#
# Enchainement :
#   1. `extraire-casc lister`   -> l'inventaire des .ogg frFR et des tables de sous-titres
#   2. jointure nom de fichier <-> sous-titre, puis choix des candidats par personnage
#   3. `extraire-casc extraire-liste` -> les .ogg sur le disque
#   4. `assemble-voice`            -> une reference .wav de trente secondes par personnage
#   5. le zip, au format que `packs.rs` sait installer
#
# Usage :
#   python tools\extract-casc\paquet-sc2.py
#   python tools\extract-casc\paquet-sc2.py --jeu "D:\Jeux\StarCraft II" --travail D:\tmp\sc2
#   python tools\extract-casc\paquet-sc2.py --reprendre   (garde l'inventaire deja etabli)

import argparse
import json
import os
import re
import subprocess
import sys
import zipfile

ICI = os.path.dirname(os.path.abspath(__file__))
RACINE = os.path.dirname(os.path.dirname(ICI))
EXTRACTEUR = os.path.join(ICI, "extraire-casc.exe")
MONTEUR = os.path.join(RACINE, "tools", "assemble-voice", "target", "release", "assemble-voice.exe")
CIBLE = os.path.join(RACINE, "packs-a-distribuer", "ventriloque-starcraft-2-USAGE-PERSONNEL.zip")

MANIFESTE = {
    "nom": "StarCraft II",
    "version": "1.0",
    "auteur": "voix extraites d'une copie du jeu - usage personnel",
    "description": "Les voix de Koprulu, dans leur doublage francais.",
}

# Le jeton du locuteur est celui qui precede le numero dans le nom de fichier :
# `zbriefing_char03_kerrigan_007.ogg` -> scene `zbriefing_char03`, locuteur `kerrigan`.
# Plusieurs jetons designent parfois le meme personnage, selon le mode de jeu.
CASTING = [
    ("Sarah Kerrigan", ["kerrigan", "kerrigancommander"]),
    ("Jim Raynor", ["raynor", "raynorcommander"]),
    ("Artanis", ["artanis", "artaniscommander"]),
    ("Nova", ["nova", "novacommander"]),
    ("Alexei Stukov", ["stukov", "stukovcommander"]),
    ("Zeratul", ["zeratul", "zeratulac", "zeratulcommander"]),
    ("Alarak", ["alarak", "alarakcommander"]),
    ("Dehaka", ["dehaka", "dehakacommander"]),
    ("Abathur", ["abathur", "abathurcommander"]),
    ("Valerian Mengsk", ["valerian"]),
    ("Arcturus Mengsk", ["mengsk", "mengskcommander"]),
    ("Zagara", ["zagara", "zagaracommander"]),
    ("Rory Swann", ["swann", "swanncommander"]),
    ("Tychus Findlay", ["tychus", "tychuscommander"]),
    ("Matt Horner", ["horner", "hornerhan"]),
    ("Amon", ["amon"]),
    ("Izsha", ["izsha"]),
    ("Vorazun", ["vorazun", "vorazuncommander"]),
    ("Karax", ["karax", "karaxcommander"]),
    ("Egon Stetmann", ["stetmann", "stetmanncommander"]),
]

# Fenetre de taille du .ogg, en octets. Le doublage tourne autour de 18 ko la seconde : en
# dessous de 20 ko c'est une interjection, au dessus de 70 ko c'est un monologue qu'il faudrait
# couper. Filtrer ici evite de decoder des milliers de fichiers pour n'en garder que quarante.
#
# TAILLE_LARGE est le repli des personnages qui ne parlent jamais court -- Amon n'a pas une
# seule replique sous les quatre secondes. Mieux vaut une reference en phrases longues que pas
# de voix du tout.
TAILLE_MIN = 20_000
TAILLE_MAX = 70_000
TAILLE_LARGE = 200_000
PRISES_MINIMUM = 6
CANDIDATS_PAR_VOIX = 45
DUREE_VISEE = 32


def identifiant(nom):
    """La meme regle que `fiches::identifiant` en Rust : le nom du fichier fait foi a la
    lecture, donc il doit etre celui que Rust aurait produit."""
    brut = "".join(c if c.isalnum() else "_" for c in nom.lower())
    return brut.strip("_")


def execute(commande):
    resultat = subprocess.run(commande, capture_output=True, text=True, encoding="utf-8", errors="replace")
    if resultat.returncode != 0:
        sys.stderr.write(resultat.stderr or "")
        raise SystemExit(f"echec : {' '.join(commande)}")
    return resultat


def inventaire(jeu, travail, reprendre):
    """Les deux listes que tout le reste utilise : les .ogg du doublage francais, et les tables
    de sous-titres. Un parcours du stockage coute une minute, d'ou `--reprendre`."""
    oggs = os.path.join(travail, "frfr-ogg.txt")
    tables = os.path.join(travail, "frfr-conversations.txt")

    for fichier, motif in ((oggs, r"*frfr.sc2assets*.ogg"),
                           (tables, r"*frfr.sc2data\localizeddata\conversationstrings.txt")):
        if reprendre and os.path.isfile(fichier):
            continue
        print(f"inventaire : {motif}")
        execute([EXTRACTEUR, "lister", jeu, motif, "--sortie", fichier])

    return oggs, tables


def sous_titres(jeu, travail, tables, reprendre):
    """Toutes les tables de sous-titres, fondues en `(scene, numero) -> texte`.

    Une meme cle vue avec deux textes differents veut dire que deux mods se partagent un nom de
    scene : on la retire plutot que de risquer de coller la mauvaise phrase sur une prise."""
    dossier = os.path.join(travail, "conversations")
    if not reprendre or not os.path.isdir(dossier):
        print("extraction des tables de sous-titres")
        execute([EXTRACTEUR, "extraire-liste", jeu, tables, dossier])

    textes = {}
    ambigues = set()
    for racine, _, fichiers in os.walk(dossier):
        for f in fichiers:
            for ligne in open(os.path.join(racine, f), encoding="utf-8", errors="replace"):
                cle, _, valeur = ligne.strip().partition("=")
                m = re.match(r"^Conversation/([^/]+)/Line(\d+)$", cle)
                if not m:
                    continue
                # La table porte « francais /// anglais » sur une seule ligne.
                texte = valeur.split(" /// ")[0].strip()
                identite = (m.group(1).lower(), int(m.group(2)))
                if identite in textes and textes[identite] != texte:
                    ambigues.add(identite)
                textes[identite] = texte

    for identite in ambigues:
        del textes[identite]
    print(f"{len(textes)} sous-titres ({len(ambigues)} cles ambigues ecartees)")
    return textes


def repliques_utilisables(texte):
    """Une replique sert si c'est une phrase, pas un fragment de gabarit d'interface."""
    if not 30 <= len(texte) <= 160:
        return False
    if any(c in texte for c in "~<>|"):
        return False
    return texte[-1] in ".!?\u2026"


def candidats(oggs, textes):
    """Pour chaque jeton de locuteur, les prises dont on connait le texte, une par phrase."""
    par_jeton = {}
    for ligne in open(oggs, encoding="utf-8", errors="replace"):
        taille, _, chemin = ligne.strip().partition("\t")
        if not chemin.lower().endswith(".ogg"):
            continue
        taille = int(taille)
        if not TAILLE_MIN <= taille <= TAILLE_LARGE:
            continue

        radical = chemin.lower().rsplit("\\", 1)[-1][:-4]
        m = re.match(r"^(.+)_([a-z0-9]+)_(\d{2,4})$", radical)
        if not m:
            continue
        scene, jeton, numero = m.group(1), m.group(2), int(m.group(3))

        texte = textes.get((scene, numero))
        if texte is None or not repliques_utilisables(texte):
            continue

        par_jeton.setdefault(jeton, []).append((scene, taille, chemin, texte))
    return par_jeton


def choisit(prises):
    """Une prise par scene d'abord, puis on repasse : la reference doit porter plusieurs
    situations, pas quinze fois le meme ton de briefing."""
    par_scene = {}
    for scene, taille, chemin, texte in prises:
        par_scene.setdefault(scene, []).append((taille, chemin, texte))
    for v in par_scene.values():
        # Les plus courtes d'abord : dix repliques breves montrent plus de manieres de dire
        # qu'un monologue de la meme duree.
        v.sort()

    retenus, vus = [], set()
    scenes = sorted(par_scene)
    tour = 0
    while len(retenus) < CANDIDATS_PAR_VOIX:
        pris = False
        for scene in scenes:
            if tour >= len(par_scene[scene]):
                continue
            taille, chemin, texte = par_scene[scene][tour]
            pris = True
            if texte in vus:
                continue
            vus.add(texte)
            retenus.append((chemin, texte))
            if len(retenus) >= CANDIDATS_PAR_VOIX:
                break
        if not pris:
            break
        tour += 1
    return retenus


def fabrique_voix(jeu, travail, nom, jetons, par_jeton):
    toutes = [p for j in jetons for p in par_jeton.get(j, [])]
    prises = [p for p in toutes if p[1] <= TAILLE_MAX]
    duree_max = "4.0"
    if len(prises) < PRISES_MINIMUM:
        prises, duree_max = toutes, "9.0"
    if not prises:
        print(f"  {nom} : aucune prise, ecarte")
        return None

    retenus = choisit(prises)
    ident = identifiant(nom)
    dossier = os.path.join(travail, "prises", ident)
    liste = os.path.join(travail, f"{ident}.txt")
    with open(liste, "w", encoding="utf-8") as f:
        for chemin, _ in retenus:
            f.write(chemin + "\n")

    execute([EXTRACTEUR, "extraire-liste", jeu, liste, dossier, "--plat"])
    fichiers = sorted(os.path.join(dossier, f) for f in os.listdir(dossier) if f.endswith(".ogg"))
    if not fichiers:
        print(f"  {nom} : rien d'extrait, ecarte")
        return None

    voix = os.path.join(travail, "voix", f"sc2_{ident}.wav")
    os.makedirs(os.path.dirname(voix), exist_ok=True)
    resultat = execute([MONTEUR, voix, "--duree", str(DUREE_VISEE), "--min", "1.0", "--max", duree_max] + fichiers)
    print("  " + resultat.stdout.strip())

    # Trois repliques pour la table. Le texte vient du jeu ; le choix, lui, vise la phrase qu'on
    # redit vingt fois dans une soiree. En dessous de trente-cinq signes c'est un ordre de
    # combat, au dessus de cent c'est un discours : le milieu est ce qui se rejoue.
    phrases = sorted({t for _, t in retenus}, key=len)
    milieu = [p for p in phrases if 35 <= len(p) <= 100]
    phrases = (milieu or phrases)[:3]
    return voix, phrases


def main():
    parseur = argparse.ArgumentParser()
    parseur.add_argument("--jeu", default=r"D:\Jeux\StarCraft II")
    parseur.add_argument("--travail", default=r"D:\tmp\sc2-voix")
    parseur.add_argument("--reprendre", action="store_true")
    args = parseur.parse_args()

    for outil in (EXTRACTEUR, MONTEUR):
        if not os.path.isfile(outil):
            raise SystemExit(f"outil absent : {outil}\nvoir outils\\extraire-casc\\README.md")
    if not os.path.isfile(os.path.join(args.jeu, ".build.info")):
        raise SystemExit(f"pas une installation Blizzard : {args.jeu}")

    os.makedirs(args.travail, exist_ok=True)
    oggs, tables = inventaire(args.jeu, args.travail, args.reprendre)
    textes = sous_titres(args.jeu, args.travail, tables, args.reprendre)
    par_jeton = candidats(oggs, textes)

    fabriquees = []
    for nom, jetons in CASTING:
        fait = fabrique_voix(args.jeu, args.travail, nom, jetons, par_jeton)
        if fait:
            fabriquees.append((nom, fait[0], fait[1]))

    if not fabriquees:
        raise SystemExit("aucune voix fabriquee")

    os.makedirs(os.path.dirname(CIBLE), exist_ok=True)
    with zipfile.ZipFile(CIBLE, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("pack.json", json.dumps(MANIFESTE, ensure_ascii=False, indent=2))
        for nom, voix, phrases in fabriquees:
            fichier = os.path.basename(voix)
            z.write(voix, "voix/" + fichier)
            fiche = {
                "id": identifiant(nom),
                "nom": nom,
                "univers": "StarCraft II",
                "voix": fichier,
                "repliques": phrases,
            }
            z.writestr("pnj/" + fiche["id"] + ".json", json.dumps(fiche, ensure_ascii=False, indent=2))

    poids = os.path.getsize(CIBLE) / 1024 / 1024
    print(f"\n{CIBLE} : {len(fabriquees)} voix + {len(fabriquees)} fiches, {poids:.1f} Mo")


if __name__ == "__main__":
    main()
