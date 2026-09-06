// La barre d'avancement d'une réplique.
//
// DEUX BARRES, PAS UNE. Tant que la synthèse tourne, la durée totale n'existe pas : le moteur
// fabrique environ trois fois plus vite qu'on n'écoute, et un pourcentage calculé sur ce qui est
// fabriqué à cet instant reculerait à chaque morceau qui arrive. La barre avance alors sans
// promettre de fin ; elle devient une vraie proportion dès que le total est connu — ce qui
// arrive vite, autour du premier tiers de la réplique.

import type { SpeechProgress } from "../ipc";

export function ProgressBar({ progress }: { progress: SpeechProgress }) {
  const share =
    progress.complete && progress.duration > 0 ? progress.position / progress.duration : 0;
  return (
    <span
      className={progress.complete ? "speech-bar" : "speech-bar unknown"}
      role="progressbar"
      aria-label="Progression de la réplique"
      {...(progress.complete
        ? {
            "aria-valuemin": 0,
            "aria-valuemax": progress.duration,
            "aria-valuenow": progress.position,
          }
        : {})}
    >
      <span style={progress.complete ? { width: `${Math.min(100, share * 100)}%` } : undefined} />
    </span>
  );
}

/** `1:04`, ou `0:07` — les répliques se comptent en secondes, pas en heures. */
export const clock = (ms: number) => {
  const total = Math.floor(ms / 1000);
  return `${Math.floor(total / 60)}:${String(total % 60).padStart(2, "0")}`;
};
