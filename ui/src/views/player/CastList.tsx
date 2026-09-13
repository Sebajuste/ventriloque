// La colonne de gauche : les personnages d'abord, parce que c'est avec eux qu'on joue, puis les
// voix brutes, utiles pour une silhouette qui n'a pas mérité de fiche.

import { useState } from "react";

import { NORMAL_PACE, type Snapshot, type Target, type Voice } from "../../ipc";

interface Props {
  snapshot: Snapshot;
  target: Target | null;
  onPick: (target: Target) => void;
}

/**
 * Une voix brute, prise telle quelle : elle n'a ni univers, ni répliques favorites, ni fiche où
 * retenir un débit.
 */
export const targetFromVoice = (voice: Voice): Target => ({
  name: voice.name,
  reference: voice.reference,
  kind: voice.kind,
  lines: [],
  character: "",
  pace: NORMAL_PACE,
});

export function CastList({ snapshot, target, onPick }: Props) {
  const [search, setSearch] = useState("");

  const matches = (text: string) =>
    search === "" || text.toLowerCase().includes(search.toLowerCase());
  const characters = snapshot.characters.filter((c) => matches(c.name) || matches(c.universe));
  const voices = snapshot.voices.filter((v) => matches(v.name));

  return (
    <>
      <input
        type="search"
        placeholder="Chercher"
        value={search}
        onChange={(e) => setSearch(e.target.value)}
      />
      <ul className="list">
        {characters.length > 0 && <li className="heading">Personnages</li>}
        {characters.map((character) => {
          const voice = snapshot.voices.find((v) => v.reference === character.voice);
          return (
            <Entry
              key={`c-${character.id}`}
              name={character.name}
              mark={character.universe || voice?.name || "sans voix"}
              active={target?.name === character.name && target.reference === character.voice}
              onClick={() =>
                onPick({
                  name: character.name,
                  reference: character.voice,
                  kind: voice?.kind ?? null,
                  lines: character.lines,
                  character: character.id,
                  pace: character.pace,
                })
              }
            />
          );
        })}

        {voices.length > 0 && <li className="heading">Voix</li>}
        {voices.map((voice) => (
          <Entry
            key={`v-${voice.reference}`}
            name={voice.name}
            mark={voice.kind}
            active={target?.name === voice.name && target.reference === voice.reference}
            onClick={() => onPick(targetFromVoice(voice))}
          />
        ))}
      </ul>
    </>
  );
}

function Entry(props: { name: string; mark: string; active: boolean; onClick: () => void }) {
  return (
    <li className={props.active ? "active" : undefined} onClick={props.onClick}>
      {props.name}
      <span className="tier">{props.mark}</span>
    </li>
  );
}
