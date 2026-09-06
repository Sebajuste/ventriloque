// De quoi monter un etat de fenetre en une ligne.
//
// Les tests ne decrivent que ce qui les concerne : `etatDeTest({ fiches: [...] })` laisse le
// reste a des valeurs qui n'attirent pas l'attention. Un test qui recopie les neuf champs de
// `Etat` ne dit plus lequel compte.

import type { Etat, Fiche, Voix } from "../api";

export const voix = (nom: string, sur: Partial<Voix> = {}): Voix => ({
  nom,
  reference: `${nom.toLowerCase()}.wav`,
  palier: "clone",
  ...sur,
});

export const fiche = (nom: string, sur: Partial<Fiche> = {}): Fiche => ({
  id: nom.toLowerCase().replace(/\s+/g, "_"),
  nom,
  univers: "Nulle part",
  voix: `${nom.toLowerCase()}.wav`,
  repliques: [],
  ...sur,
});

export const etatDeTest = (sur: Partial<Etat> = {}): Etat => ({
  pret: true,
  panne: "",
  racine: "D:/ventriloque",
  modeles: "D:/ventriloque/modeles",
  voix: [],
  fiches: [],
  packs: [],
  peripheriques: ["Haut-parleurs"],
  peripherique: 0,
  ...sur,
});
