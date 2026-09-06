// La fenêtre : un en-tête, et la page en cours dessous.
//
// CE FICHIER NE CHOISIT PAS LA VUE, IL LA REÇOIT. Les quatre pages sont des routes déclarées
// dans `routes.tsx` ; il ne reste ici que ce qui les entoure et ce qu'elles se partagent.
//
// LES VUES NE SE CACHENT PAS, ELLES N'EXISTENT PAS. La version d'avant le routeur posait
// `hidden` sur chaque vue, et une règle `main { display: flex }` l'emportait sur le
// `display: none` de l'attribut — les quatre pages s'empilaient donc sur une seule. Le routeur
// ne monte que la route courante : il n'y a plus de règle capable de ramener les autres.
//
// L'ÉTAT EST RELU, PAS TENU. Voix, fiches et paquets vivent sur le disque, et on peut les y
// déposer à la main. Chaque commande qui les modifie déclenche une relecture ; garder un miroir
// local aurait fait diverger la fenêtre du dossier.

import { NavLink, Outlet } from "react-router";

import { selectDevice } from "../ipc";
import { useAppState } from "./useAppState";
import type { ShellContext } from "./context";

const TABS = [
  { path: "/", label: "Player" },
  { path: "/fiches", label: "Fiches" },
  { path: "/atelier", label: "Atelier" },
  { path: "/packs", label: "Packs" },
] as const;

export default function AppShell() {
  const { snapshot, setSnapshot, reload, target, setTarget } = useAppState();

  return (
    <>
      <header>
        <h1>Ventriloque</h1>
        <nav>
          {TABS.map((tab) => (
            <NavLink
              key={tab.path}
              to={tab.path}
              // `end` ne change rien aujourd'hui — react-router exige déjà que le caractère
              // suivant soit un `/`, donc « / » n'est pas actif sur « /packs ». Il est là pour
              // le jour où une page prendra des sous-routes : « /fiches » resterait alors
              // marqué sur « /fiches/judy », ce qui est rarement ce qu'on veut d'un onglet.
              end
              className={({ isActive }) => (isActive ? "tab active" : "tab")}
            >
              {tab.label}
            </NavLink>
          ))}
        </nav>
        <label className="output">
          Sortie
          <select
            value={snapshot.device}
            onChange={(e) => {
              const index = Number(e.target.value);
              setSnapshot((s) => ({ ...s, device: index }));
              void selectDevice(index);
            }}
          >
            {snapshot.devices.map((name, i) => (
              <option key={name} value={i}>
                {name}
              </option>
            ))}
          </select>
        </label>
      </header>

      {snapshot.failure !== "" && (
        <p className="failure">
          Le moteur de parole n'a pas démarré : {snapshot.failure}
          {" — installe le paquet des modèles dans l'onglet Packs."}
        </p>
      )}

      <Outlet context={{ snapshot, reload, target, setTarget } satisfies ShellContext} />
    </>
  );
}
