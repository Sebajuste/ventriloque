# Emballe un paquet-recette : un manifeste et un script, rien d'autre.
#
# CE PAQUET NE PORTE AUCUN SON, et c'est ce qui le rend partageable. Un paquet de voix contient
# des enregistrements d'acteurs tires d'un jeu commercial : legitime chez soi, pas au-dela. Une
# recette ne contient que la connaissance de ou chercher — du texte. Qui possede le jeu
# reconstitue les voix en trois minutes ; qui ne le possede pas n'obtient rien.
#
# Rien ne s'execute a l'installation : Ventriloque pose le script comme une donnee, et il ne
# tourne qu'au bouton « Fabriquer », dans un processus separe. Voir `src-tauri\src\recette.rs`.
#
# Usage :
#   python outils\pack-recette.py starcraft2

import json
import os
import sys
import zipfile

RACINE = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Une entree par recette : le manifeste que le paquet portera.
RECETTES = {
    "starcraft2": {
        "nom": "StarCraft II",
        "version": "1.0",
        "auteur": "recette d'extraction - les voix restent sur votre machine",
        "description": "Vingt voix de Koprulu, fabriquees depuis votre copie du jeu.",
        "recette": "starcraft2.rhai",
        "jeu": "StarCraft II",
        # Le code de `.build.info`, qui permet de trouver l'installation sans rien demander.
        # Ce n'est pas celui de CascLib, qui dit « s2 » pour le meme jeu.
        "produit": "sc2",
    },
}


def main():
    if len(sys.argv) != 2 or sys.argv[1] not in RECETTES:
        raise SystemExit(f"usage: python outils\\pack-recette.py <{'|'.join(RECETTES)}>")

    cle = sys.argv[1]
    manifeste = RECETTES[cle]
    script = os.path.join(RACINE, "outils", "recettes", manifeste["recette"])
    if not os.path.isfile(script):
        raise SystemExit(f"recette introuvable : {script}")

    dossier = os.path.join(RACINE, "packs-a-distribuer")
    os.makedirs(dossier, exist_ok=True)
    cible = os.path.join(dossier, f"ventriloque-recette-{cle}.zip")

    with zipfile.ZipFile(cible, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("pack.json", json.dumps(manifeste, ensure_ascii=False, indent=2))
        z.write(script, "recettes/" + manifeste["recette"])

    poids = os.path.getsize(cible) / 1024
    print(f"{cible} : {manifeste['nom']}, {poids:.1f} Ko — aucune voix, seulement la recette")


if __name__ == "__main__":
    main()
