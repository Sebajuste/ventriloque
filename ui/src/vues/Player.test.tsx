// Ce que le player doit tenir.
//
// Le moteur est remplace ici par des promesses qu'on resout a la main : la file d'attente du
// player se decrit par des appels qui se CHEVAUCHENT, et un faux qui repond tout de suite ne
// laisse jamais deux repliques en vol.

import { useState } from "react";
import { act, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import Player from "./Player";
import {
  avancement,
  parler,
  pause,
  prechauffer,
  reprendre,
  taire,
  type Avance,
  type Cible,
  type Etat,
} from "../api";
import { etatDeTest, fiche, voix } from "../tests/donnees";

vi.mock("../api", async (original) => ({
  ...(await original<typeof import("../api")>()),
  parler: vi.fn(),
  taire: vi.fn(),
  avancement: vi.fn(),
  pause: vi.fn(),
  reprendre: vi.fn(),
  prechauffer: vi.fn(),
}));

/** Rien n'est encore sorti du haut-parleur : la voix se prepare. */
const RIEN: Avance = { position: 0, duree: 0, complete: false, entendue: false };
const sorti = (sur: Partial<Avance>): Avance => ({ ...RIEN, entendue: true, ...sur });

/** Une promesse qu'on resout quand le test le decide, pour garder une replique « en vol ». */
const differe = <T,>() => {
  let resoudre!: (v: T) => void;
  let rejeter!: (e: unknown) => void;
  const promesse = new Promise<T>((ok, ko) => {
    resoudre = ok;
    rejeter = ko;
  });
  return { promesse, resoudre, rejeter };
};

/**
 * `choisie` appartient a App, pas au player : sans un parent qui la retient, un clic sur un
 * personnage ne le selectionnerait jamais et la moitie des tests porterait sur rien.
 */
function Fenetre({ etat }: { etat: Etat }) {
  const [choisie, setChoisie] = useState<Cible | null>(null);
  return <Player etat={etat} choisie={choisie} setChoisie={setChoisie} />;
}

const monter = (etat: Etat) => render(<Fenetre etat={etat} />);
const champ = () => screen.getByPlaceholderText(/Ce que dit le PNJ/);

beforeEach(() => {
  vi.mocked(parler).mockReset().mockResolvedValue(undefined);
  vi.mocked(taire).mockReset().mockResolvedValue(undefined);
  vi.mocked(pause).mockReset().mockResolvedValue(undefined);
  vi.mocked(reprendre).mockReset().mockResolvedValue(undefined);
  vi.mocked(prechauffer).mockReset().mockResolvedValue([]);
  vi.mocked(avancement).mockReset().mockResolvedValue(RIEN);
});

// UN TEST QUI EXPIRE NE DEROULE PAS SON `finally`. Un seul test a horloge simulee qui echoue
// laisserait donc tous les suivants sur des timers arretes, et ils expireraient a leur tour sans
// rapport avec ce qu'ils verifient. La restitution appartient au harnais, pas au test.
afterEach(() => {
  vi.useRealTimers();
});

describe("choix de la cible", () => {
  it("prend la voix de la fiche et le palier de la voix correspondante", async () => {
    monter(
      etatDeTest({
        voix: [voix("Judy", { palier: "clone" })],
        fiches: [fiche("Judy Alvarez", { voix: "judy.wav" })],
      }),
    );

    await userEvent.click(screen.getByText("Judy Alvarez"));

    expect(screen.getByText(/voix clonée/)).toBeInTheDocument();
  });

  it("signale une fiche dont la voix a disparu du disque", async () => {
    monter(etatDeTest({ voix: [], fiches: [fiche("Panam", { voix: "panam.wav" })] }));

    await userEvent.click(screen.getByText("Panam"));

    expect(screen.getByText(/voix introuvable/)).toBeInTheDocument();
  });

  it("refuse de parler pour une fiche sans voix", async () => {
    monter(etatDeTest({ fiches: [fiche("Silhouette", { voix: "" })] }));

    await userEvent.click(screen.getByText("Silhouette"));
    await userEvent.click(screen.getByRole("button", { name: "Parler" }));

    expect(screen.getByText("ce personnage n'a pas de voix")).toBeInTheDocument();
    expect(parler).not.toHaveBeenCalled();
  });

  it("refuse de parler tant que rien n'est choisi", async () => {
    monter(etatDeTest({ voix: [voix("Judy")] }));

    await userEvent.click(screen.getByRole("button", { name: "Parler" }));

    expect(screen.getByText("choisis d'abord un personnage ou une voix")).toBeInTheDocument();
    expect(parler).not.toHaveBeenCalled();
  });
});

describe("parler", () => {
  it("envoie la reference de la cible et le texte elague, a Ctrl+Entree", async () => {
    monter(etatDeTest({ voix: [voix("Judy")] }));

    await userEvent.click(screen.getByText("Judy"));
    await userEvent.type(champ(), "   Salut, V.   ");
    fireEvent.keyDown(champ(), { key: "Enter", ctrlKey: true });

    await waitFor(() => expect(parler).toHaveBeenCalledWith("judy.wav", "Salut, V."));
  });

  it("ne dit rien d'un champ vide", async () => {
    monter(etatDeTest({ voix: [voix("Judy")] }));

    await userEvent.click(screen.getByText("Judy"));
    await userEvent.type(champ(), "   ");
    await userEvent.click(screen.getByRole("button", { name: "Parler" }));

    expect(parler).not.toHaveBeenCalled();
  });

  it("dit une replique favorite d'un clic, sans passer par le champ", async () => {
    monter(
      etatDeTest({
        voix: [voix("Judy")],
        fiches: [fiche("Judy Alvarez", { voix: "judy.wav", repliques: ["On y va ?"] })],
      }),
    );

    await userEvent.click(screen.getByText("Judy Alvarez"));
    await userEvent.click(screen.getByText("On y va ?"));

    await waitFor(() => expect(parler).toHaveBeenCalledWith("judy.wav", "On y va ?"));
    expect(champ()).toHaveValue("");
  });

  it("garde le texte apres l'avoir dit, pour le rejouer", async () => {
    monter(etatDeTest({ voix: [voix("Judy")] }));

    await userEvent.click(screen.getByText("Judy"));
    await userEvent.type(champ(), "Encore.");
    await userEvent.click(screen.getByRole("button", { name: "Parler" }));

    await waitFor(() => expect(parler).toHaveBeenCalledOnce());
    expect(champ()).toHaveValue("Encore.");
  });
});

describe("la file", () => {
  const bouton = (nom: string) => screen.getByRole("button", { name: nom });

  // TOUTES LES RECHERCHES SONT CANTONNEES A LA LISTE. La zone de saisie garde la derniere
  // replique tapee, et React rend la valeur d'un `<textarea>` dans son contenu : un
  // `getByText("Deux.")` global en trouverait deux et echouerait sans rapport avec la file.
  // La liste n'existe pas quand la file est vide : la vue montre alors son etat vide a la
  // place. Les aides rendent donc « rien » plutot que d'echouer, et un test verifie le mot.
  const laFile = () => document.querySelector("ul.file");
  const dansLaFile = (texte: string) => {
    const ul = laFile();
    return ul === null ? null : within(ul as HTMLElement).queryByText(texte);
  };
  const ligneDe = (texte: string) =>
    within(laFile() as HTMLElement).getByText(texte).closest("li") as HTMLElement;
  const textes = () => [...(laFile()?.querySelectorAll(".texte") ?? [])].map((e) => e.textContent);

  /** Choisit une voix et depose des repliques, sans en resoudre aucune. */
  const remplir = async (...quoi: string[]) => {
    const vols = quoi.map(() => differe<void>());
    vols.forEach((v) => vi.mocked(parler).mockReturnValueOnce(v.promesse));
    monter(etatDeTest({ voix: [voix("Judy")] }));
    await userEvent.click(screen.getByText("Judy"));
    for (const t of quoi) {
      await userEvent.clear(champ());
      await userEvent.type(champ(), t);
      await userEvent.click(bouton("Parler"));
    }
    return vols;
  };

  it("montre ce qui est dit et ce qui attend, dans l'ordre", async () => {
    await remplir("Un.", "Deux.", "Trois.");

    expect(textes()).toEqual(["Un.", "Deux.", "Trois."]);
    expect(ligneDe("Un.")).toHaveClass("tete");
    expect(ligneDe("Deux.")).not.toHaveClass("tete");
  });

  it("N'ENGAGE QU'UNE REPLIQUE A LA FOIS DANS RUST", async () => {
    const vols = await remplir("Un.", "Deux.", "Trois.");

    // Le lecteur audio ne sait retirer que sa tete : lui confier les trois d'un coup rendrait
    // « Retirer » impossible sur la deuxieme.
    expect(parler).toHaveBeenCalledOnce();
    expect(parler).toHaveBeenCalledWith("judy.wav", "Un.");

    vols[0]?.resoudre();

    await waitFor(() => expect(parler).toHaveBeenCalledTimes(2));
    expect(parler).toHaveBeenLastCalledWith("judy.wav", "Deux.");
  });

  it("vide la ligne au fur et a mesure qu'elle sort du haut-parleur", async () => {
    const vols = await remplir("Un.", "Deux.");
    expect(dansLaFile("Un.")).toBeInTheDocument();

    vols[0]?.resoudre();

    await waitFor(() => expect(dansLaFile("Un.")).toBeNull());
    expect(textes()).toEqual(["Deux."]);
  });

  // `fireEvent` ET PAS `userEvent` : celui-ci s'endort entre deux frappes, et sur une horloge
  // arretee il ne se reveille jamais. Le test expirait alors AVANT de rendre l'horloge, et tous
  // les suivants heritaient de timers simules -- treize tests rouges pour une cause unique.
  // `afterEach` la rend maintenant quoi qu'il arrive ; ceci n'est que la ceinture.
  const engager = () => {
    vi.useFakeTimers();
    vi.mocked(parler).mockReturnValue(differe<void>().promesse);
    monter(etatDeTest({ voix: [voix("Judy")] }));
    fireEvent.click(screen.getByText("Judy"));
    fireEvent.change(champ(), { target: { value: "Un." } });
    fireEvent.click(bouton("Parler"));
  };

  /** Laisse passer un battement de sondage, reponse comprise. */
  const battre = () => act(async () => void (await vi.advanceTimersByTimeAsync(150)));

  it("dit « prepare la voix » tant que RIEN n'est sorti du haut-parleur", async () => {
    engager();

    // L'etat ne vient plus d'un chronometre : il vient de ce que Rust a reellement joue.
    expect(screen.getByText(/prépare la voix/)).toBeInTheDocument();

    vi.mocked(avancement).mockResolvedValue(sorti({ position: 1500 }));
    await battre();

    expect(screen.queryByText(/prépare la voix/)).not.toBeInTheDocument();
    expect(screen.getByText(/^0:01 —/)).toBeInTheDocument();
  });

  it("n'annonce une duree totale qu'une fois la synthese finie", async () => {
    engager();
    vi.mocked(avancement).mockResolvedValue(sorti({ position: 1500 }));
    await battre();

    // Le moteur fabrique plus vite qu'on n'ecoute : tant qu'il tourne, le total n'existe pas,
    // et un pourcentage calcule dessus reculerait a chaque morceau qui arrive.
    expect(screen.getByText(/^0:01 —/)).toBeInTheDocument();
    expect(screen.getByRole("progressbar")).toHaveClass("indeterminee");
    expect(screen.getByRole("progressbar")).not.toHaveAttribute("aria-valuenow");

    vi.mocked(avancement).mockResolvedValue(sorti({ position: 1500, duree: 6000, complete: true }));
    await battre();

    expect(screen.getByText("0:01 / 0:06 — Judy")).toBeInTheDocument();
    const barre = screen.getByRole("progressbar");
    expect(barre).not.toHaveClass("indeterminee");
    expect(barre).toHaveAttribute("aria-valuenow", "1500");
    expect(barre.firstElementChild).toHaveStyle({ width: "25%" });
  });

  it("passe a la suivante en coupant celle-ci, et rien d'autre", async () => {
    const vols = await remplir("Un.", "Deux.");

    await userEvent.click(bouton("Suivant"));

    expect(taire).toHaveBeenCalledOnce();
    // La file n'avance pas d'elle-meme : c'est la promesse coupee qui la fait avancer, ce qui
    // ordonne le retrait de la tete et l'envoi de la suivante au lieu de les mettre en course.
    expect(dansLaFile("Un.")).toBeInTheDocument();

    vols[0]?.resoudre();

    await waitFor(() => expect(dansLaFile("Un.")).toBeNull());
    expect(parler).toHaveBeenLastCalledWith("judy.wav", "Deux.");
  });

  it("le silence jette toute la file d'un coup", async () => {
    await remplir("Un.", "Deux.", "Trois.");

    await userEvent.click(bouton("Silence"));

    expect(taire).toHaveBeenCalledOnce();
    expect(textes()).toEqual([]);
    expect(screen.getByText(/Rien en file/)).toBeInTheDocument();
  });

  it("garde la place du bloc de lecture, pleine ou vide", async () => {
    // La hauteur est fixee en CSS, que jsdom ne calcule pas. Ce qui se teste ici est ce dont
    // elle depend : la zone existe TOUJOURS, et le transport est toujours avant elle. Sans ca,
    // la barre remonterait a chaque replique et l'on viserait un bouton qui vient de bouger.
    monter(etatDeTest({ voix: [voix("Judy")] }));

    const lecture = document.querySelector(".lecture");
    expect(lecture).not.toBeNull();
    const transport = document.querySelector(".transport");
    expect(transport?.compareDocumentPosition(lecture as Node)).toBe(
      Node.DOCUMENT_POSITION_FOLLOWING,
    );
  });

  it("suspend et reprend la ou l'on en etait", async () => {
    await remplir("Un.");

    await userEvent.click(bouton("Pause"));

    expect(pause).toHaveBeenCalledOnce();
    expect(screen.getByText(/en pause/)).toBeInTheDocument();

    await userEvent.click(bouton("Reprendre"));

    expect(reprendre).toHaveBeenCalledOnce();
    expect(screen.queryByText(/en pause/)).not.toBeInTheDocument();
  });

  it("retire une replique qui attend sans rien demander au moteur", async () => {
    await remplir("Un.", "Deux.");

    await userEvent.click(within(ligneDe("Deux.")).getByRole("button", { name: "Retirer" }));

    expect(textes()).toEqual(["Un."]);
    expect(taire).not.toHaveBeenCalled();
    expect(parler).toHaveBeenCalledOnce();
  });

  it("ne propose pas de retirer la tete : on la passe, on ne l'efface pas", async () => {
    await remplir("Un.", "Deux.");

    expect(within(ligneDe("Un.")).queryByRole("button", { name: "Retirer" })).toBeNull();
  });

  it("redire remet la replique en queue de file", async () => {
    await remplir("Un.", "Deux.");

    await userEvent.click(within(ligneDe("Un.")).getByRole("button", { name: "Redire" }));

    expect(textes()).toEqual(["Un.", "Deux.", "Un."]);
  });

  it("ferme le transport quand il n'y a rien a piloter, sauf le silence", async () => {
    monter(etatDeTest({ voix: [voix("Judy")] }));

    expect(bouton("Pause")).toBeDisabled();
    expect(bouton("Suivant")).toBeDisabled();
    // Jamais desactive : c'est le bouton qu'on ecrase quand quelque chose part de travers, et le
    // trouver eteint a ce moment-la serait le pire moment.
    expect(bouton("Silence")).toBeEnabled();
  });

  it("affiche la panne du moteur sans la confondre avec un etat de file", async () => {
    const echec = differe<void>();
    vi.mocked(parler).mockReturnValueOnce(echec.promesse);

    monter(etatDeTest({ voix: [voix("Judy")] }));
    await userEvent.click(screen.getByText("Judy"));
    await userEvent.type(champ(), "Un.");
    await userEvent.click(bouton("Parler"));

    echec.rejeter("le peripherique de sortie a disparu");

    await waitFor(() =>
      expect(screen.getByRole("alert")).toHaveTextContent("le peripherique de sortie a disparu"),
    );
  });
});

describe("echap", () => {
  it("coupe la voix d'ou que l'on soit dans la fenetre", () => {
    monter(etatDeTest({ voix: [voix("Judy")] }));

    fireEvent.keyDown(document, { key: "Escape" });

    expect(taire).toHaveBeenCalledOnce();
  });

  it("ne laisse pas d'ecouteur derriere elle une fois la vue partie", () => {
    const { unmount } = monter(etatDeTest({ voix: [voix("Judy")] }));
    unmount();

    fireEvent.keyDown(document, { key: "Escape" });

    expect(taire).not.toHaveBeenCalled();
  });
});

describe("filtre", () => {
  const peuple = () =>
    etatDeTest({
      voix: [voix("Nova"), voix("Judy")],
      fiches: [
        fiche("Judy Alvarez", { univers: "Cyberpunk 2077", voix: "judy.wav" }),
        fiche("Sarah Kerrigan", { univers: "StarCraft II", voix: "nova.wav" }),
      ],
    });

  const chercher = (quoi: string) =>
    userEvent.type(screen.getByPlaceholderText("Chercher"), quoi);

  it("retient une fiche par son nom", async () => {
    monter(peuple());
    await chercher("kerrigan");

    expect(screen.getByText("Sarah Kerrigan")).toBeInTheDocument();
    expect(screen.queryByText("Judy Alvarez")).not.toBeInTheDocument();
  });

  it("retient une fiche par son univers", async () => {
    monter(peuple());
    await chercher("starcraft");

    expect(screen.getByText("Sarah Kerrigan")).toBeInTheDocument();
    expect(screen.queryByText("Judy Alvarez")).not.toBeInTheDocument();
  });

  it("filtre les voix brutes sur leur seul nom, univers ou pas", async () => {
    monter(peuple());
    await chercher("nova");

    expect(screen.getByText("Nova")).toBeInTheDocument();
    expect(screen.queryByText("Judy")).not.toBeInTheDocument();
    expect(screen.queryByText("Sarah Kerrigan")).not.toBeInTheDocument();
  });

  it("ignore la casse", async () => {
    monter(peuple());
    await chercher("JUDY");

    expect(screen.getByText("Judy Alvarez")).toBeInTheDocument();
  });
});

describe("prechauffage", () => {
  it("reste hors d'atteinte tant que le moteur n'est pas pret", () => {
    monter(etatDeTest({ pret: false }));

    expect(screen.getByRole("button", { name: "Préchauffer les voix" })).toBeDisabled();
  });

  it("annonce les voix chauffees", async () => {
    vi.mocked(prechauffer).mockResolvedValue(["judy", "nova"]);
    monter(etatDeTest({ voix: [voix("Judy")] }));

    await userEvent.click(screen.getByRole("button", { name: "Préchauffer les voix" }));

    await waitFor(() => expect(screen.getByText("judy · nova")).toBeInTheDocument());
  });
});
