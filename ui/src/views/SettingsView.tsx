// Les réglages du moteur de parole : ce qui fait qu'une voix colle, ou non, à sa référence.
//
// RIEN NE S'APPLIQUE EN BOUGEANT UN CURSEUR. Le moteur ne lit ces valeurs qu'à son lancement :
// appliquer, c'est le relancer, ~2,5 s de silence. Un geste explicite évite de le relancer à
// chaque cran — et de couper une réplique en pleine séance parce qu'on a frôlé un curseur.

import { useEffect, useState, type ReactNode } from "react";

import {
  applyEngineSettings,
  asMessage,
  engineSettings,
  type EngineSettings,
  type EngineTuning,
} from "../ipc";

interface Props {
  reload: () => Promise<void>;
}

const same = (a: EngineSettings, b: EngineSettings) =>
  (Object.keys(a) as (keyof EngineSettings)[]).every((key) => a[key] === b[key]);

export default function SettingsView({ reload }: Props) {
  const [tuning, setTuning] = useState<EngineTuning | null>(null);
  const [draft, setDraft] = useState<EngineSettings | null>(null);
  const [message, setMessage] = useState("");
  const [failed, setFailed] = useState(false);
  const [busy, setBusy] = useState(false);

  const announce = (what: string, bad = false) => {
    setMessage(what);
    setFailed(bad);
  };

  useEffect(() => {
    engineSettings()
      .then((read) => {
        setTuning(read);
        setDraft(read.current);
      })
      .catch((e) => announce(asMessage(e), true));
  }, []);

  if (tuning === null || draft === null) {
    return (
      <main>
        <section className="form">
          <span className={failed ? "message failed" : "message"}>{message}</span>
        </section>
      </main>
    );
  }

  const set = (change: Partial<EngineSettings>) => setDraft({ ...draft, ...change });

  const apply = async () => {
    setBusy(true);
    announce("relance du moteur…");
    try {
      const written = await applyEngineSettings(draft);
      setTuning({ ...tuning, current: written });
      setDraft(written);
      announce("appliqué — le moteur tourne avec ces réglages");
    } catch (e) {
      announce(asMessage(e), true);
    } finally {
      setBusy(false);
      // La panne éventuelle du moteur s'affiche dans l'en-tête, sur toutes les pages.
      await reload();
    }
  };

  const followsPack = draft.eos_extra === null;

  return (
    <main>
      <section className="form tuning">
        <p className="note">
          Le moteur ne lit ces réglages qu'à son lancement : les appliquer le relance, quelques
          secondes pendant lesquelles rien ne parle. Les voix déjà clonées gardent leur cache.
        </p>

        <Setting
          label="Température"
          value={draft.temperature}
          min={0.05}
          max={1.5}
          step={0.05}
          show={(n) => n.toFixed(2)}
          onChange={(temperature) => set({ temperature })}
        >
          Le hasard injecté à chaque instant. Plus bas, la voix colle à sa référence et reste
          constante d'une réplique à l'autre, mais l'intonation s'aplatit ; plus haut, elle vit
          davantage et dérive plus souvent.
        </Setting>

        <Setting
          label="Écrêtage du bruit"
          value={draft.noise_clamp}
          min={0}
          max={5}
          step={0.25}
          show={(n) => (n === 0 ? "aucun" : n.toFixed(2))}
          onChange={(noise_clamp) => set({ noise_clamp })}
        >
          Borne les tirages extrêmes, en écarts-types — ceux qui font dérailler une syllabe. Vers
          2 ou 3 on retire les accidents sans toucher au reste ; plus bas, la voix se fige.
        </Setting>

        <Setting
          label="Étapes de calcul"
          value={draft.lsd_steps}
          min={1}
          max={10}
          step={1}
          show={String}
          onChange={(lsd_steps) => set({ lsd_steps })}
        >
          Les passes par instant de son. Une seule est la plus rapide ; plusieurs rattrapent mieux
          un mauvais tirage, mais chaque passe ajoute du calcul avant que la voix parte.
        </Setting>

        <Setting
          label="Seuil de fin de phrase"
          value={draft.eos_threshold}
          min={-10}
          max={0}
          step={0.5}
          show={(n) => n.toFixed(1)}
          onChange={(eos_threshold) => set({ eos_threshold })}
        >
          Quand le moteur juge la phrase finie. Plus bas, il s'arrête plus tôt et peut avaler la
          dernière syllabe ; plus haut, il peut laisser traîner un souffle ou un bredouillis.
        </Setting>

        <label className="check">
          <input
            type="checkbox"
            checked={followsPack}
            onChange={(e) => set({ eos_extra: e.target.checked ? null : 8 })}
          />
          Trames après la fin : celles du paquet de modèles
        </label>
        {!followsPack && (
          <Setting
            label="Trames après la fin"
            value={draft.eos_extra ?? 0}
            min={0}
            max={30}
            step={1}
            show={String}
            onChange={(eos_extra) => set({ eos_extra })}
          >
            Ce qu'on laisse courir une fois la phrase finie, par pas de 80 ms. Trop peu coupe la
            fin des mots ; trop laisse un blanc avant la réplique suivante.
          </Setting>
        )}

        <Setting
          label="Fils de calcul"
          value={draft.threads}
          min={0}
          max={32}
          step={1}
          show={(n) => (n === 0 ? "auto" : String(n))}
          onChange={(threads) => set({ threads })}
        >
          « auto » prend la moitié des cœurs. Un seul fil est le pire réglage : la voix sort à
          peine plus vite qu'on ne l'écoute.
        </Setting>

        <div className="bar">
          <button
            type="button"
            className="primary"
            disabled={busy || same(draft, tuning.current)}
            onClick={() => void apply()}
          >
            Appliquer
          </button>
          <button
            type="button"
            disabled={busy || same(draft, tuning.defaults)}
            onClick={() => {
              setDraft(tuning.defaults);
              announce("valeurs d'origine — reste à appliquer");
            }}
          >
            Valeurs d'origine
          </button>
          <span className={failed ? "message failed" : "message"}>{message}</span>
        </div>
      </section>
    </main>
  );
}

function Setting(props: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  show: (n: number) => string;
  onChange: (n: number) => void;
  children: ReactNode;
}) {
  return (
    <div className="setting">
      <label className="slider">
        <span className="name">{props.label}</span>
        <input
          type="range"
          min={props.min}
          max={props.max}
          step={props.step}
          value={props.value}
          onChange={(e) => props.onChange(Number(e.target.value))}
        />
        <output>{props.show(props.value)}</output>
      </label>
      <p className="note">{props.children}</p>
    </div>
  );
}
