// De quoi monter un etat de fenetre en une ligne.
//
// Les tests ne decrivent que ce qui les concerne : `snapshotOf({ characters: [...] })` laisse le
// reste a des valeurs qui n'attirent pas l'attention. Un test qui recopie les neuf champs de
// `Snapshot` ne dit plus lequel compte.

import type { Character, Manifest, Snapshot, SpeechProgress, Voice } from "../ipc";

export const aVoice = (name: string, over: Partial<Voice> = {}): Voice => ({
  name,
  reference: `${name.toLowerCase()}.wav`,
  kind: "clone",
  ...over,
});

export const aCharacter = (name: string, over: Partial<Character> = {}): Character => ({
  id: name.toLowerCase().replace(/\s+/g, "_"),
  name,
  universe: "Nulle part",
  voice: `${name.toLowerCase()}.wav`,
  lines: [],
  ...over,
});

export const aPack = (name: string, over: Partial<Manifest> = {}): Manifest => ({
  name,
  version: "1.0",
  description: "",
  author: "",
  recipe: "",
  game: "",
  product: "",
  files: [],
  installed_on: "2026-09-06",
  built_on: "",
  ...over,
});

export const snapshotOf = (over: Partial<Snapshot> = {}): Snapshot => ({
  ready: true,
  failure: "",
  root: "D:/ventriloque",
  models: "D:/ventriloque/models",
  voices: [],
  characters: [],
  packs: [],
  devices: ["Haut-parleurs"],
  device: 0,
  ...over,
});

/** Rien n'est encore sorti du haut-parleur : la voix se prepare. */
export const SILENT: SpeechProgress = {
  position: 0,
  duration: 0,
  complete: false,
  audible: false,
};

/** Du son est sorti : la replique s'entend. */
export const heard = (over: Partial<SpeechProgress>): SpeechProgress => ({
  ...SILENT,
  audible: true,
  ...over,
});

/**
 * Une promesse qu'on resout quand le test le decide.
 *
 * C'est ce qui permet de garder une replique « en vol » : la file du player se decrit par des
 * appels qui se CHEVAUCHENT, et un faux qui repond tout de suite n'en laisse jamais deux.
 */
export const deferred = <T>() => {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((ok, ko) => {
    resolve = ok;
    reject = ko;
  });
  return { promise, resolve, reject };
};
