// Les paquets : des données qui arrivent par un zip.
//
// C'est ce qui remplace un installateur. L'exécutable se copie et se lance ; tout ce qui pèse —
// les modèles, les voix — s'ajoute ensuite depuis cette page.

import { useState } from "react";
import { enClair, installerPack, type Etat } from "../api";

interface Props {
  etat: Etat;
  relire: () => Promise<void>;
}

export default function Packs({ etat, relire }: Props) {
  const [dit, setDit] = useState("");
  const [rate, setRate] = useState(false);
  const [occupe, setOccupe] = useState(false);

  const installer = async () => {
    setOccupe(true);
    setDit("");
    setRate(false);
    try {
      const compte = await installerPack();
      setDit(compte === "" ? "annulé" : compte);
      if (compte !== "") await relire();
    } catch (e) {
      setDit(enClair(e));
      setRate(true);
    } finally {
      setOccupe(false);
    }
  };

  return (
    <main>
      <section className="formulaire">
        <div className="barre">
          <button type="button" className="fort" disabled={occupe} onClick={() => void installer()}>
            Installer un paquet…
          </button>
          <span className={rate ? "dit rate" : "dit"}>{dit}</span>
        </div>

        <p className="note">
          Un paquet est un zip qui porte un <code>pack.json</code> et des dossiers{" "}
          <code>voix/</code>, <code>pnj/</code> ou <code>modeles/</code>. Rien d'autre n'est
          extrait : une entrée qui vise ailleurs est ignorée.
        </p>
        <p className="note">
          Le paquet <strong>modèles</strong> est celui qui fait parler l'application. Sans lui,
          Ventriloque s'ouvre mais reste muet — c'est l'état normal d'une première installation.
        </p>

        <ul className="packs">
          {etat.packs.length === 0 && <li className="note">Aucun paquet installé.</li>}
          {etat.packs.map((p) => (
            <li key={p.nom}>
              <strong>{p.version === "" ? p.nom : `${p.nom} ${p.version}`}</strong>
              <div className="note">
                {[
                  p.description,
                  p.auteur,
                  `${p.fichiers.length} fichier${p.fichiers.length > 1 ? "s" : ""}`,
                  p.installe_le,
                ]
                  .filter((t) => t !== "")
                  .join(" · ")}
              </div>
            </li>
          ))}
        </ul>
      </section>
    </main>
  );
}
