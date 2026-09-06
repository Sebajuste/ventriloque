// L'etat que la fenetre tient : l'instantane du disque, et qui parle en ce moment.
//
// RELU PLUTOT QUE TENU. Voix, fiches et paquets vivent sur le disque, et on peut les y deposer a
// la main : chaque commande qui les modifie appelle `reload`. Un miroir local aurait diverge du
// dossier des la premiere copie de fichier faite en dehors de l'application.

import { useCallback, useEffect, useState } from "react";

import { readSnapshot, type Snapshot, type Target } from "../ipc";

/** Ce qu'affiche la fenetre avant que Rust ait repondu. Muette, mais pas cassee. */
export const EMPTY_SNAPSHOT: Snapshot = {
  ready: false,
  failure: "",
  root: "",
  models: "",
  voices: [],
  characters: [],
  packs: [],
  devices: [],
  device: 0,
};

export function useAppState() {
  const [snapshot, setSnapshot] = useState<Snapshot>(EMPTY_SNAPSHOT);
  const [target, setTarget] = useState<Target | null>(null);

  const reload = useCallback(async () => {
    setSnapshot(await readSnapshot());
  }, []);

  useEffect(() => {
    void reload();
  }, [reload]);

  return { snapshot, setSnapshot, reload, target, setTarget };
}
