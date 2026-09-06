// Les paquets : des données qui arrivent par un zip.
//
// C'est ce qui remplace un installateur. L'exécutable se copie et se lance ; tout ce qui pèse —
// les modèles, les voix — s'ajoute ensuite depuis cette page.
//
// UN PAQUET PEUT AUSSI N'APPORTER QU'UNE RECETTE : le savoir d'où sont les voix dans un jeu
// installé, sans en porter une seule. Il arrive alors « à fabriquer », et c'est le bouton
// ci-dessous — jamais l'installation — qui fait tourner son script, hors de l'application.

import { useEffect, useRef, useState } from "react";
import {
  avancementPack,
  desinstallerPack,
  enClair,
  fabriquerPack,
  installerPack,
  type AvancePack,
  type Etat,
  type Manifeste,
} from "../api";

interface Props {
  etat: Etat;
  relire: () => Promise<void>;
}

const compte = (n: number) => `${n} fichier${n > 1 ? "s" : ""}`;

// Le même battement que la file de parole. Les commandes ne rendent la main qu'à la fin :
// sans lui, une installation de 258 Mo n'aurait rien à montrer pendant une minute.
const BATTEMENT = 200;
const REPOS: AvancePack = { actif: false, faits: 0, total: 0, quoi: "", journal: [] };

export default function Packs({ etat, relire }: Props) {
  const [dit, setDit] = useState("");
  const [rate, setRate] = useState(false);
  const [occupe, setOccupe] = useState("");
  // Le paquet dont le retrait est armé. Un premier clic arme, un second efface : une boîte de
  // dialogue de plus se clique sans la lire, alors qu'un bouton qui change de texte se voit.
  const [arme, setArme] = useState("");
  const [avance, setAvance] = useState<AvancePack>(REPOS);
  const fil = useRef<HTMLPreElement>(null);

  // Un journal qui grandit doit montrer sa DERNIÈRE ligne : c'est celle qui dit où l'on en
  // est. Sans cela, la fenêtre resterait sur « stockage ouvert » pendant trois minutes.
  useEffect(() => {
    const p = fil.current;
    if (p) p.scrollTop = p.scrollHeight;
  }, [avance.journal.length]);

  // Il ne tourne que pendant qu'une opération est engagée, et s'arrête avec elle.
  useEffect(() => {
    if (occupe === "") return;
    let vivant = true;
    const battement = setInterval(() => {
      void avancementPack().then((a) => {
        // La vue a pu être démontée entre la demande et la réponse.
        if (vivant) setAvance(a);
      });
    }, BATTEMENT);
    return () => {
      vivant = false;
      clearInterval(battement);
    };
  }, [occupe]);

  // Le même enrobage pour les trois gestes : ils rendent tous un compte rendu ou une plainte.
  const mener = async (quoi: string, geste: () => Promise<string>) => {
    setOccupe(quoi);
    setArme("");
    setAvance(REPOS);
    setDit("");
    setRate(false);
    try {
      const compteRendu = await geste();
      setDit(compteRendu === "" ? "annulé" : compteRendu);
      if (compteRendu !== "") await relire();
    } catch (e) {
      setDit(enClair(e));
      setRate(true);
    } finally {
      setOccupe("");
      setAvance(REPOS);
    }
  };

  // Le compte rendu d'une installation tient sur une ligne, celui d'une fabrication en fait
  // vingt : le premier va dans la barre, le second sous la liste.
  const surUneLigne = !dit.includes("\n");

  // Une proportion n'a de sens que si le total est connu. Une fabrication ne le connaît
  // pas : sa jauge se contente d'aller et venir, et c'est la ligne d'étape qui informe.
  const part = avance.total > 0 ? Math.round((avance.faits / avance.total) * 100) : 0;
  const libelle =
    occupe === "fabrication"
      ? "fabrication en cours, quelques minutes…"
      : occupe === "retrait"
        ? "retrait en cours…"
        : avance.total > 0
          ? `installation — ${part} %`
          : "installation en cours…";

  const detail = (p: Manifeste) =>
    [
      p.description,
      p.auteur,
      p.fichiers.length > 0 ? compte(p.fichiers.length) : "",
      p.installe_le,
    ]
      .filter((t) => t !== "")
      .join(" · ");

  return (
    <main>
      <section className="formulaire">
        <div className="barre">
          <button
            type="button"
            className="fort"
            disabled={occupe !== ""}
            onClick={() => void mener("installation", installerPack)}
          >
            Installer un paquet…
          </button>
          <span className={rate ? "dit rate" : "dit"}>
            {occupe === "" ? (surUneLigne ? dit : "") : libelle}
          </span>
        </div>

        {occupe !== "" && (
          <div className="progression">
            <div
              className="jauge"
              role="progressbar"
              aria-valuenow={avance.total > 0 ? avance.faits : undefined}
              aria-valuemax={avance.total > 0 ? avance.total : undefined}
              aria-label={libelle}
            >
              <div
                className={avance.total > 0 ? "remplie" : "remplie indeterminee"}
                style={avance.total > 0 ? { width: `${part}%` } : undefined}
              />
            </div>
            {avance.quoi !== "" && <div className="note detail">{avance.quoi}</div>}
          </div>
        )}

        {/* Le journal en direct pendant l'opération, le compte rendu une fois finie. */}
        {occupe !== "" && avance.journal.length > 0 && (
          <pre className="journal" ref={fil}>
            {avance.journal.join("\n")}
          </pre>
        )}

        <p className="note">
          Un paquet est un zip qui porte un <code>pack.json</code> et des dossiers{" "}
          <code>voix/</code>, <code>pnj/</code>, <code>modeles/</code> ou <code>recettes/</code>.
          Rien d'autre n'est extrait : une entrée qui vise ailleurs est ignorée.
        </p>
        <p className="note">
          Le paquet <strong>modèles</strong> est celui qui fait parler l'application. Sans lui,
          Ventriloque s'ouvre mais reste muet — c'est l'état normal d'une première installation.
        </p>

        <ul className="packs">
          {etat.packs.length === 0 && <li className="note">Aucun paquet installé.</li>}
          {etat.packs.map((p) => {
            const aFabriquer = p.recette !== "" && p.fabrique_le === "";
            const porteLesModeles = p.fichiers.some((f) => f.startsWith("modeles/"));
            return (
              <li key={p.nom}>
                <strong>{p.version === "" ? p.nom : `${p.nom} ${p.version}`}</strong>
                <div className="note">{detail(p)}</div>

                <div className="barre">
                  {p.recette !== "" && (
                    <>
                      <button
                        type="button"
                        disabled={occupe !== ""}
                        onClick={() => void mener("fabrication", () => fabriquerPack(p.nom, false))}
                      >
                        {aFabriquer ? "Fabriquer…" : "Refabriquer…"}
                      </button>
                      {/* La sortie de secours : quand la détection ne trouve rien, ou se trompe. */}
                      <button
                        type="button"
                        disabled={occupe !== ""}
                        onClick={() => void mener("fabrication", () => fabriquerPack(p.nom, true))}
                      >
                        Autre dossier…
                      </button>
                    </>
                  )}

                  <button
                    type="button"
                    className={arme === p.nom ? "danger" : undefined}
                    disabled={occupe !== ""}
                    onClick={() =>
                      arme === p.nom
                        ? void mener("retrait", () => desinstallerPack(p.nom))
                        : setArme(p.nom)
                    }
                  >
                    {arme === p.nom ? "Confirmer le retrait" : "Retirer…"}
                  </button>

                  <span className="note">
                    {arme === p.nom
                      ? porteLesModeles
                        ? "ceci retire les modèles : Ventriloque redeviendra muet"
                        : `${p.fichiers.length} fichier${p.fichiers.length > 1 ? "s" : ""} seront effacés`
                      : p.recette === ""
                        ? ""
                        : aFabriquer
                          ? `à fabriquer depuis une copie de ${p.jeu === "" ? "votre jeu" : p.jeu}`
                          : `fabriqué le ${p.fabrique_le}`}
                  </span>
                </div>
              </li>
            );
          })}
        </ul>

        {occupe === "" && dit !== "" && !surUneLigne && (
          <pre className={rate ? "journal rate" : "journal"}>{dit}</pre>
        )}
      </section>
    </main>
  );
}
