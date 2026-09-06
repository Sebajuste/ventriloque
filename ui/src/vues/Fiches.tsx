// Les fiches de PNJ : nom, univers, voix, et les répliques qu'on redit souvent.
//
// Rien d'autre. L'histoire du personnage, ses liens, ses secrets sont déjà dans les notes du MJ
// et n'ont pas à être ressaisis ici.

import { useState } from "react";
import { ecrireFiche, enClair, supprimerFiche, type Etat, type Fiche } from "../api";

interface Props {
  etat: Etat;
  relire: () => Promise<void>;
}

const NEUVE: Fiche = { id: "", nom: "", univers: "", voix: "", repliques: [] };

export default function Fiches({ etat, relire }: Props) {
  const [brouillon, setBrouillon] = useState<Fiche>(NEUVE);
  const [dit, setDit] = useState("");
  const [rate, setRate] = useState(false);

  const annoncer = (quoi: string, mauvais = false) => {
    setDit(quoi);
    setRate(mauvais);
  };

  const editer = (f: Fiche) => {
    setBrouillon(f);
    annoncer("");
  };

  const enregistrer = async () => {
    try {
      // L'identifiant d'avant permet de renommer sans laisser un doublon derrière.
      const faite = await ecrireFiche({ ...brouillon, id: "" }, brouillon.id);
      await relire();
      setBrouillon(faite);
      annoncer("enregistrée");
    } catch (e) {
      annoncer(enClair(e), true);
    }
  };

  const supprimer = async () => {
    if (brouillon.id === "") return annoncer("rien à supprimer", true);
    try {
      await supprimerFiche(brouillon.id);
      await relire();
      setBrouillon(NEUVE);
      annoncer("supprimée");
    } catch (e) {
      annoncer(enClair(e), true);
    }
  };

  return (
    <main>
      <aside>
        <ul className="liste">
          {etat.fiches.map((f) => (
            <li
              key={f.id}
              className={f.id === brouillon.id ? "actif" : undefined}
              onClick={() => editer(f)}
            >
              {f.nom}
              {f.univers !== "" && <span className="palier">{f.univers}</span>}
            </li>
          ))}
        </ul>
        <button type="button" onClick={() => editer(NEUVE)}>
          Nouveau personnage
        </button>
      </aside>

      <section className="formulaire">
        <label>
          Nom
          <input
            value={brouillon.nom}
            placeholder="le barman de la Lanterne"
            onChange={(e) => setBrouillon({ ...brouillon, nom: e.target.value })}
          />
        </label>

        <label>
          Univers
          <input
            value={brouillon.univers}
            placeholder="Warhammer"
            onChange={(e) => setBrouillon({ ...brouillon, univers: e.target.value })}
          />
        </label>

        <label>
          Voix
          <select
            value={brouillon.voix}
            onChange={(e) => setBrouillon({ ...brouillon, voix: e.target.value })}
          >
            <option value="">— aucune —</option>
            {etat.voix.map((v) => (
              <option key={v.reference} value={v.reference}>
                {v.nom} ({v.palier})
              </option>
            ))}
          </select>
        </label>

        <label>
          Répliques favorites — une par ligne
          <textarea
            className="petit"
            spellCheck={false}
            placeholder={"Vous êtes pas d'ici, vous.\nÇa fera trois couronnes."}
            value={brouillon.repliques.join("\n")}
            onChange={(e) =>
              setBrouillon({ ...brouillon, repliques: e.target.value.split("\n") })
            }
          />
        </label>

        <p className="note">
          Cliquer une réplique favorite dans le player la fait dire tout de suite. C'est ce qui
          rend l'outil utilisable en pleine partie, quand on n'a pas le temps de taper.
        </p>

        <div className="barre">
          <button type="button" className="fort" onClick={() => void enregistrer()}>
            Enregistrer
          </button>
          <button type="button" onClick={() => void supprimer()}>
            Supprimer
          </button>
          <span className={rate ? "dit rate" : "dit"}>{dit}</span>
        </div>
      </section>
    </main>
  );
}
