// Les fiches de PNJ : nom, univers, voix, et les répliques qu'on redit souvent.
//
// Rien d'autre. L'histoire du personnage, ses liens, ses secrets sont déjà dans les notes du MJ
// et n'ont pas à être ressaisis ici.

import { useState } from "react";

import {
  NORMAL_PACE,
  asMessage,
  deleteCharacter,
  writeCharacter,
  type Character,
  type Snapshot,
} from "../ipc";

interface Props {
  snapshot: Snapshot;
  reload: () => Promise<void>;
}

const BLANK: Character = {
  id: "",
  name: "",
  universe: "",
  voice: "",
  lines: [],
  pace: NORMAL_PACE,
};

export default function CharactersView({ snapshot, reload }: Props) {
  const [draft, setDraft] = useState<Character>(BLANK);
  const [message, setMessage] = useState("");
  const [failed, setFailed] = useState(false);

  const announce = (what: string, bad = false) => {
    setMessage(what);
    setFailed(bad);
  };

  const edit = (character: Character) => {
    setDraft(character);
    announce("");
  };

  const save = async () => {
    try {
      // L'identifiant d'avant permet de renommer sans laisser un doublon derrière.
      const written = await writeCharacter({ ...draft, id: "" }, draft.id);
      await reload();
      setDraft(written);
      announce("enregistrée");
    } catch (e) {
      announce(asMessage(e), true);
    }
  };

  const remove = async () => {
    if (draft.id === "") return announce("rien à supprimer", true);
    try {
      await deleteCharacter(draft.id);
      await reload();
      setDraft(BLANK);
      announce("supprimée");
    } catch (e) {
      announce(asMessage(e), true);
    }
  };

  return (
    <main>
      <aside>
        <ul className="list">
          {snapshot.characters.map((character) => (
            <li
              key={character.id}
              className={character.id === draft.id ? "active" : undefined}
              onClick={() => edit(character)}
            >
              {character.name}
              {character.universe !== "" && (
                <span className="tier">{character.universe}</span>
              )}
            </li>
          ))}
        </ul>
        <button type="button" onClick={() => edit(BLANK)}>
          Nouveau personnage
        </button>
      </aside>

      <section className="form">
        <label>
          Nom
          <input
            value={draft.name}
            placeholder="le barman de la Lanterne"
            onChange={(e) => setDraft({ ...draft, name: e.target.value })}
          />
        </label>

        <label>
          Univers
          <input
            value={draft.universe}
            placeholder="Warhammer"
            onChange={(e) => setDraft({ ...draft, universe: e.target.value })}
          />
        </label>

        <label>
          Voix
          <select
            value={draft.voice}
            onChange={(e) => setDraft({ ...draft, voice: e.target.value })}
          >
            <option value="">— aucune —</option>
            {snapshot.voices.map((voice) => (
              <option key={voice.reference} value={voice.reference}>
                {voice.name} ({voice.kind})
              </option>
            ))}
          </select>
        </label>

        <label>
          Répliques favorites — une par ligne
          <textarea
            className="small"
            spellCheck={false}
            placeholder={"Vous êtes pas d'ici, vous.\nÇa fera trois couronnes."}
            value={draft.lines.join("\n")}
            onChange={(e) => setDraft({ ...draft, lines: e.target.value.split("\n") })}
          />
        </label>

        <p className="note">
          Cliquer une réplique favorite dans le player la fait dire tout de suite. C'est ce qui
          rend l'outil utilisable en pleine partie, quand on n'a pas le temps de taper.
        </p>

        <div className="bar">
          <button type="button" className="primary" onClick={() => void save()}>
            Enregistrer
          </button>
          <button type="button" onClick={() => void remove()}>
            Supprimer
          </button>
          <span className={failed ? "message failed" : "message"}>{message}</span>
        </div>
      </section>
    </main>
  );
}
