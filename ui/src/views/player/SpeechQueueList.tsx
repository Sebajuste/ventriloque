// Ce qui a été dit, ce qui parle et ce qui attend. Chaque réplique est rendue par
// `SpeechQueueItem` ; la liste ne fait que les ranger.
//
// LA PLACE EST RÉSERVÉE, PLEINE OU VIDE. Le bloc de lecture avait la hauteur de son contenu :
// chaque réplique ajoutée poussait le transport vers le haut, et on visait un bouton qui venait
// de bouger — en pleine partie, exactement ce qu'il ne faut pas. Ici la zone garde sa hauteur, et
// c'est la zone de texte qui absorbe la différence.

import type { SpeechQueue } from "../../speech/useSpeechQueue";
import { SpeechQueueItem } from "./SpeechQueueItem";

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

      {queue.lines.length === 0 ? (
        <p className="empty">Rien en file. Ctrl+Entrée pour parler, Échap pour couper.</p>
      ) : (
        <ul className="queue">
          {queue.lines.map((line) => (
            <SpeechQueueItem
              key={line.id}
              line={line}
              isHead={line.id === queue.head?.id}
              queue={queue}
            />
          ))}
        </ul>
      )}
    </div>
  );
}
