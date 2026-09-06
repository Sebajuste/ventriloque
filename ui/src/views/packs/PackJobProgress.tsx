// La jauge et le journal d'une opération en cours.
//
// UNE PROPORTION N'A DE SENS QUE SI LE TOTAL EST CONNU. Une fabrication ne le connaît pas : sa
// jauge se contente d'aller et venir, et c'est la ligne d'étape qui informe.

import { useEffect, useRef } from "react";

import type { PackProgress } from "../../ipc";

interface Props {
  progress: PackProgress;
  label: string;
}

export function PackJobProgress({ progress, label }: Props) {
  const known = progress.total > 0;
  const share = known ? Math.round((progress.done / progress.total) * 100) : 0;
  const log = useRef<HTMLPreElement>(null);

  // Un journal qui grandit doit montrer sa DERNIÈRE ligne : c'est celle qui dit où l'on en est.
  // Sans cela, la fenêtre resterait sur « stockage ouvert » pendant trois minutes.
  useEffect(() => {
    const pre = log.current;
    if (pre) pre.scrollTop = pre.scrollHeight;
  }, [progress.log.length]);

  return (
    <>
      <div className="progress">
        <div
          className="gauge"
          role="progressbar"
          aria-valuenow={known ? progress.done : undefined}
          aria-valuemax={known ? progress.total : undefined}
          aria-label={label}
        >
          <div
            className={known ? "filled" : "filled unknown"}
            style={known ? { width: `${share}%` } : undefined}
          />
        </div>
        {progress.step !== "" && <div className="note detail">{progress.step}</div>}
      </div>

      {/* Le journal en direct pendant l'opération, le compte rendu une fois finie. */}
      {progress.log.length > 0 && (
        <pre className="log" ref={log}>
          {progress.log.join("\n")}
        </pre>
      )}
    </>
  );
}

/** Ce que la barre annonce, selon le geste en cours et ce qu'on sait de son avancement. */
export function jobLabel(busy: string, progress: PackProgress): string {
  if (busy === "fabrication") return "fabrication en cours, quelques minutes…";
  if (busy === "retrait") return "retrait en cours…";
  if (progress.total > 0) {
    return `installation — ${Math.round((progress.done / progress.total) * 100)} %`;
  }
  return "installation en cours…";
}
