// Payer d'avance le clonage : ~6 s par voix, une seule fois, pendant qu'on installe la table.
//
// Entendues au moment où un PNJ prend la parole, ces secondes sont un silence que personne ne
// comprend. Passées ici, elles n'existent pas.

import { useState } from "react";

import { asMessage, warmUp } from "../../ipc";

export function WarmUpButton({ ready }: { ready: boolean }) {
  const [report, setReport] = useState("");
  const [busy, setBusy] = useState(false);

  const run = async () => {
    setBusy(true);
    setReport("préchauffage…");
    try {
      setReport((await warmUp()).join(" · "));
    } catch (e) {
      setReport(asMessage(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <>
      <button type="button" disabled={busy || !ready} onClick={() => void run()}>
        Préchauffer les voix
      </button>
      {report !== "" && <p className="warm-up">{report}</p>}
    </>
  );
}
