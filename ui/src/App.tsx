// La fenêtre : un en-tête, un onglet à la fois.
//
// LES VUES NE SE CACHENT PAS, ELLES N'EXISTENT PAS. La version précédente posait `hidden` sur
// chaque vue, et une règle `main { display: flex }` l'emportait sur le `display: none` de
// l'attribut — les quatre pages s'empilaient donc sur une seule. Ici l'onglet inactif n'est pas
// rendu du tout : il n'y a plus de règle capable de le ramener.
//
// L'ÉTAT EST RELU, PAS TENU. Voix, fiches et paquets vivent sur le disque, et on peut les y
// déposer à la main. Chaque commande qui les modifie déclenche une relecture ; garder un miroir
// local aurait fait diverger la fenêtre du dossier.

import { useCallback, useEffect, useState } from "react";
import { lireEtat, choisirPeripherique, type Cible, type Etat } from "./api";
import Player from "./vues/Player";
import Fiches from "./vues/Fiches";
import Atelier from "./vues/Atelier";
import Packs from "./vues/Packs";

const ONGLETS = [
  { cle: "player", nom: "Player" },
  { cle: "fiches", nom: "Fiches" },
  { cle: "atelier", nom: "Atelier" },
  { cle: "packs", nom: "Packs" },
] as const;

type Onglet = (typeof ONGLETS)[number]["cle"];

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

export default function App() {
  const [etat, setEtat] = useState<Etat>(VIDE);
  const [onglet, setOnglet] = useState<Onglet>("player");
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
            <button
              key={o.cle}
              type="button"
              className={o.cle === onglet ? "onglet actif" : "onglet"}
              onClick={() => setOnglet(o.cle)}
            >
              {o.nom}
            </button>
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

      {onglet === "player" && (
        <Player etat={etat} choisie={choisie} setChoisie={setChoisie} />
      )}
      {onglet === "fiches" && <Fiches etat={etat} relire={relire} />}
      {onglet === "atelier" && <Atelier relire={relire} setChoisie={setChoisie} />}
      {onglet === "packs" && <Packs etat={etat} relire={relire} />}
    </>
  );
}
