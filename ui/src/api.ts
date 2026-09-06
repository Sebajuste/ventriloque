// Le seul endroit qui parle au Rust.
//
// Les commandes sont typées ici et nulle part ailleurs : un nom de commande mal orthographié ou
// un argument oublié devient une erreur de compilation plutôt qu'une promesse rejetée à
// l'exécution, au moment où l'on joue.
//
// Les noms des champs suivent ceux que Rust sérialise — `installe_le` et pas `installeLe` —
// parce que ces structures viennent telles quelles du disque et que les renommer ici obligerait
// à traduire dans les deux sens pour rien.

import { invoke } from "@tauri-apps/api/core";

export type Palier = "clone" | "catalogue";

export interface Voix {
  nom: string;
  /** Ce qu'on envoie au moteur : `judy.wav` pour un clone, `jean` pour une voix de catalogue. */
  reference: string;
  palier: Palier;
}

export interface Fiche {
  id: string;
  nom: string;
  univers: string;
  voix: string;
  repliques: string[];
}

export interface Manifeste {
  nom: string;
  version: string;
  description: string;
  auteur: string;
  fichiers: string[];
  installe_le: string;
}

export interface Etat {
  pret: boolean;
  panne: string;
  racine: string;
  modeles: string;
  voix: Voix[];
  fiches: Fiche[];
  packs: Manifeste[];
  peripheriques: string[];
  peripherique: number;
}

/** Qui parle, en ce moment, dans le player. Une fiche et une voix brute y arrivent pareilles. */
export interface Cible {
  nom: string;
  reference: string;
  palier: Palier | null;
  repliques: string[];
}

export const lireEtat = () => invoke<Etat>("etat");

/**
 * Ne se résout que lorsque la réplique est SORTIE DU HAUT-PARLEUR, et pas quand son calcul est
 * fini : le moteur fabrique environ trois fois plus vite qu'on n'écoute. Le son, lui, commence
 * ~150 ms après l'appel.
 */
export const parler = (reference: string, texte: string) =>
  invoke<void>("parler", { reference, texte });

export const taire = () => invoke<void>("taire");

export const choisirPeripherique = (rang: number) =>
  invoke<void>("choisir_peripherique", { rang });

export const choisirFichiers = () => invoke<string[]>("choisir_fichiers");

export const forger = (nom: string, fichiers: string[], pitch: number, formants: number) =>
  invoke<string>("forger", { nom, fichiers, pitch, formants });

/** `ancien` porte l'identifiant d'avant, pour qu'un renommage ne laisse pas de doublon. */
export const ecrireFiche = (fiche: Fiche, ancien: string) =>
  invoke<Fiche>("ecrire_fiche", { fiche, ancien });

export const supprimerFiche = (id: string) => invoke<void>("supprimer_fiche", { id });

export const prechauffer = () => invoke<string[]>("prechauffer");

export const installerPack = () => invoke<string>("installer_pack");

/** Ce que Rust renvoie en cas d'erreur est une chaîne ; le reste est un imprévu. */
export const enClair = (e: unknown) => (typeof e === "string" ? e : String(e));
