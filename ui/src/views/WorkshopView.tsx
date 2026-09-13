// L'atelier : des sons d'un personnage en entrée, une voix nommée en sortie.
//
// Volontairement pauvre en réglages. Cloner une voix depuis des fichiers, ou se servir du
// catalogue livré avec les modèles, couvre le besoin d'une table ; tout ce qu'on ajouterait ici
// serait pris sur le temps de jouer. Les deux curseurs restent repliés pour cette raison.

import { useState } from "react";

import { NORMAL_PACE, asMessage, forgeVoice, pickAudioFiles, type Target } from "../ipc";

interface Props {
  reload: () => Promise<void>;
  setTarget: (target: Target) => void;
}

export default function WorkshopView({ reload, setTarget }: Props) {
  const [name, setName] = useState("");
  const [files, setFiles] = useState<string[]>([]);
  const [pitch, setPitch] = useState(0);
  const [formants, setFormants] = useState(0);
  const [message, setMessage] = useState("");
  const [failed, setFailed] = useState(false);
  const [busy, setBusy] = useState(false);

  const announce = (what: string, bad = false) => {
    setMessage(what);
    setFailed(bad);
  };

  const choose = async () => {
    // Le sélecteur vit côté Rust : un `<input type="file">` ne donnerait pas les chemins réels,
    // dont l'atelier a besoin pour aller lire les sons.
    const picked = await pickAudioFiles();
    if (picked.length === 0) return;
    setFiles(picked);
    announce(`${picked.length} fichier${picked.length > 1 ? "s" : ""}`);
  };

  const forge = async () => {
    if (name.trim() === "") return announce("il faut nommer la voix", true);
    if (files.length === 0) return announce("il faut au moins un fichier son", true);

    setBusy(true);
    announce("assemblage et décalage…");
    try {
      const made = await forgeVoice(name, files, pitch, formants);
      await reload();
      announce(`${made} — la voix est prête, elle est dans le player`);
      setTarget({
        name: made.replace(/\.wav$/, ""),
        reference: made,
        kind: "clone",
        lines: [],
        character: "",
        pace: NORMAL_PACE,
      });
    } catch (e) {
      announce(asMessage(e), true);
    } finally {
      setBusy(false);
    }
  };

  return (
    <main>
      <section className="form">
        <label>
          Nom de la voix
          <input value={name} placeholder="barman" onChange={(e) => setName(e.target.value)} />
        </label>

        <label>
          Sons du personnage
          <button type="button" onClick={() => void choose()}>
            Choisir des fichiers…
          </button>
        </label>

        <ul className="picked">
          {files.map((file) => (
            <li key={file}>{file}</li>
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
          <Slider label="Hauteur" value={pitch} onChange={setPitch} />
          <Slider label="Formants" value={formants} onChange={setFormants} />
        </details>

        <div className="bar">
          <button type="button" className="primary" disabled={busy} onClick={() => void forge()}>
            Fabriquer la voix
          </button>
          <span className={failed ? "message failed" : "message"}>{message}</span>
        </div>
      </section>
    </main>
  );
}

/** Huit demi-tons de part et d'autre : au-delà, la voix cesse d'être crédible. */
const RANGE = 8;

function Slider(props: { label: string; value: number; onChange: (n: number) => void }) {
  return (
    <label className="slider">
      {props.label}
      <input
        type="range"
        min={-RANGE}
        max={RANGE}
        step={0.5}
        value={props.value}
        onChange={(e) => props.onChange(Number(e.target.value))}
      />
      <output>{props.value}</output> demi-tons
    </label>
  );
}
