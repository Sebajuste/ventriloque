// Dépose l'exécutable portable dans `dist\`, après que Tauri l'a bâti.
//
// Lancé par le script npm `postbuild`, que npm enchaîne tout seul après `build`. Tauri laisse
// son binaire dans `src-tauri\target\release\` ; `dist\` est le seul endroit stable où le
// chercher, et c'est ce qu'on copie sur une clé.
//
// EN NODE ET PAS EN POWERSHELL parce que c'est une copie de fichier, pas une décision : rien
// ici n'a besoin de Windows, et npm sait déjà lancer node.
//
//   VENTRILOQUE_BAC=<dossier>   copie aussi là, si le dossier existe

import { copyFileSync, existsSync, mkdirSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const racine = dirname(dirname(fileURLToPath(import.meta.url)));
const exe = join(racine, "src-tauri", "target", "release", "ventriloque.exe");

if (!existsSync(exe)) {
  console.error(`introuvable : ${exe}\nla compilation n'a rien produit ?`);
  process.exit(1);
}

const dist = join(racine, "dist");
mkdirSync(dist, { recursive: true });
copyFileSync(exe, join(dist, "ventriloque.exe"));

// Le bac à sable, nommé par l'environnement : ce dépôt doit se bâtir sur n'importe quelle
// machine, et personne d'autre n'a le même dossier d'essai.
const bac = process.env.VENTRILOQUE_BAC;
if (bac && existsSync(bac)) {
  copyFileSync(exe, join(bac, "ventriloque.exe"));
  console.log(`copié aussi dans ${bac}`);
}

const poids = (statSync(exe).size / 1024 / 1024).toFixed(1);
console.log(`dist\\ventriloque.exe — ${poids} Mo, portable`);
