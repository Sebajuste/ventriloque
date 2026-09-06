// Ce que le player doit tenir.
//
// Le moteur est remplace ici par des promesses qu'on resout a la main : la file d'attente du
// player se decrit par des appels qui se CHEVAUCHENT, et un faux qui repond tout de suite ne
// laisse jamais deux repliques en vol.

import { useState } from "react";
import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import Player from "./Player";
import { parler, prechauffer, taire, type Cible, type Etat } from "../api";
import { etatDeTest, fiche, voix } from "../tests/donnees";

vi.mock("../api", async (original) => ({
  ...(await original<typeof import("../api")>()),
  parler: vi.fn(),
  taire: vi.fn(),
  prechauffer: vi.fn(),
}));

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
  vi.mocked(prechauffer).mockReset().mockResolvedValue([]);
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

describe("file d'attente", () => {
  it("compte ce qui reste a ENTENDRE, celle en cours comprise", async () => {
    const une = differe<void>();
    const deux = differe<void>();
    vi.mocked(parler).mockReturnValueOnce(une.promesse).mockReturnValueOnce(deux.promesse);

    monter(etatDeTest({ voix: [voix("Judy")] }));
    await userEvent.click(screen.getByText("Judy"));
    await userEvent.type(champ(), "Un.");

    await userEvent.click(screen.getByRole("button", { name: "Parler" }));
    expect(screen.getByText("…")).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Parler" }));
    expect(screen.getByText("2 répliques à venir")).toBeInTheDocument();

    une.resoudre();
    await waitFor(() => expect(screen.getByText("…")).toBeInTheDocument());

    deux.resoudre();
    await waitFor(() => expect(screen.queryByText("…")).not.toBeInTheDocument());
  });

  it("affiche la panne du moteur plutot que le decompte", async () => {
    const echec = differe<void>();
    vi.mocked(parler).mockReturnValueOnce(echec.promesse);

    monter(etatDeTest({ voix: [voix("Judy")] }));
    await userEvent.click(screen.getByText("Judy"));
    await userEvent.type(champ(), "Un.");
    await userEvent.click(screen.getByRole("button", { name: "Parler" }));

    echec.rejeter("le peripherique de sortie a disparu");

    await waitFor(() => {
      expect(screen.getByText("le peripherique de sortie a disparu")).toHaveClass("rate");
    });
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
