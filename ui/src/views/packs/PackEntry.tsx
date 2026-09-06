// Un paquet dans la liste : ce qu'il apporte, et les trois gestes qu'on peut lui faire.
//
// LE RETRAIT S'ARME AVANT DE PARTIR. Un premier clic arme, un second efface : une boîte de
// dialogue de plus se clique sans la lire, alors qu'un bouton qui change de texte se voit.

import type { Manifest } from "../../ipc";

interface Props {
  pack: Manifest;
  busy: boolean;
  armed: boolean;
  onArm: () => void;
  onBuild: (chooseFolder: boolean) => void;
  onUninstall: () => void;
}

const fileCount = (n: number) => `${n} fichier${n > 1 ? "s" : ""}`;

export function PackEntry({ pack, busy, armed, onArm, onBuild, onUninstall }: Props) {
  const needsBuild = pack.recipe !== "" && pack.built_on === "";
  const carriesModels = pack.files.some((f) => f.startsWith("models/"));

  const detail = [
    pack.description,
    pack.author,
    pack.files.length > 0 ? fileCount(pack.files.length) : "",
    pack.installed_on,
  ]
    .filter((text) => text !== "")
    .join(" · ");

  return (
    <li>
      <strong>{pack.version === "" ? pack.name : `${pack.name} ${pack.version}`}</strong>
      <div className="note">{detail}</div>

      <div className="bar">
        {pack.recipe !== "" && (
          <>
            <button type="button" disabled={busy} onClick={() => onBuild(false)}>
              {needsBuild ? "Fabriquer…" : "Refabriquer…"}
            </button>
            {/* La sortie de secours : quand la détection ne trouve rien, ou se trompe. */}
            <button type="button" disabled={busy} onClick={() => onBuild(true)}>
              Autre dossier…
            </button>
          </>
        )}

        <button
          type="button"
          className={armed ? "danger" : undefined}
          disabled={busy}
          onClick={() => (armed ? onUninstall() : onArm())}
        >
          {armed ? "Confirmer le retrait" : "Retirer…"}
        </button>

        <span className="note">
          {armed
            ? carriesModels
              ? "ceci retire les modèles : Ventriloque redeviendra muet"
              : `${fileCount(pack.files.length)} seront effacés`
            : pack.recipe === ""
              ? ""
              : needsBuild
                ? `à fabriquer depuis une copie de ${pack.game === "" ? "votre jeu" : pack.game}`
                : `fabriqué le ${pack.built_on}`}
        </span>
      </div>
    </li>
  );
}
