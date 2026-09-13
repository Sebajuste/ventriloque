// La file de parole : ce qui a été dit, ce qui sort du haut-parleur, et ce qui attend derrière.
//
// LA FILE VIT ICI, PAS DANS RUST, et c'est la décision qui commande tout le reste. Le lecteur
// audio ne sait retirer que sa tête ou tout jeter — rien au milieu. Tant que la fenêtre lui
// confiait toutes ses répliques d'un coup, en retirer une seule était impossible. Elle ne lui en
// confie donc plus qu'UNE à la fois : les suivantes sont à nous, on les retire, on les redit,
// on les réordonne.
//
// RIEN NE DISPARAÎT TOUT SEUL. Une réplique dite reste dans la liste, marquée comme telle : en
// séance on redit souvent ce qui vient de passer, et la retrouver sous les yeux vaut mieux que
// la retaper. On la retire à la main, ou l'on vide tout d'un coup.
//
// REDIRE REJOUE LA PRISE, SANS LE MOTEUR. Le moteur tire au sort à chaque synthèse : refaire la
// réplique donnerait une autre intonation, après l'attente du calcul. Chaque réplique dite
// garde donc le numéro de sa prise, et sa copie la rejoue telle quelle. Une nouvelle prise se
// demande à part.
//
// Ce que ça coûte : entre deux répliques enchaînées, le silence que met le moteur à sortir son
// premier morceau — ~200 ms sur une voix déjà entendue. Entre deux phrases d'un personnage, ce
// souffle passe pour une respiration ; c'est le prix du contrôle sur la file.
//
// Ce que ça rend, en plus du contrôle : le drapeau d'arrêt est UNIQUE et partagé côté Rust.
// Avec plusieurs répliques en vol il était ambigu — couper l'une les coupait toutes. Avec une
// seule engagée, il désigne exactement ce qu'on croit.

import { useCallback, useEffect, useRef, useState } from "react";

import {
  asMessage,
  pause,
  replay,
  resume,
  silence,
  speak,
  speechProgress,
  type SpeechProgress,
} from "../ipc";

/** Où en est une réplique. La tête de file est la première qui attend encore. */
export type LineStatus = "waiting" | "said" | "cut";

export interface Line {
  id: number;
  /** Qui la dit — gardé avec la réplique, car on peut changer de personnage entre deux envois. */
  speaker: string;
  reference: string;
  text: string;
  /** Le debit au moment de l'envoi : le changer ensuite ne rattrape pas ce qui attend deja. */
  pace: number;
  status: LineStatus;
  /** La prise que Rust garde pour la redire à l'identique. `null` tant qu'il faut la synthétiser. */
  take: number | null;
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

const firstWaiting = (lines: Line[]) => lines.find((line) => line.status === "waiting");

export function useSpeechQueue() {
  const [lines, setLines] = useState<Line[]>([]);
  const [paused, setPaused] = useState(false);
  const [failure, setFailure] = useState("");
  const [progress, setProgress] = useState<SpeechProgress>(IDLE);

  // Des références et pas des états : elles sont lues et écrites depuis des promesses qui se
  // chevauchent, et un état de React y perdrait des écritures.
  const engaged = useRef<number | null>(null);
  /** La réplique coupée avant sa fin : elle finira « coupée », pas « dite ». */
  const interrupted = useRef<number | null>(null);
  const pausedNow = useRef(false);
  const nextId = useRef(0);

  const head = firstWaiting(lines);

  // Engager la tête, et une seule fois. Le garde sur l'identifiant compte double : il empêche le
  // double montage de `StrictMode` d'envoyer la réplique deux fois, et il évite de la réengager
  // à chaque fois que la file change derrière elle.
  useEffect(() => {
    const line = firstWaiting(lines);
    if (line === undefined || engaged.current === line.id) return;
    engaged.current = line.id;
    setProgress(IDLE);

    void (async () => {
      let outcome: LineStatus = "said";
      let take = line.take;
      try {
        // Une prise oubliée de Rust, poussée dehors par de plus récentes, se refait.
        const replayed =
          take !== null && (await replay(take, line.reference, line.text, line.pace));
        if (!replayed) take = await speak(line.reference, line.text, line.pace);
      } catch (e) {
        setFailure(asMessage(e));
        outcome = "cut";
      }
      if (interrupted.current === line.id) outcome = "cut";
      pausedNow.current = false;
      setPaused(false);
      setProgress(IDLE);
      // Seulement si elle attend encore : le silence a pu la marquer « coupée » entre-temps, et
      // « vider » la retirer tout à fait.
      setLines((ls) =>
        ls.map((l) =>
          l.id === line.id && l.status === "waiting" ? { ...l, status: outcome, take } : l,
        ),
      );
    })();
  }, [lines]);

  // Le battement qui lit l'avancement. Il ne tourne que pendant qu'une réplique est engagée.
  const busy = head !== undefined;
  useEffect(() => {
    if (!busy) return;
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
  }, [busy]);

  const state: LineState | null =
    head === undefined ? null : paused ? "paused" : progress.audible ? "speaking" : "preparing";

  const say = useCallback((line: Omit<Line, "id" | "status" | "take">) => {
    if (line.text.trim() === "") return;
    setFailure("");
    setLines((ls) => [
      ...ls,
      { ...line, id: nextId.current++, text: line.text.trim(), status: "waiting", take: null },
    ]);
  }, []);

  /**
   * Tout couper, sans rien effacer : ce qui attendait passe « coupée », et reste là pour être
   * redit. La promesse en vol se résoudra de son côté ; on ne l'attend pas pour rendre la main.
   */
  const stop = useCallback(() => {
    interrupted.current = engaged.current;
    setLines((ls) => ls.map((l) => (l.status === "waiting" ? { ...l, status: "cut" } : l)));
    void silence();
  }, []);

  /** Tout effacer — et couper ce qui parle : une voix sans ligne à l'écran ne se pilote plus. */
  const clear = useCallback(() => {
    interrupted.current = engaged.current;
    setLines([]);
    void silence();
  }, []);

  /**
   * PASSER À LA SUIVANTE, C'EST COUPER CELLE-CI. Rien de plus : le lecteur lâche sa source, la
   * promesse en vol se résout, la marque « coupée », et l'effet engage la suivante — dans cet
   * ordre, garanti par les promesses. Avancer la file nous-mêmes ici créerait une course avec
   * cette résolution.
   */
  const skip = useCallback(() => {
    interrupted.current = engaged.current;
    void silence();
  }, []);

  const togglePause = useCallback(() => {
    const next = !pausedNow.current;
    pausedNow.current = next;
    setPaused(next);
    void (next ? pause() : resume());
  }, []);

  /** Retirer une réplique, dite ou à venir. La tête ne se retire pas, elle se passe. */
  const remove = useCallback((id: number) => {
    setLines((ls) => (firstWaiting(ls)?.id === id ? ls : ls.filter((l) => l.id !== id)));
  }, []);

  /**
   * Redire la même prise, à l'identique et sans attente. C'est LA MÊME LIGNE, déplacée en queue
   * de file : rien ne change dans ce qu'elle dit, un doublon n'apporterait rien.
   *
   * UN NOUVEL IDENTIFIANT, quand même : l'effet refuse de réengager un identifiant déjà engagé,
   * et la ligne qu'on vient d'entendre est justement celle-là.
   */
  const repeat = useCallback((id: number) => {
    setLines((ls) => {
      const line = ls.find((l) => l.id === id);
      if (line === undefined || line.status === "waiting") return ls;
      return [
        ...ls.filter((l) => l.id !== id),
        { ...line, id: nextId.current++, status: "waiting" },
      ];
    });
  }, []);

  /**
   * Redire avec une nouvelle prise : une autre intonation, au prix du calcul. UNE NOUVELLE LIGNE,
   * et l'ancienne reste avec sa prise : la nouvelle peut être moins bonne, et l'on doit pouvoir
   * revenir à celle d'avant.
   */
  const retake = useCallback((id: number) => {
    setLines((ls) => {
      const line = ls.find((l) => l.id === id);
      return line === undefined
        ? ls
        : [...ls, { ...line, id: nextId.current++, status: "waiting", take: null }];
    });
  }, []);

  return {
    lines,
    head,
    state,
    progress,
    failure,
    say,
    stop,
    clear,
    skip,
    togglePause,
    remove,
    repeat,
    retake,
  };
}

export type SpeechQueue = ReturnType<typeof useSpeechQueue>;
