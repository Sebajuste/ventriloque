// Le player : la vue de séance.
//
// TROIS CANAUX DE MESSAGE, PAS UN SEUL. La version d'avant partageait une chaîne unique entre le
// décompte, les pannes du moteur et le préchauffage : une panne effaçait le décompte, et le
// décompte effaçait la panne — au moment précis où l'on avait besoin des deux. Ici la file dit
// ce qui parle, la ligne rouge dit ce qui a cassé, et le préchauffage parle chez lui.
//
// Échap coupe la voix D'OÙ QUE L'ON SOIT dans la fenêtre. En pleine partie on ne cherche pas le
// bon champ avant de faire taire un PNJ.

import { useEffect, useRef, useState } from "react";

import { asMessage, writeCharacter, type Snapshot, type Target } from "../ipc";
import { useSpeechQueue } from "../speech/useSpeechQueue";
import { CastList } from "./player/CastList";
import { SpeechQueueList } from "./player/SpeechQueueList";
import { TransportBar } from "./player/TransportBar";
import { WarmUpButton } from "./player/WarmUpButton";

interface Props {
  snapshot: Snapshot;
  reload: () => Promise<void>;
  target: Target | null;
  setTarget: (target: Target) => void;
}

/**
 * Le débit s'écrit dans la fiche une fois le curseur posé, pas à chaque cran : un glissé en
 * produit une vingtaine, et chacun réécrirait le fichier.
 */
const PACE_SAVE_MS = 400;

export default function PlayerView({ snapshot, reload, target, setTarget }: Props) {
  const [text, setText] = useState("");
  const [refusal, setRefusal] = useState("");
  const editor = useRef<HTMLTextAreaElement>(null);
  const saving = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const queue = useSpeechQueue();

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") queue.stop();
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [queue]);

  const say = (forced?: string) => {
    if (target === null) return setRefusal("choisis d'abord un personnage ou une voix");
    if (target.reference === "") return setRefusal("ce personnage n'a pas de voix");
    const what = (forced ?? text).trim();
    if (what === "") return;

    setRefusal("");
    queue.say({ speaker: target.name, reference: target.reference, text: what, pace: target.pace });
  };

  // LE DEBIT APPARTIENT AU PERSONNAGE, et c'est dans sa fiche qu'il se retient. Il s'entend dès
  // la réplique suivante, sans attendre l'écriture. Une voix brute n'a pas de fiche : son débit
  // ne vaut que tant qu'on la garde choisie.
  const changePace = (pace: number) => {
    if (target === null) return;
    setTarget({ ...target, pace });
    const sheet = snapshot.characters.find((c) => c.id === target.character);
    if (sheet === undefined) return;
    clearTimeout(saving.current);
    saving.current = setTimeout(() => {
      writeCharacter({ ...sheet, id: "", pace }, sheet.id)
        .then(reload)
        .catch((e) => setRefusal(asMessage(e)));
    }, PACE_SAVE_MS);
  };

  const pick = (picked: Target) => {
    setTarget(picked);
    setRefusal("");
    editor.current?.focus();
  };

  return (
    <main>
      <aside>
        <CastList snapshot={snapshot} target={target} onPick={pick} />
        <WarmUpButton ready={snapshot.ready} />
      </aside>

      <section>
        <div className="speaker-row">
          <div className="speaker">
            {target === null ? (
              "Choisis un personnage ou une voix"
            ) : (
              <>
                <strong>{target.name}</strong> — {describe(target)}
              </>
            )}
          </div>
          {target !== null && (
            <label
              className="pace"
              title="Étire ou tasse la voix sans changer sa hauteur. Retenu dans la fiche du personnage."
            >
              Débit
              <input
                type="range"
                min={70}
                max={140}
                step={5}
                value={target.pace}
                onChange={(e) => changePace(Number(e.target.value))}
              />
              <output>{target.pace} %</output>
            </label>
          )}
        </div>

        {/* Un clic dit la réplique : c'est le geste utile en pleine partie, quand on n'a pas
            le temps de taper. */}
        {target !== null && target.lines.length > 0 && (
          <ul className="favourites">
            {target.lines.map((line) => (
              <li key={line} title="Dire cette réplique" onClick={() => say(line)}>
                {line}
              </li>
            ))}
          </ul>
        )}

        <textarea
          ref={editor}
          spellCheck={false}
          placeholder={"Ce que dit le PNJ\n\nCtrl+Entrée pour parler, Échap pour couper"}
          value={text}
          onChange={(e) => setText(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
              e.preventDefault();
              say();
            }
          }}
        />

        <TransportBar queue={queue} onSpeak={() => say()} />
        <SpeechQueueList queue={queue} failure={queue.failure !== "" ? queue.failure : refusal} />
      </section>
    </main>
  );
}

/** D'où vient la voix de la cible, dit en clair sous son nom. */
function describe(target: Target): string {
  if (target.reference === "") return "aucune voix";
  if (target.kind === "clone") return "voix clonée";
  if (target.kind === "catalog") return "voix de catalogue";
  return "voix introuvable";
}
