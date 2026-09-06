// Le seul endroit qui parle au Rust.
//
// LES TYPES NE SONT PLUS ECRITS ICI, ILS SONT DERIVES DE `lien.ts`, que `cargo test` regenere
// depuis les commandes de `src-tauri`. Un champ renomme cote Rust renomme le champ ici, et les
// vues qui le lisaient ne compilent plus -- la ou avant elles compilaient tres bien et cassaient
// en seance, sur une promesse rejetee au moment ou l'on joue.
//
// Ce fichier ne garde donc que ce que Rust ne peut pas dire : les noms francais des enveloppes,
// les commentaires qui expliquent quand une commande se resout, et `Cible`, qui n'existe que
// dans la fenetre.

import { commands, type Etat as EtatBrut, type Fiche as FicheBrute } from "./lien";
import type { Manifeste as ManifesteBrut, Palier, Voix as VoixBrute } from "./lien";

export type { Palier };

/**
 * Rend obligatoire ce que `lien.ts` declare facultatif, en profondeur.
 *
 * Les `#[serde(default)]` cote Rust decrivent la LECTURE d'un fichier a moitie rempli sur le
 * disque ; ils s'exportent en `champ?:`. Mais rien de ce qui remonte dans la fenetre n'est
 * incomplet : `serde` serialise toujours tous les champs. Sans ce passage, chaque vue devrait
 * traiter un `undefined` qui n'arrive jamais.
 *
 * En profondeur parce qu'un `Etat` porte des `Fiche[]` : les rendre obligatoires en surface
 * seulement laisserait les fiches facultatives a l'interieur.
 */
type Requis<T> = T extends (infer U)[]
  ? Requis<U>[]
  : T extends object
    ? { [K in keyof T]-?: Requis<T[K]> }
    : T;

export type Voix = Requis<VoixBrute>;
export type Fiche = Requis<FicheBrute>;
export type Manifeste = Requis<ManifesteBrut>;
export type Etat = Requis<EtatBrut>;

/** Qui parle, en ce moment, dans le player. Une fiche et une voix brute y arrivent pareilles. */
export interface Cible {
  nom: string;
  reference: string;
  palier: Palier | null;
  repliques: string[];
}

/**
 * Rend a l'appelant la promesse rejetee qu'il attend.
 *
 * tauri-specta rend les `Result` de Rust en `{ status: "ok" | "error" }` plutot qu'en promesse
 * rejetee. C'est plus sur en soi, mais toutes les vues sont ecrites autour d'un `try`/`catch`,
 * et les convertir aurait change leur code sans rien apporter : la conversion tient en trois
 * lignes, ici, une fois.
 */
type Reponse<T, E> = { status: "ok"; data: T } | { status: "error"; error: E };

const deballer = async <T, E>(reponse: Promise<Reponse<T, E>>): Promise<T> => {
  const r = await reponse;
  if (r.status === "error") throw r.error;
  return r.data;
};

export const lireEtat = () => commands.etat() as Promise<Etat>;

/**
 * Ne se resout que lorsque la replique est SORTIE DU HAUT-PARLEUR, et pas quand son calcul est
 * fini : le moteur fabrique environ trois fois plus vite qu'on n'ecoute. Le son, lui, commence
 * ~150 ms apres l'appel.
 */
export const parler = async (reference: string, texte: string): Promise<void> => {
  await deballer(commands.parler(reference, texte));
};

export const taire = () => commands.taire();

export const choisirPeripherique = (rang: number) => commands.choisirPeripherique(rang);

export const choisirFichiers = () => commands.choisirFichiers();

export const forger = (nom: string, fichiers: string[], pitch: number, formants: number) =>
  deballer(commands.forger(nom, fichiers, pitch, formants));

/** `ancien` porte l'identifiant d'avant, pour qu'un renommage ne laisse pas de doublon. */
export const ecrireFiche = (fiche: Fiche, ancien: string) =>
  deballer(commands.ecrireFiche(fiche, ancien)) as Promise<Fiche>;

export const supprimerFiche = async (id: string): Promise<void> => {
  await deballer(commands.supprimerFiche(id));
};

export const prechauffer = () => deballer(commands.prechauffer());

export const installerPack = () => deballer(commands.installerPack());

/** Ce que Rust renvoie en cas d'erreur est une chaine ; le reste est un imprevu. */
export const enClair = (e: unknown) => (typeof e === "string" ? e : String(e));
