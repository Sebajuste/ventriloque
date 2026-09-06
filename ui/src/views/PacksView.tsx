// Les paquets : des données qui arrivent par un zip.
//
// C'est ce qui remplace un installateur. L'exécutable se copie et se lance ; tout ce qui pèse —
// les modèles, les voix — s'ajoute ensuite depuis cette page.
//
// UN PAQUET PEUT AUSSI N'APPORTER QU'UNE RECETTE : le savoir d'où sont les voix dans un jeu
// installé, sans en porter une seule. Il arrive alors « à fabriquer », et c'est le bouton de sa
// ligne — jamais l'installation — qui fait tourner son script, hors de l'application.

import { useState } from "react";

import { buildPack, installPack, uninstallPack, type Snapshot } from "../ipc";
import { PackEntry } from "./packs/PackEntry";
import { PackJobProgress, jobLabel } from "./packs/PackJobProgress";
import { usePackJob } from "./packs/usePackJob";

interface Props {
  snapshot: Snapshot;
  reload: () => Promise<void>;
}

export default function PacksView({ snapshot, reload }: Props) {
  const job = usePackJob(reload);
  // Le paquet dont le retrait est armé.
  const [armed, setArmed] = useState("");

  const run = (what: string, gesture: () => Promise<string>) => {
    setArmed("");
    void job.run(what, gesture);
  };

  // Le compte rendu d'une installation tient sur une ligne, celui d'une fabrication en fait
  // vingt : le premier va dans la barre, le second sous la liste.
  const oneLine = !job.report.includes("\n");
  const label = jobLabel(job.busy, job.progress);

  return (
    <main>
      <section className="form">
        <div className="bar">
          <button
            type="button"
            className="primary"
            disabled={job.busy !== ""}
            onClick={() => run("installation", installPack)}
          >
            Installer un paquet…
          </button>
          <span className={job.failed ? "message failed" : "message"}>
            {job.busy === "" ? (oneLine ? job.report : "") : label}
          </span>
        </div>

        {job.busy !== "" && <PackJobProgress progress={job.progress} label={label} />}

        <p className="note">
          Un paquet est un zip qui porte un <code>pack.json</code> et des dossiers{" "}
          <code>voices/</code>, <code>characters/</code>, <code>models/</code> ou{" "}
          <code>recipes/</code>. Rien d'autre n'est extrait : une entrée qui vise ailleurs est
          ignorée.
        </p>
        <p className="note">
          Le paquet <strong>modèles</strong> est celui qui fait parler l'application. Sans lui,
          Ventriloque s'ouvre mais reste muet — c'est l'état normal d'une première installation.
        </p>

        <ul className="packs">
          {snapshot.packs.length === 0 && <li className="note">Aucun paquet installé.</li>}
          {snapshot.packs.map((pack) => (
            <PackEntry
              key={pack.name}
              pack={pack}
              busy={job.busy !== ""}
              armed={armed === pack.name}
              onArm={() => setArmed(pack.name)}
              onBuild={(chooseFolder) =>
                run("fabrication", () => buildPack(pack.name, chooseFolder))
              }
              onUninstall={() => run("retrait", () => uninstallPack(pack.name))}
            />
          ))}
        </ul>

        {job.busy === "" && job.report !== "" && !oneLine && (
          <pre className={job.failed ? "log failed" : "log"}>{job.report}</pre>
        )}
      </section>
    </main>
  );
}
