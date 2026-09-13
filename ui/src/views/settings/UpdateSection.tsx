// La mise à jour, proposée et jamais subie.
//
// RIEN NE SE CHERCHE AU MONTAGE. Une recherche coûte un aller-retour réseau, et hors ligne elle
// met plusieurs secondes à échouer : ouvrir les réglages ne doit pas traîner pour ça, et une
// séance sans internet ne doit rien voir clignoter. C'est un bouton.
//
// INSTALLER COUPE LA PAROLE. L'installateur remplace l'exécutable et le moteur est tué avant
// lui : c'est dit avant le clic, pas découvert après.

import { useEffect, useState } from "react";

import {
  asMessage,
  checkUpdate,
  installUpdate,
  updateProgress,
  type UpdateFound,
  type UpdateProgress,
} from "../../ipc";

// Le même battement que les paquets : une barre de téléchargement n'a pas besoin de plus fin.
const TICK_MS = 200;

const IDLE: UpdateProgress = { active: false, downloaded: 0, total: 0 };

/** `12,4 Mo` — un installateur se compte en mégaoctets, jamais en octets. */
const megabytes = (bytes: number) => `${(bytes / 1_000_000).toFixed(1)} Mo`;

export default function UpdateSection() {
  const [found, setFound] = useState<UpdateFound | null>(null);
  const [message, setMessage] = useState("");
  const [failed, setFailed] = useState(false);
  const [busy, setBusy] = useState<"" | "recherche" | "installation">("");
  const [progress, setProgress] = useState<UpdateProgress>(IDLE);

  // Le battement ne tourne QUE pendant l'installation, et s'arrête avec elle.
  useEffect(() => {
    if (busy !== "installation") return;
    let alive = true;
    const tick = setInterval(() => {
      void updateProgress().then((p) => {
        // La vue a pu être démontée entre la demande et la réponse.
        if (alive) setProgress(p);
      });
    }, TICK_MS);
    return () => {
      alive = false;
      clearInterval(tick);
    };
  }, [busy]);

  const announce = (what: string, bad = false) => {
    setMessage(what);
    setFailed(bad);
  };

  const look = async () => {
    setBusy("recherche");
    announce("recherche…");
    setFound(null);
    try {
      const seen = await checkUpdate();
      if (seen.version === "") {
        announce(`Ventriloque ${seen.current} est à jour`);
      } else {
        setFound(seen);
        announce("");
      }
    } catch (e) {
      announce(asMessage(e), true);
    } finally {
      setBusy("");
    }
  };

  const install = async () => {
    setBusy("installation");
    setProgress(IDLE);
    announce("téléchargement…");
    try {
      await installUpdate();
      // On n'arrive ici que si la relance n'a pas eu lieu : autrement la fenêtre est déjà partie.
      announce("installé — Ventriloque redémarre");
    } catch (e) {
      announce(asMessage(e), true);
      setBusy("");
      setProgress(IDLE);
    }
  };

  const known = progress.total > 0;
  const share = known ? Math.round((progress.downloaded / progress.total) * 100) : 0;

  return (
    <section className="form">
      <h2>Mise à jour</h2>
      <p className="note">
        Ventriloque va chercher ses mises à jour sur sa page de publication. Le paquet est vérifié
        par signature avant d'être posé : un installateur qui ne vient pas de là est refusé.
      </p>

      {found !== null && (
        <>
          <p>
            Version {found.version} disponible — vous avez la {found.current}.
          </p>
          {found.notes !== "" && <pre className="log">{found.notes}</pre>}
          <p className="note">
            Installer remplace Ventriloque et le relance : le moteur de parole est coupé, une
            réplique en cours s'arrête net. Les voix, les fiches et les paquets ne bougent pas.
          </p>
        </>
      )}

      {busy === "installation" && (
        <div className="progress">
          <div
            className="gauge"
            role="progressbar"
            aria-valuenow={known ? progress.downloaded : undefined}
            aria-valuemax={known ? progress.total : undefined}
            aria-label="Téléchargement de la mise à jour"
          >
            <div
              className={known ? "filled" : "filled unknown"}
              style={known ? { width: `${share}%` } : undefined}
            />
          </div>
          <div className="note detail">
            {known
              ? `${share} % — ${megabytes(progress.downloaded)} sur ${megabytes(progress.total)}`
              : megabytes(progress.downloaded)}
          </div>
        </div>
      )}

      <div className="bar">
        <button type="button" disabled={busy !== ""} onClick={() => void look()}>
          Chercher une mise à jour
        </button>
        {found !== null && (
          <button
            type="button"
            className="primary"
            disabled={busy !== ""}
            onClick={() => void install()}
          >
            Installer et relancer
          </button>
        )}
        <span className={failed ? "message failed" : "message"}>{message}</span>
      </div>
    </section>
  );
}
