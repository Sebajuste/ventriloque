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
// Ce que ça rend, en plus du contrôle : `abandon` est un drapeau UNIQUE et partagé dans
// `main.rs`. Avec plusieurs répliques en vol il était ambigu — couper l'une les coupait toutes.
// Avec une seule engagée, il désigne exactement ce qu'on croit.

import { useCallback, useEffect, useRef, useState } from "react";
import { avancement, enClair, parler, pause, reprendre, taire, type Avance } from "./api";

export interface Replique {
  id: number;
  /** Qui la dit — gardé avec la réplique, car on peut changer de personnage entre deux envois. */
  nom: string;
  reference: string;
  texte: string;
}

/** Où en est la tête de file. Les autres attendent, et n'ont pas d'état à elles. */
export type Etat = "prepare" | "parle" | "pause";

// LE MOTEUR EST INTERROGÉ, PLUS ESTIMÉ. Une version précédente devinait « prépare » ou « parle »
// au chronomètre — moins de 250 ms sur une voix connue, moins de 6 s sur un premier clonage.
// C'était faux dès que la machine peinait, et ça ne pouvait pas donner de progression du tout.
// Rust compte maintenant ce qui est réellement sorti du haut-parleur ; on le lit dix fois par
// seconde, ce qui est assez fin pour l'œil et négligeable pour l'IPC.
const BATTEMENT = 100;

const RIEN: Avance = { position: 0, duree: 0, complete: false, entendue: false };

export function useFileDeParole() {
  const [file, setFile] = useState<Replique[]>([]);
  const [enPause, setEnPause] = useState(false);
  const [panne, setPanne] = useState("");
  const [avance, setAvance] = useState<Avance>(RIEN);

  // Des références et pas des états : elles sont lues et écrites depuis des promesses qui se
  // chevauchent, et un état de React y perdrait des écritures.
  const engagee = useRef<number | null>(null);
  const enPauseVrai = useRef(false);
  const prochainId = useRef(0);

  // Engager la tête, et une seule fois. Le garde sur l'identifiant compte double : il empêche le
  // double montage de `StrictMode` d'envoyer la réplique deux fois, et il évite de la réengager
  // à chaque fois que la file change derrière elle.
  useEffect(() => {
    const tete = file[0];
    if (tete === undefined || engagee.current === tete.id) return;
    engagee.current = tete.id;
    setAvance(RIEN);

    void (async () => {
      try {
        await parler(tete.reference, tete.texte);
      } catch (e) {
        setPanne(enClair(e));
      } finally {
        enPauseVrai.current = false;
        setEnPause(false);
        setAvance(RIEN);
        setFile((f) => f.filter((r) => r.id !== tete.id));
      }
    })();
  }, [file]);

  // Le battement qui lit l'avancement. Il ne tourne que pendant qu'une réplique est engagée.
  useEffect(() => {
    if (file.length === 0) return;
    let vivant = true;
    const battement = setInterval(() => {
      void avancement().then((a) => {
        // La vue a pu être démontée entre la demande et la réponse : écrire ici avertirait React
        // d'une mise à jour sur un composant parti.
        if (vivant) setAvance(a);
      });
    }, BATTEMENT);
    return () => {
      vivant = false;
      clearInterval(battement);
    };
  }, [file.length]);

  const tete = file[0];
  const etat: Etat | null =
    tete === undefined ? null : enPause ? "pause" : avance.entendue ? "parle" : "prepare";

  const dire = useCallback((r: Omit<Replique, "id">) => {
    if (r.texte.trim() === "") return;
    setPanne("");
    setFile((f) => [...f, { ...r, id: prochainId.current++, texte: r.texte.trim() }]);
  }, []);

  /**
   * Tout jeter. La promesse en vol se résoudra de son côté et retirera sa tête d'une file déjà
   * vide — sans effet, et c'est voulu : on ne l'attend pas pour rendre la main.
   */
  const couper = useCallback(() => {
    setFile([]);
    void taire();
  }, []);

  /**
   * PASSER À LA SUIVANTE, C'EST COUPER CELLE-CI. Rien de plus : le lecteur lâche sa source, la
   * promesse en vol se résout, son `finally` retire la tête, et l'effet engage la suivante — dans
   * cet ordre, garanti par les promesses. Avancer la file nous-mêmes ici créerait une course
   * avec ce `finally`.
   */
  const passer = useCallback(() => {
    void taire();
  }, []);

  const basculerPause = useCallback(() => {
    const neuf = !enPauseVrai.current;
    enPauseVrai.current = neuf;
    setEnPause(neuf);
    void (neuf ? pause() : reprendre());
  }, []);

  /** Retirer une réplique qui attend. La tête ne se retire pas, elle se passe. */
  const retirer = useCallback((id: number) => {
    setFile((f) => (f[0]?.id === id ? f : f.filter((r) => r.id !== id)));
  }, []);

  /** Redire : la remettre en queue. « Précédent » n'existe pas — ce qui est sorti est perdu. */
  const redire = useCallback((id: number) => {
    setFile((f) => {
      const r = f.find((x) => x.id === id);
      return r === undefined ? f : [...f, { ...r, id: prochainId.current++ }];
    });
  }, []);

  return { file, etat, avance, panne, dire, couper, passer, basculerPause, retirer, redire };
}
