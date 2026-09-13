// Le seul endroit qui parle au Rust.
//
// Ce fichier ne garde que ce que Rust ne peut pas dire : quand une commande se resout, ce qu'elle
// coute, et pourquoi elle est faite ainsi. Les types, eux, viennent de `bindings.ts` -- voir
// `types.ts`.

import { commands } from "./bindings";
import type { Character, EngineSettings, EngineTuning, Snapshot } from "./types";

/**
 * Rend a l'appelant la promesse rejetee qu'il attend.
 *
 * tauri-specta rend les `Result` de Rust en `{ status: "ok" | "error" }` plutot qu'en promesse
 * rejetee. C'est plus sur en soi, mais toutes les vues sont ecrites autour d'un `try`/`catch`,
 * et les convertir aurait change leur code sans rien apporter : la conversion tient en trois
 * lignes, ici, une fois.
 */
type Reply<T, E> = { status: "ok"; data: T } | { status: "error"; error: E };

const unwrap = async <T, E>(reply: Promise<Reply<T, E>>): Promise<T> => {
  const r = await reply;
  if (r.status === "error") throw r.error;
  return r.data;
};

/** Ce que Rust renvoie en cas d'erreur est une chaine ; le reste est un imprevu. */
export const asMessage = (e: unknown) => (typeof e === "string" ? e : String(e));

// ── L'etat de l'application ─────────────────────────────────────────────────

export const readSnapshot = () => commands.snapshot() as Promise<Snapshot>;

// ── La parole ───────────────────────────────────────────────────────────────

/**
 * Ne se resout que lorsque la replique est SORTIE DU HAUT-PARLEUR, et pas quand son calcul est
 * fini : le moteur fabrique environ trois fois plus vite qu'on n'ecoute. Le son, lui, commence
 * ~150 ms apres l'appel.
 *
 * `pace` en pourcent du debit du moteur : la lecture etire le son sans toucher a sa hauteur.
 *
 * Rend le numero de la prise, a donner a `replay` pour la redire telle quelle — ou `null` si la
 * synthese a ete coupee avant sa fin.
 */
export const speak = (reference: string, text: string, pace: number): Promise<number | null> =>
  unwrap(commands.speak(reference, text, pace));

/**
 * Redit une prise deja entendue, sans le moteur : meme son, sans attente de calcul. Se resout,
 * comme `speak`, une fois le son sorti.
 *
 * La voix, le texte et le debit accompagnent le numero : Rust ne rend la prise que s'ils
 * correspondent, si bien qu'un numero egare ne peut pas faire parler une autre replique.
 *
 * `false` si la prise manque ou ne correspond pas : a refaire avec `speak`.
 */
export const replay = (
  take: number,
  reference: string,
  text: string,
  pace: number,
): Promise<boolean> => unwrap(commands.replay(take, reference, text, pace));

export const silence = () => commands.silence();

/**
 * Où en est la réplique en cours, en millisecondes réellement sorties du haut-parleur.
 *
 * `duration` vaut 0 tant que `complete` est faux : le moteur fabrique environ trois fois plus
 * vite qu'on n'écoute, donc le total n'est connu qu'une fois la synthèse finie. Afficher une
 * proportion avant, c'est diviser par un dénominateur qui grandit — la barre reculerait.
 */
export const speechProgress = () => commands.speechProgress();

/**
 * Suspend la réplique en cours là où elle en est. Le calcul continue de remplir son tampon
 * pendant la pause : reprendre ne redemande rien au moteur, le son repart tout de suite.
 */
export const pause = () => commands.pause();

export const resume = () => commands.resume();

export const selectDevice = (index: number) => commands.selectDevice(index);

/**
 * Paie d'avance le clonage des voix que les fiches nomment : ~6 s chacune, une seule fois.
 * Rend une ligne de compte rendu par voix.
 */
export const warmUp = () => unwrap(commands.warmUp());

// ── Les réglages du moteur ──────────────────────────────────────────────────

/** Les réglages en vigueur, et ceux d'origine — la configuration validée en jeu. */
export const engineSettings = () => commands.engineSettings() as Promise<EngineTuning>;

/**
 * Enregistre les réglages et RELANCE le moteur, qui ne les lit qu'à son lancement : ~2,5 s.
 * Rend ce qui a été écrit, ramené dans les bornes du moteur si besoin.
 */
export const applyEngineSettings = (settings: EngineSettings) =>
  unwrap(commands.applyEngineSettings(settings)) as Promise<EngineSettings>;

// ── L'atelier ───────────────────────────────────────────────────────────────

export const pickAudioFiles = () => commands.pickAudioFiles();

/** Rend le nom du fichier produit, `barman.wav`. Bloquant : quelques secondes. */
export const forgeVoice = (name: string, files: string[], pitch: number, formants: number) =>
  unwrap(commands.forgeVoice(name, files, pitch, formants));

// ── Les fiches ──────────────────────────────────────────────────────────────

/** `previousId` porte l'identifiant d'avant, pour qu'un renommage ne laisse pas de doublon. */
export const writeCharacter = (character: Character, previousId: string) =>
  unwrap(commands.writeCharacter(character, previousId)) as Promise<Character>;

export const deleteCharacter = async (id: string): Promise<void> => {
  await unwrap(commands.deleteCharacter(id));
};

// ── Les paquets ─────────────────────────────────────────────────────────────

export const installPack = () => unwrap(commands.installPack());

/**
 * Où en est l'opération sur les paquets. À lire à intervalle pendant qu'elle tourne : les
 * commandes ne rendent la main qu'à la fin, c'est ce battement qui fait vivre la barre.
 *
 * `total` à zéro veut dire « en cours, sans compte connu » — une fabrication annonce ses
 * étapes au fur et à mesure sans les compter d'avance.
 */
export const packProgress = () => commands.packProgress();

/**
 * Retire les fichiers d'un paquet et son inscription. Ce qu'un autre paquet installé revendique
 * aussi est laissé en place.
 */
export const uninstallPack = (pack: string) => unwrap(commands.uninstallPack(pack));

/**
 * Fait tourner la recette d'un paquet sur une copie installée du jeu. Ouvre d'abord un sélecteur
 * de dossier — c'est là que l'utilisateur consent à exécuter un script venu d'ailleurs.
 *
 * Le dossier du jeu est trouvé tout seul quand c'est possible ; `chooseFolder` force la question,
 * pour le cas où la détection tomberait sur la mauvaise installation.
 *
 * Rend le journal de la fabrication, ou une chaîne vide si le sélecteur a été annulé.
 */
export const buildPack = (pack: string, chooseFolder = false) =>
  unwrap(commands.buildPack(pack, chooseFolder));
