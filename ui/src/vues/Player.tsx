// Le player : la vue de séance.
//
// Deux groupes dans la colonne de gauche — les personnages d'abord, parce que c'est avec eux
// qu'on joue, puis les voix brutes, utiles pour une silhouette qui n'a pas mérité de fiche.
//
// TROIS CANAUX DE MESSAGE, PAS UN SEUL. La version d'avant partageait une chaîne unique entre le
// décompte, les pannes du moteur et le préchauffage : une panne effaçait le décompte, et le
// décompte effaçait la panne — au moment précis où l'on avait besoin des deux. Ici la file dit
// ce qui parle, la ligne rouge dit ce qui a cassé, et l'atelier de préchauffage parle chez lui.
//
// Échap coupe la voix D'OÙ QUE L'ON SOIT dans la fenêtre. En pleine partie on ne cherche pas le
// bon champ avant de faire taire un PNJ.

import { useEffect, useRef, useState } from "react";
import { Pause, Play, RotateCcw, SkipForward, Square, X } from "lucide-react";
import { enClair, prechauffer, type Cible, type Etat, type Voix } from "../api";
import { useFileDeParole } from "../file";

interface Props {
  etat: Etat;
  choisie: Cible | null;
  setChoisie: (c: Cible) => void;
}

const depuisVoix = (v: Voix): Cible => ({
  nom: v.nom,
  reference: v.reference,
  palier: v.palier,
  repliques: [],
});

export default function Player({ etat, choisie, setChoisie }: Props) {
  const [filtre, setFiltre] = useState("");
  const [texte, setTexte] = useState("");
  const [refus, setRefus] = useState("");
  const [chauffage, setChauffage] = useState("");
  const [chauffe, setChauffe] = useState(false);
  const zone = useRef<HTMLTextAreaElement>(null);

  const parole = useFileDeParole();

  useEffect(() => {
    const surTouche = (e: KeyboardEvent) => {
      if (e.key === "Escape") parole.couper();
    };
    document.addEventListener("keydown", surTouche);
    return () => document.removeEventListener("keydown", surTouche);
  }, [parole]);

  const dire = (impose?: string) => {
    if (choisie === null) return setRefus("choisis d'abord un personnage ou une voix");
    if (choisie.reference === "") return setRefus("ce personnage n'a pas de voix");
    const quoi = (impose ?? texte).trim();
    if (quoi === "") return;

    setRefus("");
    parole.dire({ nom: choisie.nom, reference: choisie.reference, texte: quoi });
  };

  const garde = (t: string) => filtre === "" || t.toLowerCase().includes(filtre.toLowerCase());
  const fiches = etat.fiches.filter((f) => garde(f.nom) || garde(f.univers));
  const brutes = etat.voix.filter((v) => garde(v.nom));

  const prendre = (c: Cible) => {
    setChoisie(c);
    setRefus("");
    zone.current?.focus();
  };

  const marqueDe = (c: Cible) =>
    c.reference === ""
      ? "aucune voix"
      : c.palier === "clone"
        ? "voix clonée"
        : c.palier === "catalogue"
          ? "voix de catalogue"
          : "voix introuvable";

  const enCours = parole.file.length > 0;
  const panne = parole.panne !== "" ? parole.panne : refus;

  return (
    <main>
      <aside>
        <input
          type="search"
          placeholder="Chercher"
          value={filtre}
          onChange={(e) => setFiltre(e.target.value)}
        />
        <ul className="liste">
          {fiches.length > 0 && <li className="titre">Personnages</li>}
          {fiches.map((f) => {
            const voisine = etat.voix.find((v) => v.reference === f.voix);
            const cible: Cible = {
              nom: f.nom,
              reference: f.voix,
              palier: voisine?.palier ?? null,
              repliques: f.repliques,
            };
            return (
              <Entree
                key={`f-${f.id}`}
                nom={f.nom}
                marque={f.univers || voisine?.nom || "sans voix"}
                actif={choisie?.nom === f.nom && choisie.reference === f.voix}
                onClick={() => prendre(cible)}
              />
            );
          })}

          {brutes.length > 0 && <li className="titre">Voix</li>}
          {brutes.map((v) => (
            <Entree
              key={`v-${v.reference}`}
              nom={v.nom}
              marque={v.palier}
              actif={choisie?.nom === v.nom && choisie.reference === v.reference}
              onClick={() => prendre(depuisVoix(v))}
            />
          ))}
        </ul>

        {/* Payer d'avance le clonage : ~6 s par voix, une seule fois, pendant qu'on installe
            la table. Entendues au moment où un PNJ prend la parole, ces secondes sont un
            silence que personne ne comprend. */}
        <button
          type="button"
          disabled={chauffe || !etat.pret}
          onClick={async () => {
            setChauffe(true);
            setChauffage("préchauffage…");
            try {
              setChauffage((await prechauffer()).join(" · "));
            } catch (e) {
              setChauffage(enClair(e));
            } finally {
              setChauffe(false);
            }
          }}
        >
          Préchauffer les voix
        </button>
        {chauffage !== "" && <p className="chauffage">{chauffage}</p>}
      </aside>

      <section>
        <div className="qui">
          {choisie === null ? (
            "Choisis un personnage ou une voix"
          ) : (
            <>
              <strong>{choisie.nom}</strong> — {marqueDe(choisie)}
            </>
          )}
        </div>

        {/* Un clic dit la réplique : c'est le geste utile en pleine partie, quand on n'a pas
            le temps de taper. */}
        {choisie !== null && choisie.repliques.length > 0 && (
          <ul className="favorites">
            {choisie.repliques.map((r) => (
              <li key={r} title="Dire cette réplique" onClick={() => dire(r)}>
                {r}
              </li>
            ))}
          </ul>
        )}

        <textarea
          ref={zone}
          spellCheck={false}
          placeholder={"Ce que dit le PNJ\n\nCtrl+Entrée pour parler, Échap pour couper"}
          value={texte}
          onChange={(e) => setTexte(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
              e.preventDefault();
              dire();
            }
          }}
        />

        {/* LES ICÔNES PORTENT UN `aria-label`, PAS UN TEXTE VIDE. Un bouton de transport se
            reconnaît à sa forme plus vite qu'il ne se lit — c'est tout l'intérêt — mais un
            lecteur d'écran, et les tests, ont besoin du mot. */}
        <div className="barre transport">
          <button type="button" className="fort" onClick={() => dire()}>
            <Play size={15} aria-hidden="true" />
            Parler
          </button>
          <button
            type="button"
            className="icone"
            disabled={!enCours}
            aria-label={parole.etat === "pause" ? "Reprendre" : "Pause"}
            aria-pressed={parole.etat === "pause"}
            title={parole.etat === "pause" ? "Reprendre là où on en était" : "Suspendre"}
            onClick={parole.basculerPause}
          >
            {parole.etat === "pause" ? <Play size={16} /> : <Pause size={16} />}
          </button>
          <button
            type="button"
            className="icone"
            disabled={!enCours}
            aria-label="Suivant"
            title="Couper celle-ci et passer à la suivante"
            onClick={parole.passer}
          >
            <SkipForward size={16} />
          </button>
          {/* Jamais désactivé : c'est le bouton qu'on écrase quand quelque chose part de
              travers, et le trouver éteint à ce moment-là serait le pire moment. */}
          <button
            type="button"
            className="icone"
            aria-label="Silence"
            title="Tout couper (Échap)"
            onClick={parole.couper}
          >
            <Square size={16} />
          </button>
        </div>

        {/* LA PLACE EST RÉSERVÉE, PLEINE OU VIDE. Le bloc de lecture avait la hauteur de son
            contenu : chaque réplique ajoutée poussait le transport vers le haut, et on visait
            un bouton qui venait de bouger — en pleine partie, exactement ce qu'il ne faut pas.
            Ici la zone garde sa hauteur, et c'est la zone de texte qui absorbe la différence. */}
        <div className="lecture">
          {panne !== "" && (
            <p className="dit rate" role="alert">
              {panne}
            </p>
          )}

          {parole.file.length === 0 ? (
            <p className="vide">Rien en file. Ctrl+Entrée pour parler, Échap pour couper.</p>
          ) : (
            <ul className="file">
              {parole.file.map((r, rang) => (
                <li key={r.id} className={rang === 0 ? "tete" : undefined}>
                  <span className="puce" aria-hidden="true" />
                  <span className="corps">
                    <span className="texte">{r.texte}</span>
                    <span className="etat">
                      {rang > 0
                        ? `en attente — ${r.nom}`
                        : parole.etat === "pause"
                          ? `en pause — ${r.nom}`
                          : parole.etat === "prepare"
                            ? `prépare la voix… — ${r.nom}`
                            : `${horloge(parole.avance.position)}${
                                parole.avance.complete
                                  ? ` / ${horloge(parole.avance.duree)}`
                                  : ""
                              } — ${r.nom}`}
                    </span>
                    {rang === 0 && parole.etat !== "prepare" && (
                      <Barre avance={parole.avance} />
                    )}
                  </span>
                  <span className="gestes">
                    <button
                      type="button"
                      className="icone"
                      aria-label="Redire"
                      title="Redire cette réplique"
                      onClick={() => parole.redire(r.id)}
                    >
                      <RotateCcw size={14} />
                    </button>
                    {rang > 0 && (
                      <button
                        type="button"
                        className="icone"
                        aria-label="Retirer"
                        title="Retirer de la file"
                        onClick={() => parole.retirer(r.id)}
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
      </section>
    </main>
  );
}

/** `1:04`, ou `0:07` — les répliques se comptent en secondes, pas en heures. */
const horloge = (ms: number) => {
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};

/**
 * DEUX BARRES, PAS UNE. Tant que la synthèse tourne, la durée totale n'existe pas : le moteur
 * fabrique environ trois fois plus vite qu'on n'écoute, et un pourcentage calculé sur ce qui est
 * fabriqué à cet instant reculerait à chaque morceau qui arrive. La barre avance alors sans
 * promettre de fin ; elle devient une vraie proportion dès que le total est connu — ce qui
 * arrive vite, autour du premier tiers de la réplique.
 */
function Barre({ avance }: { avance: { position: number; duree: number; complete: boolean } }) {
  const part = avance.complete && avance.duree > 0 ? avance.position / avance.duree : 0;
  return (
    <span
      className={avance.complete ? "barre-avance" : "barre-avance indeterminee"}
      role="progressbar"
      aria-label="Progression de la réplique"
      {...(avance.complete
        ? { "aria-valuemin": 0, "aria-valuemax": avance.duree, "aria-valuenow": avance.position }
        : {})}
    >
      <span style={avance.complete ? { width: `${Math.min(100, part * 100)}%` } : undefined} />
    </span>
  );
}

function Entree(props: { nom: string; marque: string; actif: boolean; onClick: () => void }) {
  return (
    <li className={props.actif ? "actif" : undefined} onClick={props.onClick}>
      {props.nom}
      <span className="palier">{props.marque}</span>
    </li>
  );
}
