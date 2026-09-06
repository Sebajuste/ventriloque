// Le player : la vue de séance.
//
// Deux groupes dans la colonne de gauche — les personnages d'abord, parce que c'est avec eux
// qu'on joue, puis les voix brutes, utiles pour une silhouette qui n'a pas mérité de fiche.
//
// Échap coupe la voix D'OÙ QUE L'ON SOIT dans la fenêtre. En pleine partie on ne cherche pas le
// bon champ avant de faire taire un PNJ.

import { useEffect, useRef, useState } from "react";
import {
  enClair,
  parler,
  prechauffer,
  taire,
  type Cible,
  type Etat,
  type Voix,
} from "../api";

interface Props {
  etat: Etat;
  choisie: Cible | null;
  setChoisie: (c: Cible) => void;
}

const depuisVoix = (v: Voix): Cible => ({
  nom: v.nom,
  reference: v.reference,
  palier: v.palier,
  repliques: [],
});

export default function Player({ etat, choisie, setChoisie }: Props) {
  const [filtre, setFiltre] = useState("");
  const [texte, setTexte] = useState("");
  const [dit, setDit] = useState("");
  const [rate, setRate] = useState(false);
  const [chauffe, setChauffe] = useState(false);
  const zone = useRef<HTMLTextAreaElement>(null);

  // Le compte de ce qui reste à ENTENDRE, celle en cours comprise. Une référence plutôt qu'un
  // état : il est lu et écrit depuis des promesses qui se chevauchent, et un état de React
  // rendrait des incréments perdus.
  const enFile = useRef(0);

  useEffect(() => {
    const surTouche = (e: KeyboardEvent) => {
      if (e.key === "Escape") void taire();
    };
    document.addEventListener("keydown", surTouche);
    return () => document.removeEventListener("keydown", surTouche);
  }, []);

  const annoncer = (quoi: string, mauvais = false) => {
    setDit(quoi);
    setRate(mauvais);
  };

  const compter = () => {
    const n = enFile.current;
    annoncer(n <= 0 ? "" : n === 1 ? "…" : `${n} répliques à venir`);
  };

  const dire = async (impose?: string) => {
    if (choisie === null) return annoncer("choisis d'abord un personnage ou une voix", true);
    if (choisie.reference === "") return annoncer("ce personnage n'a pas de voix", true);
    const quoi = (impose ?? texte).trim();
    if (quoi === "") return;

    enFile.current += 1;
    compter();
    try {
      await parler(choisie.reference, quoi);
      enFile.current -= 1;
      compter();
    } catch (e) {
      enFile.current -= 1;
      annoncer(enClair(e), true);
    }
  };

  const garde = (t: string) => filtre === "" || t.toLowerCase().includes(filtre.toLowerCase());
  const fiches = etat.fiches.filter((f) => garde(f.nom) || garde(f.univers));
  const brutes = etat.voix.filter((v) => garde(v.nom));

  const prendre = (c: Cible) => {
    setChoisie(c);
    zone.current?.focus();
  };

  const marqueDe = (c: Cible) =>
    c.reference === ""
      ? "aucune voix"
      : c.palier === "clone"
        ? "voix clonée"
        : c.palier === "catalogue"
          ? "voix de catalogue"
          : "voix introuvable";

  return (
    <main>
      <aside>
        <input
          type="search"
          placeholder="Chercher"
          value={filtre}
          onChange={(e) => setFiltre(e.target.value)}
        />
        <ul className="liste">
          {fiches.length > 0 && <li className="titre">Personnages</li>}
          {fiches.map((f) => {
            const voisine = etat.voix.find((v) => v.reference === f.voix);
            const cible: Cible = {
              nom: f.nom,
              reference: f.voix,
              palier: voisine?.palier ?? null,
              repliques: f.repliques,
            };
            return (
              <Entree
                key={`f-${f.id}`}
                nom={f.nom}
                marque={f.univers || voisine?.nom || "sans voix"}
                actif={choisie?.nom === f.nom && choisie.reference === f.voix}
                onClick={() => prendre(cible)}
              />
            );
          })}

          {brutes.length > 0 && <li className="titre">Voix</li>}
          {brutes.map((v) => (
            <Entree
              key={`v-${v.reference}`}
              nom={v.nom}
              marque={v.palier}
              actif={choisie?.nom === v.nom && choisie.reference === v.reference}
              onClick={() => prendre(depuisVoix(v))}
            />
          ))}
        </ul>

        {/* Payer d'avance le clonage : ~6 s par voix, une seule fois, pendant qu'on installe
            la table. Entendues au moment où un PNJ prend la parole, ces secondes sont un
            silence que personne ne comprend. */}
        <button
          type="button"
          disabled={chauffe || !etat.pret}
          onClick={async () => {
            setChauffe(true);
            annoncer("préchauffage…");
            try {
              annoncer((await prechauffer()).join(" · "));
            } catch (e) {
              annoncer(enClair(e), true);
            } finally {
              setChauffe(false);
            }
          }}
        >
          Préchauffer les voix
        </button>
      </aside>

      <section>
        <div className="qui">
          {choisie === null ? (
            "Choisis un personnage ou une voix"
          ) : (
            <>
              <strong>{choisie.nom}</strong> — {marqueDe(choisie)}
            </>
          )}
        </div>

        {/* Un clic dit la réplique : c'est le geste utile en pleine partie, quand on n'a pas
            le temps de taper. */}
        {choisie !== null && choisie.repliques.length > 0 && (
          <ul className="favorites">
            {choisie.repliques.map((r) => (
              <li key={r} title="Dire cette réplique" onClick={() => void dire(r)}>
                {r}
              </li>
            ))}
          </ul>
        )}

        <textarea
          ref={zone}
          spellCheck={false}
          placeholder={"Ce que dit le PNJ\n\nCtrl+Entrée pour parler, Échap pour couper"}
          value={texte}
          onChange={(e) => setTexte(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
              e.preventDefault();
              void dire();
            }
          }}
        />

        <div className="barre">
          <button type="button" className="fort" onClick={() => void dire()}>
            Parler
          </button>
          <button type="button" onClick={() => void taire()}>
            Silence
          </button>
          <span className={rate ? "dit rate" : "dit"}>{dit}</span>
        </div>
      </section>
    </main>
  );
}

function Entree(props: { nom: string; marque: string; actif: boolean; onClick: () => void }) {
  return (
    <li className={props.actif ? "actif" : undefined} onClick={props.onClick}>
      {props.nom}
      <span className="palier">{props.marque}</span>
    </li>
  );
}
