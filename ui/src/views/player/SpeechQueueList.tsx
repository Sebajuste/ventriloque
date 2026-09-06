// Ce qui parle et ce qui attend, avec de quoi redire et retirer.
//
// LA PLACE EST RÉSERVÉE, PLEINE OU VIDE. Le bloc de lecture avait la hauteur de son contenu :
// chaque réplique ajoutée poussait le transport vers le haut, et on visait un bouton qui venait
// de bouger — en pleine partie, exactement ce qu'il ne faut pas. Ici la zone garde sa hauteur, et
// c'est la zone de texte qui absorbe la différence.

import { RotateCcw, X } from "lucide-react";

import { ProgressBar, clock } from "../../components/ProgressBar";
import type { SpeechQueue } from "../../speech/useSpeechQueue";

interface Props {
  queue: SpeechQueue;
  failure: string;
}

export function SpeechQueueList({ queue, failure }: Props) {
  return (
    <div className="playback">
      {failure !== "" && (
        <p className="message failed" role="alert">
          {failure}
        </p>
      )}

      {queue.queue.length === 0 ? (
        <p className="empty">Rien en file. Ctrl+Entrée pour parler, Échap pour couper.</p>
      ) : (
        <ul className="queue">
          {queue.queue.map((line, rank) => (
            <li key={line.id} className={rank === 0 ? "head" : undefined}>
              <span className="bullet" aria-hidden="true" />
              <span className="body">
                <span className="text">{line.text}</span>
                <span className="status">{status(queue, rank, line.speaker)}</span>
                {rank === 0 && queue.state !== "preparing" && (
                  <ProgressBar progress={queue.progress} />
                )}
              </span>
              <span className="actions">
                <button
                  type="button"
                  className="icon"
                  aria-label="Redire"
                  title="Redire cette réplique"
                  onClick={() => queue.repeat(line.id)}
                >
                  <RotateCcw size={14} />
                </button>
                {rank > 0 && (
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
          ))}
        </ul>
      )}
    </div>
  );
}

/** Ce que dit la ligne d'état, sous la réplique. Seule la tête a un état à elle. */
function status(queue: SpeechQueue, rank: number, speaker: string): string {
  if (rank > 0) return `en attente — ${speaker}`;
  if (queue.state === "paused") return `en pause — ${speaker}`;
  if (queue.state === "preparing") return `prépare la voix… — ${speaker}`;

  const elapsed = clock(queue.progress.position);
  const total = queue.progress.complete ? ` / ${clock(queue.progress.duration)}` : "";
  return `${elapsed}${total} — ${speaker}`;
}
