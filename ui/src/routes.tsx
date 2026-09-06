// Les quatre pages, et rien d'autre.
//
// LES VUES NE CONNAISSENT PAS LE ROUTEUR. Chacune reçoit encore exactement les props qu'elle
// recevait avant, et ce sont les quatre adaptateurs ci-dessous qui vont les chercher dans le
// contexte de la fenêtre. Le détour coûte vingt lignes et rend deux choses : une vue se monte
// seule dans un test, sans routeur autour, et remplacer le routeur un jour ne touchera pas une
// ligne des pages.
//
// PAS DE `Router` ICI. `main.tsx` l'enveloppe dans un `HashRouter` — celui qui survit au
// protocole de Tauri — et les tests dans un `MemoryRouter`, qui part d'où ils veulent et ne
// laisse pas d'adresse derrière lui d'un test à l'autre.

import { Navigate, Route, Routes } from "react-router";

import Fenetre, { useFenetre } from "./App";
import Player from "./vues/Player";
import Fiches from "./vues/Fiches";
import Atelier from "./vues/Atelier";
import Packs from "./vues/Packs";

function PagePlayer() {
  const { etat, choisie, setChoisie } = useFenetre();
  return <Player etat={etat} choisie={choisie} setChoisie={setChoisie} />;
}

function PageFiches() {
  const { etat, relire } = useFenetre();
  return <Fiches etat={etat} relire={relire} />;
}

function PageAtelier() {
  const { relire, setChoisie } = useFenetre();
  return <Atelier relire={relire} setChoisie={setChoisie} />;
}

function PagePacks() {
  const { etat, relire } = useFenetre();
  return <Packs etat={etat} relire={relire} />;
}

export function Routage() {
  return (
    <Routes>
      <Route element={<Fenetre />}>
        <Route index element={<PagePlayer />} />
        <Route path="fiches" element={<PageFiches />} />
        <Route path="atelier" element={<PageAtelier />} />
        <Route path="packs" element={<PagePacks />} />
        {/* Une adresse inconnue ramène au player plutôt que de laisser une fenêtre vide. En
            seance, un `#/` mal formé ne doit pas ressembler à une application qui a planté. */}
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
