// La file de parole : ce qui sort du haut-parleur, et ce qui attend derrière.
//
// LA FILE VIT ICI, PAS DANS RUST, et c'est la décision qui commande tout le reste. Le lecteur
// audio ne sait retirer que sa tête ou tout jeter — rien au milieu. Tant que la fenêtre lui
// confiait toutes ses répliques d'un coup, en retirer une seule était impossible. Elle ne lui en
// confie donc plus qu'UNE à la fois : les suivantes sont à nous, on les retire, on les redit,
// on les réordonne.
//
// Ce que ça coûte : entre deux répliques enchaînées, le silence que met le moteur à sortir son
// premier morceau — ~200 ms sur une voix déjà entendue. Avant, la suivante se fabriquait pendant
// qu'on écoutait la précédente et l'enchaînement était sans couture. Entre deux phrases d'un
// personnage, ce souffle passe pour une respiration ; c'est le prix du contrôle sur la file.
//
// Ce que ça rend, en plus du contrôle : le drapeau d'arrêt est UNIQUE et partagé côté Rust.
// Avec plusieurs répliques en vol il était ambigu — couper l'une les coupait toutes. Avec une
// seule engagée, il désigne exactement ce qu'on croit.

import { useCallback, useEffect, useRef, useState } from "react";

import {
  asMessage,
  pause,
  resume,
  silence,
  speak,
  speechProgress,
  type SpeechProgress,
} from "../ipc";

export interface Line {
  id: number;
  /** Qui la dit — gardé avec la réplique, car on peut changer de personnage entre deux envois. */
  speaker: string;
  reference: string;
  text: string;
}

/** Où en est la tête de file. Les autres attendent, et n'ont pas d'état à elles. */
export type LineState = "preparing" | "speaking" | "paused";

// LE MOTEUR EST INTERROGÉ, PLUS ESTIMÉ. Une version précédente devinait « prépare » ou « parle »
// au chronomètre — moins de 250 ms sur une voix connue, moins de 6 s sur un premier clonage.
// C'était faux dès que la machine peinait, et ça ne pouvait pas donner de progression du tout.
// Rust compte maintenant ce qui est réellement sorti du haut-parleur ; on le lit dix fois par
// seconde, ce qui est assez fin pour l'œil et négligeable pour l'IPC.
const TICK_MS = 100;

const IDLE: SpeechProgress = { position: 0, duration: 0, complete: false, audible: false };

export function useSpeechQueue() {
  const [queue, setQueue] = useState<Line[]>([]);
  const [paused, setPaused] = useState(false);
  const [failure, setFailure] = useState("");
  const [progress, setProgress] = useState<SpeechProgress>(IDLE);

  // Des références et pas des états : elles sont lues et écrites depuis des promesses qui se
  // chevauchent, et un état de React y perdrait des écritures.
  const engaged = useRef<number | null>(null);
  const pausedNow = useRef(false);
  const nextId = useRef(0);

  // Engager la tête, et une seule fois. Le garde sur l'identifiant compte double : il empêche le
  // double montage de `StrictMode` d'envoyer la réplique deux fois, et il évite de la réengager
  // à chaque fois que la file change derrière elle.
  useEffect(() => {
    const head = queue[0];
    if (head === undefined || engaged.current === head.id) return;
    engaged.current = head.id;
    setProgress(IDLE);

    void (async () => {
      try {
        await speak(head.reference, head.text);
      } catch (e) {
        setFailure(asMessage(e));
      } finally {
        pausedNow.current = false;
        setPaused(false);
        setProgress(IDLE);
        setQueue((q) => q.filter((line) => line.id !== head.id));
      }
    })();
  }, [queue]);

  // Le battement qui lit l'avancement. Il ne tourne que pendant qu'une réplique est engagée.
  useEffect(() => {
    if (queue.length === 0) return;
    let alive = true;
    const tick = setInterval(() => {
      void speechProgress().then((p) => {
        // La vue a pu être démontée entre la demande et la réponse : écrire ici avertirait React
        // d'une mise à jour sur un composant parti.
        if (alive) setProgress(p);
      });
    }, TICK_MS);
    return () => {
      alive = false;
      clearInterval(tick);
    };
  }, [queue.length]);

  const head = queue[0];
  const state: LineState | null =
    head === undefined ? null : paused ? "paused" : progress.audible ? "speaking" : "preparing";

  const say = useCallback((line: Omit<Line, "id">) => {
    if (line.text.trim() === "") return;
    setFailure("");
    setQueue((q) => [...q, { ...line, id: nextId.current++, text: line.text.trim() }]);
  }, []);

  /**
   * Tout jeter. La promesse en vol se résoudra de son côté et retirera sa tête d'une file déjà
   * vide — sans effet, et c'est voulu : on ne l'attend pas pour rendre la main.
   */
  const stop = useCallback(() => {
    setQueue([]);
    void silence();
  }, []);

  /**
   * PASSER À LA SUIVANTE, C'EST COUPER CELLE-CI. Rien de plus : le lecteur lâche sa source, la
   * promesse en vol se résout, son `finally` retire la tête, et l'effet engage la suivante — dans
   * cet ordre, garanti par les promesses. Avancer la file nous-mêmes ici créerait une course
   * avec ce `finally`.
   */
  const skip = useCallback(() => {
    void silence();
  }, []);

  const togglePause = useCallback(() => {
    const next = !pausedNow.current;
    pausedNow.current = next;
    setPaused(next);
    void (next ? pause() : resume());
  }, []);

  /** Retirer une réplique qui attend. La tête ne se retire pas, elle se passe. */
  const remove = useCallback((id: number) => {
    setQueue((q) => (q[0]?.id === id ? q : q.filter((line) => line.id !== id)));
  }, []);

  /** Redire : la remettre en queue. « Précédent » n'existe pas — ce qui est sorti est perdu. */
  const repeat = useCallback((id: number) => {
    setQueue((q) => {
      const line = q.find((l) => l.id === id);
      return line === undefined ? q : [...q, { ...line, id: nextId.current++ }];
    });
  }, []);

  return { queue, state, progress, failure, say, stop, skip, togglePause, remove, repeat };
}

export type SpeechQueue = ReturnType<typeof useSpeechQueue>;
