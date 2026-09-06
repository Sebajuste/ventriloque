// Ce que la fenetre prete aux pages.
//
// `target` TRAVERSE LES ROUTES : l'atelier la pose en sortant de la forge, le player la lit.
// C'est ce qui fait de « forger, entendre, corriger » un geste et pas une navigation, et c'est
// la raison pour laquelle cet etat vit dans la fenetre plutot que dans une page.
//
// DANS SON PROPRE FICHIER, et pas dans `AppShell.tsx` : les pages ont besoin du crochet, pas du
// composant. Les garder ensemble faisait importer la fenetre entiere a chaque page -- et, dans
// les tests, montait l'en-tete pour verifier une liste.

import { useOutletContext } from "react-router";

import type { Snapshot, Target } from "../ipc";

export interface ShellContext {
  snapshot: Snapshot;
  reload: () => Promise<void>;
  target: Target | null;
  setTarget: (target: Target) => void;
}

export const useShell = () => useOutletContext<ShellContext>();
