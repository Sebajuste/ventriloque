# Fabrique les deux paquets qui sortent du jeu de modeles : le moteur, et les voix libres.
#
# POURQUOI DEUX ET PAS UN. Le moteur et le catalogue vivent dans le meme dossier chez Kyutai,
# mais ils ne repondent pas a la meme question :
#
#   moteur       les .onnx, le tokenizer, le bos. Sans eux, rien ne parle et rien ne clone.
#                Indispensable, une fois par machine.
#   voix libres  les neuf .kv du catalogue. Des voix toutes faites, lues UNIQUEMENT a la demande
#                quand on en choisit une. Un pack de voix comme Cyberpunk ou StarCraft.
#
# Les livrer ensemble obligeait a choisir entre un paquet de 364 Mio et une variante allegee qui
# repetait les memes .onnx -- une duplication qui n'avait aucune raison d'exister.
#
# (les voix de catalogue atterrissent sous models/catalogue/ parce que c'est la que le moteur
# les cherche : cfg_.models_dir + "/catalogue/" + nom + ".kv". Le dossier est celui du moteur,
# la nature du contenu est celle d'un pack de voix.)
#
# CE QUI NE SE PARTAGE PAS. Le moteur vient du depot `kyutai/pocket-tts`, sur liste
# d'autorisation : l'acces est accorde nominativement, et fabriquer le paquet chez soi n'est pas
# le redistribuer. Legitime d'une de tes machines a l'autre, pas au-dela. Les voix de catalogue
# viennent de corpus ouverts (VCTK, EARS) et de poids CC-BY-4.0.
#
# DEFLATE, ET PAS STORED. Mesure du 2026-09-06 : les .onnx int8 se compressent de 29 a 53 %.
# Une premiere version ecrivait sans compression, sur l'idee qu'un modele int8 est deja compact.
# Ce n'est vrai que du catalogue, qui ne gagne que 7 % -- et c'est lui qui m'avait fait
# generaliser a tort.
#
#   flow_lm_main_int8.onnx    289,3 -> 204,0 Mio   29 %
#   mimi_encoder.onnx          37,5 ->  17,7 Mio   53 %
#   flow_lm_flow.onnx          37,3 ->  17,6 Mio   53 %
#   mimi_decoder_int8.onnx     21,6 ->  11,6 Mio   46 %
#   text_conditioner.onnx      15,6 ->   7,3 Mio   53 %
#   catalogue/*.kv            114,1 -> 105,9 Mio    7 %

import argparse
import json
import os
import time
import zipfile

# LE DOSSIER DES MODELES N'EST PAS DANS CE DEPOT et ne peut pas y etre : un demi-gigaoctet,
# venu de `kyutai/pocket-tts` sur liste d'autorisation. On le nomme donc au lancement, par
# `--source` ou par la variable d'environnement VENTRILOQUE_MODELES. Aucun chemin en dur :
# ce script doit tourner sur n'importe laquelle de tes machines.
SOURCE = os.environ.get("VENTRILOQUE_MODELES", "")
DEST = "packs-a-distribuer"

MOTEUR = {
    "cible": "ventriloque-moteur-fr-USAGE-PERSONNEL.zip",
    "manifeste": {
        "name": "Moteur de parole francais",
        "version": "pocket-tts 2.1.0 / fr_24l int8",
        "author": "Kyutai Labs (poids CC-BY-4.0) - pack de clonage, usage personnel",
        "description": "Ce qui parle et ce qui clone. Indispensable, et sans aucune voix.",
    },
}

VOIX_LIBRES = {
    "cible": "ventriloque-voix-libres-fr.zip",
    "manifeste": {
        "name": "Voix libres",
        "version": "1.0",
        "author": "Kyutai Labs, d'apres les corpus ouverts VCTK et EARS (CC-BY-4.0)",
        "description": "Neuf voix toutes faites, sans clonage et sans personnage.",
    },
}


def ecrire(cible, manifeste, fichiers, source):
    """fichiers : liste de chemins relatifs a `source`, poses sous models/ dans le zip."""
    chemin = os.path.join(DEST, cible)
    debut = time.time()
    brut = 0
    with zipfile.ZipFile(chemin, "w", zipfile.ZIP_DEFLATED, allowZip64=True, compresslevel=6) as z:
        z.writestr("pack.json", json.dumps(manifeste, ensure_ascii=False, indent=2))
        for rel in fichiers:
            plein = os.path.join(source, rel)
            z.write(plein, "models/" + rel.replace(os.sep, "/"))
            brut += os.path.getsize(plein)
    poids = os.path.getsize(chemin)
    gain = 100 - 100 * poids / brut if brut else 0
    print(
        f"{chemin}\n"
        f"  {len(fichiers)} fichiers | {brut / 1048576:.1f} Mio bruts -> {poids / 1048576:.1f} Mio "
        f"({gain:.0f} % gagnes) en {time.time() - debut:.0f} s"
    )


def main():
    a = argparse.ArgumentParser(description="Fabrique le paquet moteur et le paquet des voix libres.")
    a.add_argument("--source", default=SOURCE, help="le dossier fr-cloning des modeles")
    args = a.parse_args()

    if not args.source:
        raise SystemExit(
            "ou sont les modeles ? Donne --source <dossier fr-cloning>, ou pose "
            "VENTRILOQUE_MODELES dans ton environnement."
        )
    if not os.path.isdir(args.source):
        raise SystemExit(f"dossier introuvable : {args.source}")

    # Le fichier qui distingue le jeu de clonage du jeu libre. Sans lui, le binaire autonome ne
    # se contente pas de refuser de cloner : il ne demarre pas du tout.
    if not os.path.isfile(os.path.join(args.source, "mimi_encoder.onnx")):
        raise SystemExit(
            f"mimi_encoder.onnx est absent de {args.source} : c'est le jeu LIBRE, "
            "et le moteur autonome ne demarrera pas avec."
        )

    moteur, catalogue = [], []
    for racine, _, fichiers in os.walk(args.source):
        for f in fichiers:
            rel = os.path.relpath(os.path.join(racine, f), args.source)
            (catalogue if rel.replace(os.sep, "/").startswith("catalogue/") else moteur).append(rel)

    os.makedirs(DEST, exist_ok=True)
    ecrire(MOTEUR["cible"], MOTEUR["manifeste"], sorted(moteur), args.source)
    ecrire(VOIX_LIBRES["cible"], VOIX_LIBRES["manifeste"], sorted(catalogue), args.source)


if __name__ == "__main__":
    main()
