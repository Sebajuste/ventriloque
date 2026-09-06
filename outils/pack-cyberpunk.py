# Fabrique le paquet Cyberpunk 2077 : les neuf voix et les neuf fiches.
#
# LES VOIX NE SONT PAS DANS CE DEPOT et ne peuvent pas y etre. Elles sont extraites des archives
# du jeu par `ai_npc-holo`, sur la copie du joueur, et ce sont des enregistrements d'acteurs.
# Ce script les prend la ou elles ont ete faites et les emballe ; il ne les redistribue pas.
#
# LES REPLIQUES SONT ECRITES POUR LA TABLE, pas reprises du jeu. Trois par personnage, courtes,
# choisies pour servir a peu pres n'importe quelle scene : un accueil, une fin de non-recevoir,
# une relance. Ce sont celles qu'on redit vingt fois dans une soiree, donc celles qui gagnent a
# etre a un clic.

import argparse
import glob
import json
import os
import zipfile

# LES VOIX VIENNENT D'UN AUTRE PROJET, celui qui les extrait du jeu. Son emplacement ne
# regarde pas ce depot : on le nomme par `--source`, ou par VENTRILOQUE_VOIX_CP77.
SOURCE = os.environ.get("VENTRILOQUE_VOIX_CP77", "")
CIBLE = "packs-a-distribuer/ventriloque-cyberpunk-2077-USAGE-PERSONNEL.zip"

MANIFESTE = {
    "nom": "Cyberpunk 2077",
    "version": "1.2",
    "auteur": "voix extraites par ai_npc-holo depuis une copie du jeu - usage personnel",
    "description": "Neuf habitants de Night City, avec leurs fiches et leurs repliques.",
}

# fichier de voix -> (nom affiche, repliques)
CAST = {
    "jackie.wav": (
        "Jackie Welles",
        [
            "Hé, mon pote ! Ça faisait un bail.",
            "T'inquiète, je gère. On fait ça propre.",
            "Un verre au Coyote, et on en reparle.",
        ],
    ),
    "judy.wav": (
        "Judy Alvarez",
        [
            "Passe-moi ça, je vais voir ce que ça donne.",
            "J'ai pas le temps pour les conneries des corpos.",
            "T'es sûr de toi ? Parce que moi, non.",
        ],
    ),
    "kerry_eurodyne.wav": (
        "Kerry Eurodyne",
        [
            "Tu sais qui je suis, ou tu fais semblant ?",
            "C'était pas mieux avant. C'était juste plus fort.",
            "Fais pas cette tête, c'est que du bruit.",
        ],
    ),
    "panam.wav": (
        "Panam Palmer",
        [
            "On fait ça à ma façon, ou on le fait pas.",
            "T'as intérêt à pas me décevoir.",
            "Monte. On roule.",
        ],
    ),
    "river_ward.wav": (
        "River Ward",
        [
            "Je suis pas là en tant que flic. Pas ce soir.",
            "Raconte-moi tout, depuis le début.",
            "Y a des choses qu'on peut pas laisser passer.",
        ],
    ),
    "rogue.wav": (
        "Rogue Amendiares",
        [
            "Le prix a changé. C'est comme ça.",
            "J'ai pas besoin de connaître tes raisons.",
            "Assieds-toi. Et parle vite.",
        ],
    ),
    "songbird.wav": (
        "Songbird",
        [
            "Je sais déjà ce que tu vas demander.",
            "Rien de ce que tu vois ici n'est vraiment là.",
            "Fais-moi confiance. Une dernière fois.",
        ],
    ),
    "takemura.wav": (
        "Goro Takemura",
        [
            "Il y a une manière correcte de faire les choses.",
            "Ma loyauté ne s'achète pas.",
            "Vous jugez trop vite. Attendez.",
        ],
    ),
    "victor_vector.wav": (
        "Viktor Vector",
        [
            "Allonge-toi. Ça va piquer un peu.",
            "T'as encore repoussé la limite, hein ?",
            "Paye quand tu peux. Mais paye.",
        ],
    ),
}


def identifiant(nom):
    """La meme regle que `fiches::identifiant` en Rust : le nom du fichier fait foi a la
    lecture, donc il doit etre celui que Rust aurait produit."""
    brut = "".join(c if c.isalnum() else "_" for c in nom.lower())
    return brut.strip("_")


def main():
    a = argparse.ArgumentParser(description="Emballe les neuf voix de Night City.")
    a.add_argument("--source", default=SOURCE, help="le dossier des .wav extraits")
    args = a.parse_args()

    if not args.source:
        raise SystemExit(
            "ou sont les voix ? Donne --source <dossier>, ou pose VENTRILOQUE_VOIX_CP77."
        )
    os.makedirs("packs-a-distribuer", exist_ok=True)
    manquantes = [f for f in CAST if not os.path.isfile(os.path.join(args.source, f))]
    if manquantes:
        raise SystemExit(f"voix introuvables dans {args.source} : {', '.join(manquantes)}")

    with zipfile.ZipFile(CIBLE, "w", zipfile.ZIP_DEFLATED) as z:
        z.writestr("pack.json", json.dumps(MANIFESTE, ensure_ascii=False, indent=2))
        for fichier, (nom, repliques) in sorted(CAST.items()):
            z.write(os.path.join(args.source, fichier), "voix/" + fichier)
            fiche = {
                "id": identifiant(nom),
                "nom": nom,
                "univers": "Cyberpunk 2077",
                "voix": fichier,
                "repliques": repliques,
            }
            z.writestr(
                "pnj/" + fiche["id"] + ".json",
                json.dumps(fiche, ensure_ascii=False, indent=2),
            )

    poids = os.path.getsize(CIBLE) / 1024 / 1024
    print(f"{CIBLE} : {len(CAST)} voix + {len(CAST)} fiches, {poids:.1f} Mo")
    for fichier, (nom, _) in sorted(CAST.items(), key=lambda x: x[1][0]):
        print(f"  {nom:<20} {fichier}")

    inattendues = sorted(
        os.path.basename(w) for w in glob.glob(os.path.join(args.source, "*.wav"))
        if os.path.basename(w) not in CAST
    )
    if inattendues:
        print("voix presentes a la source mais hors du casting :", ", ".join(inattendues))


if __name__ == "__main__":
    main()
