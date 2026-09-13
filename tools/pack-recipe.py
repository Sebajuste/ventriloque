# Emballe un paquet-recette : un manifeste et un script, rien d'autre.
#
# CE PAQUET NE PORTE AUCUN SON, et c'est ce qui le rend partageable. Un paquet de voix contient
# des enregistrements d'acteurs tires d'un jeu commercial : legitime chez soi, pas au-dela. Une
# recette ne contient que la connaissance de ou chercher — du texte. Qui possede le jeu
# reconstitue les voix en trois minutes ; qui ne le possede pas n'obtient rien.
#
# Rien ne s'execute a l'installation : Ventriloque pose le script comme une donnee, et il ne
# tourne qu'au bouton « Fabriquer », dans un processus separe. Voir `src-tauri\src\packs\build.rs`.
#
# Usage :
#   python tools\pack-recipe.py starcraft2

import json
import os
import sys
import zipfile

RACINE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Une entree par recette : le manifeste que le paquet portera.
RECIPES = {
    "starcraft2": {
        "name": "StarCraft II",
        "version": "1.0",
        "author": "recette d'extraction - les voix restent sur votre machine",
        "description": "Vingt voix de Koprulu, fabriquees depuis votre copie du jeu.",
        "recipe": "starcraft2.rhai",
        "game": "StarCraft II",
        # Le code de `.build.info`, qui permet de trouver l'installation sans rien demander.
        # Ce n'est pas celui de CascLib, qui dit « s2 » pour le meme jeu.
        "product": "sc2",
    },
    "cyberpunk2077": {
        "name": "Cyberpunk 2077",
        "version": "1.0",
        "author": "recette d'extraction - les voix restent sur votre machine",
        "description": "Neuf habitants de Night City, fabriques depuis votre copie du jeu.",
        "recipe": "cyberpunk2077.rhai",
        "game": "Cyberpunk 2077",
        # CYBERPUNK N'A PAS DE `.build.info` : il n'est pas de chez Blizzard, et rien a la racine
        # de son installation ne porte un code de produit. Ce champ est donc celui que le
        # fabricant rend une fois les archives ouvertes -- le meme que verifie `product()` dans la
        # recette -- et non un code lisible avant.
        "product": "cp77",
        # D'ou le marqueur : ce que l'application cherche pour reconnaitre le dossier avant
        # d'ouvrir quoi que ce soit. L'executable du jeu, sous son dossier, sans ambiguite.
        "marker": "bin/x64/Cyberpunk2077.exe",
    },
}


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in RECIPES:
        raise SystemExit(f"usage: python tools\pack-recipe.py <{'|'.join(RECIPES)}>")

    cle = sys.argv[1]
    manifeste = RECIPES[cle]
    script = os.path.join(RACINE, "tools", "recipes", manifeste["recipe"])
    if not os.path.isfile(script):
        raise SystemExit(f"recette introuvable : {script}")

    dossier = os.path.join(RACINE, "packs-a-distribuer")
    os.makedirs(dossier, exist_ok=True)
    cible = os.path.join(dossier, f"ventriloque-recette-{cle}.zip")

    with zipfile.ZipFile(cible, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("pack.json", json.dumps(manifeste, ensure_ascii=False, indent=2))
        z.write(script, "recipes/" + manifeste["recipe"])

    poids = os.path.getsize(cible) / 1024
    print(f"{cible} : {manifeste['name']}, {poids:.1f} Ko — aucune voix, seulement la recette")


if __name__ == "__main__":
    main()
