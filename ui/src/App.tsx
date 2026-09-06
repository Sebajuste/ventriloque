// La fenêtre : un en-tête, et la page en cours dessous.
//
// CE FICHIER NE CHOISIT PLUS LA VUE, IL LA REÇOIT. Les quatre pages sont des routes déclarées
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

import { useCallback, useEffect, useState } from "react";
import { NavLink, Outlet, useOutletContext } from "react-router";
import { lireEtat, choisirPeripherique, type Cible, type Etat } from "./api";

const ONGLETS = [
  { chemin: "/", nom: "Player" },
  { chemin: "/fiches", nom: "Fiches" },
  { chemin: "/atelier", nom: "Atelier" },
  { chemin: "/packs", nom: "Packs" },
] as const;

const VIDE: Etat = {
  pret: false,
  panne: "",
  racine: "",
  modeles: "",
  voix: [],
  fiches: [],
  packs: [],
  peripheriques: [],
  peripherique: 0,
};

/**
 * Ce que la fenêtre prête aux pages.
 *
 * `choisie` TRAVERSE LES ROUTES : l'atelier la pose en sortant de la forge, le player la lit.
 * C'est ce qui fait de « forger, entendre, corriger » un geste et pas une navigation, et c'est
 * la raison pour laquelle cet état vit ici plutôt que dans une page.
 */
export interface Contexte {
  etat: Etat;
  relire: () => Promise<void>;
  choisie: Cible | null;
  setChoisie: (c: Cible) => void;
}

export const useFenetre = () => useOutletContext<Contexte>();

export default function Fenetre() {
  const [etat, setEtat] = useState<Etat>(VIDE);
  const [choisie, setChoisie] = useState<Cible | null>(null);

  const relire = useCallback(async () => {
    setEtat(await lireEtat());
  }, []);

  useEffect(() => {
    void relire();
  }, [relire]);

  return (
    <>
      <header>
        <h1>Ventriloque</h1>
        <nav>
          {ONGLETS.map((o) => (
            <NavLink
              key={o.chemin}
              to={o.chemin}
              // `end` ne change rien aujourd'hui — react-router exige déjà que le caractère
              // suivant soit un `/`, donc « / » n'est pas actif sur « /packs ». Il est là pour
              // le jour où une page prendra des sous-routes : « /fiches » resterait alors
              // marqué sur « /fiches/judy », ce qui est rarement ce qu'on veut d'un onglet.
              end
              className={({ isActive }) => (isActive ? "onglet actif" : "onglet")}
            >
              {o.nom}
            </NavLink>
          ))}
        </nav>
        <label className="sortie">
          Sortie
          <select
            value={etat.peripherique}
            onChange={(e) => {
              const rang = Number(e.target.value);
              setEtat((v) => ({ ...v, peripherique: rang }));
              void choisirPeripherique(rang);
            }}
          >
            {etat.peripheriques.map((nom, i) => (
              <option key={nom} value={i}>
                {nom}
              </option>
            ))}
          </select>
        </label>
      </header>

      {etat.panne !== "" && (
        <p className="panne">
          Le moteur de parole n'a pas démarré : {etat.panne}
          {" — installe le paquet des modèles dans l'onglet Packs."}
        </p>
      )}

      <Outlet context={{ etat, relire, choisie, setChoisie } satisfies Contexte} />
    </>
  );
}
