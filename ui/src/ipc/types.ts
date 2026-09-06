// Les types que la fenetre manipule, derives de ceux que Rust expose.
//
// LES TYPES NE SONT PAS ECRITS ICI, ILS SONT DERIVES DE `bindings.ts`, que `cargo test`
// regenere depuis les commandes de `src-tauri`. Un champ renomme cote Rust renomme le champ ici,
// et les vues qui le lisaient ne compilent plus -- la ou avant elles compilaient tres bien et
// cassaient en seance, sur une promesse rejetee au moment ou l'on joue.

import type {
  Character as RawCharacter,
  Manifest as RawManifest,
  Snapshot as RawSnapshot,
  Voice as RawVoice,
  VoiceKind,
} from "./bindings";

export type { PackProgress, SpeechProgress, VoiceKind } from "./bindings";

/**
 * Rend obligatoire ce que `bindings.ts` declare facultatif, en profondeur.
 *
 * Les `#[serde(default)]` cote Rust decrivent la LECTURE d'un fichier a moitie rempli sur le
 * disque ; ils s'exportent en `champ?:`. Mais rien de ce qui remonte dans la fenetre n'est
 * incomplet : `serde` serialise toujours tous les champs. Sans ce passage, chaque vue devrait
 * traiter un `undefined` qui n'arrive jamais.
 *
 * En profondeur parce qu'un `Snapshot` porte des `Character[]` : les rendre obligatoires en
 * surface seulement laisserait les fiches facultatives a l'interieur.
 */
type Complete<T> = T extends (infer U)[]
  ? Complete<U>[]
  : T extends object
    ? { [K in keyof T]-?: Complete<T[K]> }
    : T;

export type Voice = Complete<RawVoice>;
export type Character = Complete<RawCharacter>;
export type Manifest = Complete<RawManifest>;
export type Snapshot = Complete<RawSnapshot>;

/** Qui parle, en ce moment, dans le player. Une fiche et une voix brute y arrivent pareilles. */
export interface Target {
  name: string;
  reference: string;
  kind: VoiceKind | null;
  lines: string[];
}
