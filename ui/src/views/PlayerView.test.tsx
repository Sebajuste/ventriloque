// Ce que le player doit tenir.
//
// Le moteur est remplace ici par des promesses qu'on resout a la main : la file d'attente du
// player se decrit par des appels qui se CHEVAUCHENT, et un faux qui repond tout de suite ne
// laisse jamais deux repliques en vol.

import { useState } from "react";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import PlayerView from "./PlayerView";
import {
  pause,
  resume,
  silence,
  speak,
  speechProgress,
  warmUp,
  type Snapshot,
  type Target,
} from "../ipc";
import { SILENT, aCharacter, aVoice, deferred, heard, snapshotOf } from "../test/fixtures";

vi.mock("../ipc", async (original) => ({
  ...(await original<typeof import("../ipc")>()),
  speak: vi.fn(),
  silence: vi.fn(),
  speechProgress: vi.fn(),
  pause: vi.fn(),
  resume: vi.fn(),
  warmUp: vi.fn(),
}));

/**
 * `target` appartient a la fenetre, pas au player : sans un parent qui la retient, un clic sur un
 * personnage ne le selectionnerait jamais et la moitie des tests porterait sur rien.
 */
function Shell({ snapshot }: { snapshot: Snapshot }) {
  const [target, setTarget] = useState<Target | null>(null);
  return <PlayerView snapshot={snapshot} target={target} setTarget={setTarget} />;
}

const mount = (snapshot: Snapshot) => render(<Shell snapshot={snapshot} />);
const editor = () => screen.getByPlaceholderText(/Ce que dit le PNJ/);
const button = (name: string) => screen.getByRole("button", { name });

beforeEach(() => {
  vi.mocked(speak).mockReset().mockResolvedValue(undefined);
  vi.mocked(silence).mockReset().mockResolvedValue(undefined);
  vi.mocked(pause).mockReset().mockResolvedValue(undefined);
  vi.mocked(resume).mockReset().mockResolvedValue(undefined);
  vi.mocked(warmUp).mockReset().mockResolvedValue([]);
  vi.mocked(speechProgress).mockReset().mockResolvedValue(SILENT);
});

// UN TEST QUI EXPIRE NE DEROULE PAS SON `finally`. Un seul test a horloge simulee qui echoue
// laisserait donc tous les suivants sur des timers arretes, et ils expireraient a leur tour sans
// rapport avec ce qu'ils verifient. La restitution appartient au harnais, pas au test.
afterEach(() => {
  vi.useRealTimers();
});

describe("choix de la cible", () => {
  it("prend la voix de la fiche et le palier de la voix correspondante", async () => {
    mount(
      snapshotOf({
        voices: [aVoice("Judy", { kind: "clone" })],
        characters: [aCharacter("Judy Alvarez", { voice: "judy.wav" })],
      }),
    );

    await userEvent.click(screen.getByText("Judy Alvarez"));

    expect(screen.getByText(/voix clonée/)).toBeInTheDocument();
  });

  it("signale une fiche dont la voix a disparu du disque", async () => {
    mount(snapshotOf({ voices: [], characters: [aCharacter("Panam", { voice: "panam.wav" })] }));

    await userEvent.click(screen.getByText("Panam"));

    expect(screen.getByText(/voix introuvable/)).toBeInTheDocument();
  });

  it("refuse de parler pour une fiche sans voix", async () => {
    mount(snapshotOf({ characters: [aCharacter("Silhouette", { voice: "" })] }));

    await userEvent.click(screen.getByText("Silhouette"));
    await userEvent.click(button("Parler"));

    expect(screen.getByText("ce personnage n'a pas de voix")).toBeInTheDocument();
    expect(speak).not.toHaveBeenCalled();
  });

  it("refuse de parler tant que rien n'est choisi", async () => {
    mount(snapshotOf({ voices: [aVoice("Judy")] }));

    await userEvent.click(button("Parler"));

    expect(screen.getByText("choisis d'abord un personnage ou une voix")).toBeInTheDocument();
    expect(speak).not.toHaveBeenCalled();
  });
});

describe("parler", () => {
  it("envoie la reference de la cible et le texte elague, a Ctrl+Entree", async () => {
    mount(snapshotOf({ voices: [aVoice("Judy")] }));

    await userEvent.click(screen.getByText("Judy"));
    await userEvent.type(editor(), "   Salut, V.   ");
    fireEvent.keyDown(editor(), { key: "Enter", ctrlKey: true });

    await waitFor(() => expect(speak).toHaveBeenCalledWith("judy.wav", "Salut, V."));
  });

  it("ne dit rien d'un champ vide", async () => {
    mount(snapshotOf({ voices: [aVoice("Judy")] }));

    await userEvent.click(screen.getByText("Judy"));
    await userEvent.type(editor(), "   ");
    await userEvent.click(button("Parler"));

    expect(speak).not.toHaveBeenCalled();
  });

  it("dit une replique favorite d'un clic, sans passer par le champ", async () => {
    mount(
      snapshotOf({
        voices: [aVoice("Judy")],
        characters: [aCharacter("Judy Alvarez", { voice: "judy.wav", lines: ["On y va ?"] })],
      }),
    );

    await userEvent.click(screen.getByText("Judy Alvarez"));
    await userEvent.click(screen.getByText("On y va ?"));

    await waitFor(() => expect(speak).toHaveBeenCalledWith("judy.wav", "On y va ?"));
    expect(editor()).toHaveValue("");
  });

  it("garde le texte apres l'avoir dit, pour le rejouer", async () => {
    mount(snapshotOf({ voices: [aVoice("Judy")] }));

    await userEvent.click(screen.getByText("Judy"));
    await userEvent.type(editor(), "Encore.");
    await userEvent.click(button("Parler"));

    await waitFor(() => expect(speak).toHaveBeenCalledOnce());
    expect(editor()).toHaveValue("Encore.");
  });
});

describe("la file", () => {
  // TOUTES LES RECHERCHES SONT CANTONNEES A LA LISTE. La zone de saisie garde la derniere
  // replique tapee, et React rend la valeur d'un `<textarea>` dans son contenu : un
  // `getByText("Deux.")` global en trouverait deux et echouerait sans rapport avec la file.
  // La liste n'existe pas quand la file est vide : la vue montre alors son etat vide a la
  // place. Les aides rendent donc « rien » plutot que d'echouer, et un test verifie le mot.
  const list = () => document.querySelector("ul.queue");
  const inList = (text: string) => {
    const ul = list();
    return ul === null ? null : within(ul as HTMLElement).queryByText(text);
  };
  const rowOf = (text: string) =>
    within(list() as HTMLElement).getByText(text).closest("li") as HTMLElement;
  const texts = () => [...(list()?.querySelectorAll(".text") ?? [])].map((e) => e.textContent);

  /** Choisit une voix et depose des repliques, sans en resoudre aucune. */
  const fill = async (...lines: string[]) => {
    const inFlight = lines.map(() => deferred<void>());
    inFlight.forEach((f) => vi.mocked(speak).mockReturnValueOnce(f.promise));
    mount(snapshotOf({ voices: [aVoice("Judy")] }));
    await userEvent.click(screen.getByText("Judy"));
    for (const line of lines) {
      await userEvent.clear(editor());
      await userEvent.type(editor(), line);
      await userEvent.click(button("Parler"));
    }
    return inFlight;
  };

  it("montre ce qui est dit et ce qui attend, dans l'ordre", async () => {
    await fill("Un.", "Deux.", "Trois.");

    expect(texts()).toEqual(["Un.", "Deux.", "Trois."]);
    expect(rowOf("Un.")).toHaveClass("head");
    expect(rowOf("Deux.")).not.toHaveClass("head");
  });

  it("N'ENGAGE QU'UNE REPLIQUE A LA FOIS DANS RUST", async () => {
    const inFlight = await fill("Un.", "Deux.", "Trois.");

    // Le lecteur audio ne sait retirer que sa tete : lui confier les trois d'un coup rendrait
    // « Retirer » impossible sur la deuxieme.
    expect(speak).toHaveBeenCalledOnce();
    expect(speak).toHaveBeenCalledWith("judy.wav", "Un.");

    inFlight[0]?.resolve();

    await waitFor(() => expect(speak).toHaveBeenCalledTimes(2));
    expect(speak).toHaveBeenLastCalledWith("judy.wav", "Deux.");
  });

  it("vide la ligne au fur et a mesure qu'elle sort du haut-parleur", async () => {
    const inFlight = await fill("Un.", "Deux.");
    expect(inList("Un.")).toBeInTheDocument();

    inFlight[0]?.resolve();

    await waitFor(() => expect(inList("Un.")).toBeNull());
    expect(texts()).toEqual(["Deux."]);
  });

  // `fireEvent` ET PAS `userEvent` : celui-ci s'endort entre deux frappes, et sur une horloge
  // arretee il ne se reveille jamais. Le test expirait alors AVANT de rendre l'horloge, et tous
  // les suivants heritaient de timers simules -- treize tests rouges pour une cause unique.
  // `afterEach` la rend maintenant quoi qu'il arrive ; ceci n'est que la ceinture.
  const engage = () => {
    vi.useFakeTimers();
    vi.mocked(speak).mockReturnValue(deferred<void>().promise);
    mount(snapshotOf({ voices: [aVoice("Judy")] }));
    fireEvent.click(screen.getByText("Judy"));
    fireEvent.change(editor(), { target: { value: "Un." } });
    fireEvent.click(button("Parler"));
  };

  /** Laisse passer un battement de sondage, reponse comprise. */
  const tick = () => act(async () => void (await vi.advanceTimersByTimeAsync(150)));

  it("dit « prepare la voix » tant que RIEN n'est sorti du haut-parleur", async () => {
    engage();

    // L'etat ne vient plus d'un chronometre : il vient de ce que Rust a reellement joue.
    expect(screen.getByText(/prépare la voix/)).toBeInTheDocument();

    vi.mocked(speechProgress).mockResolvedValue(heard({ position: 1500 }));
    await tick();

    expect(screen.queryByText(/prépare la voix/)).not.toBeInTheDocument();
    expect(screen.getByText(/^0:01 —/)).toBeInTheDocument();
  });

  it("n'annonce une duree totale qu'une fois la synthese finie", async () => {
    engage();
    vi.mocked(speechProgress).mockResolvedValue(heard({ position: 1500 }));
    await tick();

    // Le moteur fabrique plus vite qu'on n'ecoute : tant qu'il tourne, le total n'existe pas,
    // et un pourcentage calcule dessus reculerait a chaque morceau qui arrive.
    expect(screen.getByText(/^0:01 —/)).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveClass("unknown");
    expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow");

    vi.mocked(speechProgress).mockResolvedValue(
      heard({ position: 1500, duration: 6000, complete: true }),
    );
    await tick();

    expect(screen.getByText("0:01 / 0:06 — Judy")).toBeInTheDocument();
    const bar = screen.getByRole("progressbar");
    expect(bar).not.toHaveClass("unknown");
    expect(bar).toHaveAttribute("aria-valuenow", "1500");
    expect(bar.firstElementChild).toHaveStyle({ width: "25%" });
  });

  it("passe a la suivante en coupant celle-ci, et rien d'autre", async () => {
    const inFlight = await fill("Un.", "Deux.");

    await userEvent.click(button("Suivant"));

    expect(silence).toHaveBeenCalledOnce();
    // La file n'avance pas d'elle-meme : c'est la promesse coupee qui la fait avancer, ce qui
    // ordonne le retrait de la tete et l'envoi de la suivante au lieu de les mettre en course.
    expect(inList("Un.")).toBeInTheDocument();

    inFlight[0]?.resolve();

    await waitFor(() => expect(inList("Un.")).toBeNull());
    expect(speak).toHaveBeenLastCalledWith("judy.wav", "Deux.");
  });

  it("le silence jette toute la file d'un coup", async () => {
    await fill("Un.", "Deux.", "Trois.");

    await userEvent.click(button("Silence"));

    expect(silence).toHaveBeenCalledOnce();
    expect(texts()).toEqual([]);
    expect(screen.getByText(/Rien en file/)).toBeInTheDocument();
  });

  it("garde la place du bloc de lecture, pleine ou vide", async () => {
    // La hauteur est fixee en CSS, que jsdom ne calcule pas. Ce qui se teste ici est ce dont
    // elle depend : la zone existe TOUJOURS, et le transport est toujours avant elle. Sans ca,
    // la barre remonterait a chaque replique et l'on viserait un bouton qui vient de bouger.
    mount(snapshotOf({ voices: [aVoice("Judy")] }));

    const reading = document.querySelector(".playback");
    expect(reading).not.toBeNull();
    const transport = document.querySelector(".transport");
    expect(transport?.compareDocumentPosition(reading as Node)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
  });

  it("suspend et reprend la ou l'on en etait", async () => {
    await fill("Un.");

    await userEvent.click(button("Pause"));

    expect(pause).toHaveBeenCalledOnce();
    expect(screen.getByText(/en pause/)).toBeInTheDocument();

    await userEvent.click(button("Reprendre"));

    expect(resume).toHaveBeenCalledOnce();
    expect(screen.queryByText(/en pause/)).not.toBeInTheDocument();
  });

  it("retire une replique qui attend sans rien demander au moteur", async () => {
    await fill("Un.", "Deux.");

    await userEvent.click(within(rowOf("Deux.")).getByRole("button", { name: "Retirer" }));

    expect(texts()).toEqual(["Un."]);
    expect(silence).not.toHaveBeenCalled();
    expect(speak).toHaveBeenCalledOnce();
  });

  it("ne propose pas de retirer la tete : on la passe, on ne l'efface pas", async () => {
    await fill("Un.", "Deux.");

    expect(within(rowOf("Un.")).queryByRole("button", { name: "Retirer" })).toBeNull();
  });

  it("redire remet la replique en queue de file", async () => {
    await fill("Un.", "Deux.");

    await userEvent.click(within(rowOf("Un.")).getByRole("button", { name: "Redire" }));

    expect(texts()).toEqual(["Un.", "Deux.", "Un."]);
  });

  it("ferme le transport quand il n'y a rien a piloter, sauf le silence", async () => {
    mount(snapshotOf({ voices: [aVoice("Judy")] }));

    expect(button("Pause")).toBeDisabled();
    expect(button("Suivant")).toBeDisabled();
    // Jamais desactive : c'est le bouton qu'on ecrase quand quelque chose part de travers, et le
    // trouver eteint a ce moment-la serait le pire moment.
    expect(button("Silence")).toBeEnabled();
  });

  it("affiche la panne du moteur sans la confondre avec un etat de file", async () => {
    const failure = deferred<void>();
    vi.mocked(speak).mockReturnValueOnce(failure.promise);

    mount(snapshotOf({ voices: [aVoice("Judy")] }));
    await userEvent.click(screen.getByText("Judy"));
    await userEvent.type(editor(), "Un.");
    await userEvent.click(button("Parler"));

    failure.reject("le peripherique de sortie a disparu");

    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent("le peripherique de sortie a disparu"),
    );
  });
});

describe("echap", () => {
  it("coupe la voix d'ou que l'on soit dans la fenetre", () => {
    mount(snapshotOf({ voices: [aVoice("Judy")] }));

    fireEvent.keyDown(document, { key: "Escape" });

    expect(silence).toHaveBeenCalledOnce();
  });

  it("ne laisse pas d'ecouteur derriere elle une fois la vue partie", () => {
    const { unmount } = mount(snapshotOf({ voices: [aVoice("Judy")] }));
    unmount();

    fireEvent.keyDown(document, { key: "Escape" });

    expect(silence).not.toHaveBeenCalled();
  });
});

describe("filtre", () => {
  const cast = () =>
    snapshotOf({
      voices: [aVoice("Nova"), aVoice("Judy")],
      characters: [
        aCharacter("Judy Alvarez", { universe: "Cyberpunk 2077", voice: "judy.wav" }),
        aCharacter("Sarah Kerrigan", { universe: "StarCraft II", voice: "nova.wav" }),
      ],
    });

  const search = (what: string) => userEvent.type(screen.getByPlaceholderText("Chercher"), what);

  it("retient une fiche par son nom", async () => {
    mount(cast());
    await search("kerrigan");

    expect(screen.getByText("Sarah Kerrigan")).toBeInTheDocument();
    expect(screen.queryByText("Judy Alvarez")).not.toBeInTheDocument();
  });

  it("retient une fiche par son univers", async () => {
    mount(cast());
    await search("starcraft");

    expect(screen.getByText("Sarah Kerrigan")).toBeInTheDocument();
    expect(screen.queryByText("Judy Alvarez")).not.toBeInTheDocument();
  });

  it("filtre les voix brutes sur leur seul nom, univers ou pas", async () => {
    mount(cast());
    await search("nova");

    expect(screen.getByText("Nova")).toBeInTheDocument();
    expect(screen.queryByText("Judy")).not.toBeInTheDocument();
    expect(screen.queryByText("Sarah Kerrigan")).not.toBeInTheDocument();
  });

  it("ignore la casse", async () => {
    mount(cast());
    await search("JUDY");

    expect(screen.getByText("Judy Alvarez")).toBeInTheDocument();
  });
});

describe("prechauffage", () => {
  it("reste hors d'atteinte tant que le moteur n'est pas pret", () => {
    mount(snapshotOf({ ready: false }));

    expect(button("Préchauffer les voix")).toBeDisabled();
  });

  it("annonce les voix chauffees", async () => {
    vi.mocked(warmUp).mockResolvedValue(["judy", "nova"]);
    mount(snapshotOf({ voices: [aVoice("Judy")] }));

    await userEvent.click(button("Préchauffer les voix"));

    await waitFor(() => expect(screen.getByText("judy · nova")).toBeInTheDocument());
  });
});
