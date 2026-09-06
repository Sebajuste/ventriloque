// Les boutons de transport : parler, suspendre, passer, tout couper.
//
// LES ICÔNES PORTENT UN `aria-label`, PAS UN TEXTE VIDE. Un bouton de transport se reconnaît à sa
// forme plus vite qu'il ne se lit — c'est tout l'intérêt — mais un lecteur d'écran, et les tests,
// ont besoin du mot.

import { Pause, Play, SkipForward, Square } from "lucide-react";

import type { SpeechQueue } from "../../speech/useSpeechQueue";

interface Props {
  queue: SpeechQueue;
  onSpeak: () => void;
}

export function TransportBar({ queue, onSpeak }: Props) {
  const running = queue.queue.length > 0;
  const paused = queue.state === "paused";

  return (
    <div className="bar transport">
      <button type="button" className="primary" onClick={onSpeak}>
        <Play size={15} aria-hidden="true" />
        Parler
      </button>
      <button
        type="button"
        className="icon"
        disabled={!running}
        aria-label={paused ? "Reprendre" : "Pause"}
        aria-pressed={paused}
        title={paused ? "Reprendre là où on en était" : "Suspendre"}
        onClick={queue.togglePause}
      >
        {paused ? <Play size={16} /> : <Pause size={16} />}
      </button>
      <button
        type="button"
        className="icon"
        disabled={!running}
        aria-label="Suivant"
        title="Couper celle-ci et passer à la suivante"
        onClick={queue.skip}
      >
        <SkipForward size={16} />
      </button>
      {/* Jamais désactivé : c'est le bouton qu'on écrase quand quelque chose part de travers,
          et le trouver éteint à ce moment-là serait le pire moment. */}
      <button
        type="button"
        className="icon"
        aria-label="Silence"
        title="Tout couper (Échap)"
        onClick={queue.stop}
      >
        <Square size={16} />
      </button>
    </div>
  );
}
