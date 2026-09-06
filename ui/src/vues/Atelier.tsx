// L'atelier : des sons d'un personnage en entrée, une voix nommée en sortie.
//
// Volontairement pauvre en réglages. Cloner une voix depuis des fichiers, ou se servir du
// catalogue livré avec les modèles, couvre le besoin d'une table ; tout ce qu'on ajouterait ici
// serait pris sur le temps de jouer. Les deux curseurs restent repliés pour cette raison.

import { useState } from "react";
import { choisirFichiers, enClair, forger, type Cible } from "../api";

interface Props {
  relire: () => Promise<void>;
  setChoisie: (c: Cible) => void;
}

export default function Atelier({ relire, setChoisie }: Props) {
  const [nom, setNom] = useState("");
  const [fichiers, setFichiers] = useState<string[]>([]);
  const [pitch, setPitch] = useState(0);
  const [formants, setFormants] = useState(0);
  const [dit, setDit] = useState("");
  const [rate, setRate] = useState(false);
  const [occupe, setOccupe] = useState(false);

  const annoncer = (quoi: string, mauvais = false) => {
    setDit(quoi);
    setRate(mauvais);
  };

  const choisir = async () => {
    // Le sélecteur vit côté Rust : un `<input type="file">` ne donnerait pas les chemins réels,
    // dont l'atelier a besoin pour aller lire les sons.
    const pris = await choisirFichiers();
    if (pris.length === 0) return;
    setFichiers(pris);
    annoncer(`${pris.length} fichier${pris.length > 1 ? "s" : ""}`);
  };

  const fabriquer = async () => {
    if (nom.trim() === "") return annoncer("il faut nommer la voix", true);
    if (fichiers.length === 0) return annoncer("il faut au moins un fichier son", true);

    setOccupe(true);
    annoncer("assemblage et décalage…");
    try {
      const fait = await forger(nom, fichiers, pitch, formants);
      await relire();
      annoncer(`${fait} — la voix est prête, elle est dans le player`);
      setChoisie({
        nom: fait.replace(/\.wav$/, ""),
        reference: fait,
        palier: "clone",
        repliques: [],
      });
    } catch (e) {
      annoncer(enClair(e), true);
    } finally {
      setOccupe(false);
    }
  };

  return (
    <main>
      <section className="formulaire">
        <label>
          Nom de la voix
          <input value={nom} placeholder="barman" onChange={(e) => setNom(e.target.value)} />
        </label>

        <label>
          Sons du personnage
          <button type="button" onClick={() => void choisir()}>
            Choisir des fichiers…
          </button>
        </label>

        <ul className="choisis">
          {fichiers.map((f) => (
            <li key={f}>{f}</li>
          ))}
        </ul>

        <p className="note">
          Une trentaine de secondes suffit — dix ou vingt répliques courtes valent mieux qu'un
          long monologue. Au-delà, le temps de clonage augmente sans que la voix gagne.
        </p>

        <details>
          <summary>Déplacer la voix</summary>
          <p className="note">
            La hauteur seule garde la personne et la fait parler plus grave. Les formants seuls
            changent le gabarit de la gorge sans toucher à la mélodie — c'est ce qui rend une
            référence méconnaissable.
          </p>
          <Curseur nom="Hauteur" valeur={pitch} setValeur={setPitch} />
          <Curseur nom="Formants" valeur={formants} setValeur={setFormants} />
        </details>

        <div className="barre">
          <button type="button" className="fort" disabled={occupe} onClick={() => void fabriquer()}>
            Fabriquer la voix
          </button>
          <span className={rate ? "dit rate" : "dit"}>{dit}</span>
        </div>
      </section>
    </main>
  );
}

function Curseur(props: { nom: string; valeur: number; setValeur: (n: number) => void }) {
  return (
    <label className="curseur">
      {props.nom}
      <input
        type="range"
        min={-8}
        max={8}
        step={0.5}
        value={props.valeur}
        onChange={(e) => props.setValeur(Number(e.target.value))}
      />
      <output>{props.valeur}</output> demi-tons
    </label>
  );
}
