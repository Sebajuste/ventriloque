// Une réplique de la file : son texte, où elle en est, et ce qu'on peut en faire.
//
// UNE LIGNE NE PARLE QUE D'ELLE-MÊME. Chaque bouton agit sur la réplique que cette ligne
// affiche, et sur aucune autre.

import { useEffect, useRef } from "react";
import { RotateCcw, Sparkles, X } from "lucide-react";

import { ProgressBar, clock } from "../../components/ProgressBar";
import type { Line, SpeechQueue } from "../../speech/useSpeechQueue";

interface Props {
  line: Line;
  isHead: boolean;
  queue: SpeechQueue;
}

export function SpeechQueueItem({ line, isHead, queue }: Props) {
  const row = useRef<HTMLLIElement>(null);

  // LA LISTE SUIT CE QUI PARLE : sans ce suivi, la ligne en cours finirait sous le bord de la
  // zone dès que l'historique s'allonge. `?.` sur la méthode : jsdom ne l'implémente pas.
  useEffect(() => {
    if (isHead) row.current?.scrollIntoView?.({ block: "nearest" });
  }, [isHead]);

  const finished = line.status !== "waiting";

  return (
    <li
      ref={row}
      className={isHead ? "head" : line.status === "waiting" ? undefined : line.status}
    >
      <span className="bullet" aria-hidden="true" />
      <span className="body">
        <span className="text">{line.text}</span>
        <span className="status">{status(queue, line, isHead)}</span>
        {isHead && queue.state !== "preparing" && <ProgressBar progress={queue.progress} />}
      </span>
      <span className="actions">
        {/* Seulement une fois dite ou coupée : ce qui attend va déjà être dit. */}
        {finished && (
          <>
            <button
              type="button"
              className="icon"
              aria-label="Redire"
              title="Redire cette réplique, à l'identique"
              onClick={() => queue.repeat(line.id)}
            >
              <RotateCcw size={14} />
            </button>
            <button
              type="button"
              className="icon"
              aria-label="Nouvelle prise"
              title="Redire cette réplique avec une autre intonation"
              onClick={() => queue.retake(line.id)}
            >
              <Sparkles size={14} />
            </button>
          </>
        )}
        {!isHead && (
          <button
            type="button"
            className="icon"
            aria-label="Retirer"
            title="Retirer de la file"
            onClick={() => queue.remove(line.id)}
          >
            <X size={14} />
          </button>
        )}
      </span>
    </li>
  );
}

/** Ce que dit la ligne d'état, sous la réplique. Seule la tête a un avancement à elle. */
function status(queue: SpeechQueue, line: Line, isHead: boolean): string {
  if (line.status === "said") return `dite — ${line.speaker}`;
  if (line.status === "cut") return `coupée — ${line.speaker}`;
  if (!isHead) return `en attente — ${line.speaker}`;
  if (queue.state === "paused") return `en pause — ${line.speaker}`;
  if (queue.state === "preparing") return `prépare la voix… — ${line.speaker}`;

  const elapsed = clock(queue.progress.position);
  const total = queue.progress.complete ? ` / ${clock(queue.progress.duration)}` : "";
  return `${elapsed}${total} — ${line.speaker}`;
}
