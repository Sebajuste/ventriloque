// La fenêtre : un en-tête, une barre latérale, et la page en cours à leur droite.
//
// CE FICHIER NE CHOISIT PAS LA VUE, IL LA REÇOIT. Les pages sont des routes déclarées
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

import { Hammer, MicVocal, Package, SlidersHorizontal, Users } from "lucide-react";
import { NavLink, Outlet } from "react-router";

import { selectDevice } from "../ipc";
import { useAppState } from "./useAppState";
import type { ShellContext } from "./context";

// La barre laterale. L'icone donne le repere, le mot le confirme : en seance on vise la ligne
// d'un coup d'oeil, mais « Atelier » et « Packs » ne se distinguent pas par un pictogramme seul.
const TABS = [
  { path: "/", label: "Player", Icon: MicVocal },
  { path: "/fiches", label: "Fiches", Icon: Users },
  { path: "/atelier", label: "Atelier", Icon: Hammer },
  { path: "/packs", label: "Packs", Icon: Package },
  { path: "/reglages", label: "Réglages", Icon: SlidersHorizontal },
] as const;

export default function AppShell() {
  const { snapshot, setSnapshot, reload, target, setTarget } = useAppState();

  return (
    <>
      <header>
        <h1>Ventriloque</h1>
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

      <div className="workspace">
        <nav className="rail">
          {TABS.map(({ path, label, Icon }) => (
            <NavLink
              key={path}
              to={path}
              // `end` ne change rien aujourd'hui — react-router exige déjà que le caractère
              // suivant soit un `/`, donc « / » n'est pas actif sur « /packs ». Il est là pour
              // le jour où une page prendra des sous-routes : « /fiches » resterait alors
              // marqué sur « /fiches/judy », ce qui est rarement ce qu'on veut d'un onglet.
              end
              className={({ isActive }) => (isActive ? "tab active" : "tab")}
            >
              <Icon size={17} aria-hidden />
              {label}
            </NavLink>
          ))}
        </nav>

        <Outlet context={{ snapshot, reload, target, setTarget } satisfies ShellContext} />
      </div>
    </>
  );
}
