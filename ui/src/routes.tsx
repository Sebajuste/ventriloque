// Les cinq pages, et rien d'autre.
//
// LES VUES NE CONNAISSENT PAS LE ROUTEUR. Chacune recoit exactement les props dont elle a besoin,
// et ce sont les adaptateurs ci-dessous qui vont les chercher dans le contexte de la
// fenetre. Le detour coute vingt lignes et rend deux choses : une vue se monte seule dans un
// test, sans routeur autour, et remplacer le routeur un jour ne touchera pas une ligne des pages.
//
// PAS DE `Router` ICI. `main.tsx` l'enveloppe dans un `HashRouter` -- celui qui survit au
// protocole de Tauri -- et les tests dans un `MemoryRouter`, qui part d'ou ils veulent et ne
// laisse pas d'adresse derriere lui d'un test a l'autre.

import { Navigate, Route, Routes } from "react-router";

import AppShell from "./shell/AppShell";
import { useShell } from "./shell/context";
import CharactersView from "./views/CharactersView";
import PacksView from "./views/PacksView";
import PlayerView from "./views/PlayerView";
import SettingsView from "./views/SettingsView";
import WorkshopView from "./views/WorkshopView";

function PlayerPage() {
  const { snapshot, reload, target, setTarget } = useShell();
  return <PlayerView snapshot={snapshot} reload={reload} target={target} setTarget={setTarget} />;
}

function CharactersPage() {
  const { snapshot, reload } = useShell();
  return <CharactersView snapshot={snapshot} reload={reload} />;
}

function WorkshopPage() {
  const { reload, setTarget } = useShell();
  return <WorkshopView reload={reload} setTarget={setTarget} />;
}

function PacksPage() {
  const { snapshot, reload } = useShell();
  return <PacksView snapshot={snapshot} reload={reload} />;
}

function SettingsPage() {
  const { reload } = useShell();
  return <SettingsView reload={reload} />;
}

export function AppRoutes() {
  return (
    <Routes>
      <Route element={<AppShell />}>
        <Route index element={<PlayerPage />} />
        <Route path="fiches" element={<CharactersPage />} />
        <Route path="atelier" element={<WorkshopPage />} />
        <Route path="packs" element={<PacksPage />} />
        <Route path="reglages" element={<SettingsPage />} />
        {/* Une adresse inconnue ramene au player plutot que de laisser une fenetre vide. En
            seance, un `#/` mal forme ne doit pas ressembler a une application qui a plante. */}
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  );
}
