// Le battement qui fait vivre la barre des paquets.
//
// Les commandes ne rendent la main qu'à la fin : sans lui, une installation de 258 Mo n'aurait
// rien à montrer pendant une minute. Il ne tourne que pendant qu'une opération est engagée, et
// s'arrête avec elle.

import { useEffect, useState } from "react";

import { asMessage, packProgress, type PackProgress } from "../../ipc";

// Le même battement que la file de parole, deux fois plus lent : une barre d'installation n'a
// pas besoin de la finesse d'une réplique.
const TICK_MS = 200;

const IDLE: PackProgress = { active: false, done: 0, total: 0, step: "", log: [] };

export function usePackJob(reload: () => Promise<void>) {
  const [busy, setBusy] = useState("");
  const [report, setReport] = useState("");
  const [failed, setFailed] = useState(false);
  const [progress, setProgress] = useState<PackProgress>(IDLE);

  useEffect(() => {
    if (busy === "") return;
    let alive = true;
    const tick = setInterval(() => {
      void packProgress().then((p) => {
        // La vue a pu être démontée entre la demande et la réponse.
        if (alive) setProgress(p);
      });
    }, TICK_MS);
    return () => {
      alive = false;
      clearInterval(tick);
    };
  }, [busy]);

  /** Le même enrobage pour les trois gestes : ils rendent tous un compte rendu ou une plainte. */
  const run = async (what: string, gesture: () => Promise<string>) => {
    setBusy(what);
    setProgress(IDLE);
    setReport("");
    setFailed(false);
    try {
      const outcome = await gesture();
      setReport(outcome === "" ? "annulé" : outcome);
      if (outcome !== "") await reload();
    } catch (e) {
      setReport(asMessage(e));
      setFailed(true);
    } finally {
      setBusy("");
      setProgress(IDLE);
    }
  };

  return { busy, report, failed, progress, run };
}

export type PackJob = ReturnType<typeof usePackJob>;
