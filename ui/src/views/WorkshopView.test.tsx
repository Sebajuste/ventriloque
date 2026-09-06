// Ce que l'atelier doit tenir.
//
// La boucle de l'atelier passe par le player : une voix qui vient d'etre forgee doit se
// retrouver CHOISIE, sans que l'on ait a la chercher dans la liste. C'est ce qui fait de
// « forger, entendre, corriger » un geste et pas une navigation.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import WorkshopView from "./WorkshopView";
import { forgeVoice, pickAudioFiles } from "../ipc";

vi.mock("../ipc", async (original) => ({
  ...(await original<typeof import("../ipc")>()),
  pickAudioFiles: vi.fn(),
  forgeVoice: vi.fn(),
}));

const reload = vi.fn<() => Promise<void>>();
const setTarget = vi.fn();
const mount = () => render(<WorkshopView reload={reload} setTarget={setTarget} />);

const nameField = () => screen.getByLabelText(/Nom de la voix/);
const button = (name: string) => screen.getByRole("button", { name });

// PAS `getByRole`. Ce bouton-la vit dans un `<label>`, et un `<button>` est un element
// etiquetable : son nom accessible devient donc celui du label, « Sons du personnage », et pas
// son propre texte. On le prend par ce qu'on y lit.
const filesButton = () => screen.getByText("Choisir des fichiers…");

beforeEach(() => {
  reload.mockReset().mockResolvedValue(undefined);
  setTarget.mockReset();
  vi.mocked(pickAudioFiles).mockReset().mockResolvedValue([]);
  vi.mocked(forgeVoice).mockReset().mockResolvedValue("barman.wav");
});

describe("choix des sons", () => {
  it("passe par le selecteur de Rust, qui seul rend de vrais chemins", async () => {
    vi.mocked(pickAudioFiles).mockResolvedValue(["D:/sons/a.wav", "D:/sons/b.wav"]);
    mount();

    await userEvent.click(filesButton());

    await waitFor(() => expect(screen.getByText("2 fichiers")).toBeInTheDocument());
    expect(screen.getByText("D:/sons/a.wav")).toBeInTheDocument();
    expect(screen.getByText("D:/sons/b.wav")).toBeInTheDocument();
  });

  it("accorde le compte au singulier", async () => {
    vi.mocked(pickAudioFiles).mockResolvedValue(["D:/sons/a.wav"]);
    mount();

    await userEvent.click(filesButton());

    await waitFor(() => expect(screen.getByText("1 fichier")).toBeInTheDocument());
  });

  it("ne touche a rien si le selecteur est annule", async () => {
    vi.mocked(pickAudioFiles).mockResolvedValue(["D:/sons/a.wav"]);
    mount();
    await userEvent.click(filesButton());
    await waitFor(() => expect(screen.getByText("1 fichier")).toBeInTheDocument());

    vi.mocked(pickAudioFiles).mockResolvedValue([]);
    await userEvent.click(filesButton());

    // La liste d'avant tient : annuler n'est pas vider.
    expect(screen.getByText("D:/sons/a.wav")).toBeInTheDocument();
    expect(screen.getByText("1 fichier")).toBeInTheDocument();
  });
});

describe("refus", () => {
  it("exige un nom", async () => {
    vi.mocked(pickAudioFiles).mockResolvedValue(["D:/sons/a.wav"]);
    mount();
    await userEvent.click(filesButton());

    await userEvent.click(button("Fabriquer la voix"));

    expect(screen.getByText("il faut nommer la voix")).toHaveClass("failed");
    expect(forgeVoice).not.toHaveBeenCalled();
  });

  it("ne prend pas un nom fait d'espaces", async () => {
    mount();
    await userEvent.type(nameField(), "   ");

    await userEvent.click(button("Fabriquer la voix"));

    expect(screen.getByText("il faut nommer la voix")).toHaveClass("failed");
    expect(forgeVoice).not.toHaveBeenCalled();
  });

  it("exige au moins un son", async () => {
    mount();
    await userEvent.type(nameField(), "barman");

    await userEvent.click(button("Fabriquer la voix"));

    expect(screen.getByText("il faut au moins un fichier son")).toHaveClass("failed");
    expect(forgeVoice).not.toHaveBeenCalled();
  });
});

describe("fabrication", () => {
  const prepare = async () => {
    vi.mocked(pickAudioFiles).mockResolvedValue(["D:/sons/a.wav"]);
    mount();
    await userEvent.type(nameField(), "barman");
    await userEvent.click(filesButton());
    await waitFor(() => expect(screen.getByText("1 fichier")).toBeInTheDocument());
  };

  it("forge avec les deux curseurs a plat par defaut", async () => {
    await prepare();

    await userEvent.click(button("Fabriquer la voix"));

    await waitFor(() => expect(forgeVoice).toHaveBeenCalledWith("barman", ["D:/sons/a.wav"], 0, 0));
  });

  it("depose la voix neuve dans le player, sans son extension", async () => {
    await prepare();

    await userEvent.click(button("Fabriquer la voix"));

    await waitFor(() =>
      expect(setTarget).toHaveBeenCalledWith({
        name: "barman",
        reference: "barman.wav",
        kind: "clone",
        lines: [],
      }),
    );
    expect(reload).toHaveBeenCalledOnce();
  });

  it("montre la panne et laisse le bouton reprenable", async () => {
    vi.mocked(forgeVoice).mockRejectedValue("le moteur de parole n'a pas démarré");
    await prepare();

    await userEvent.click(button("Fabriquer la voix"));

    await waitFor(() =>
      expect(screen.getByText("le moteur de parole n'a pas démarré")).toHaveClass("failed"),
    );
    expect(button("Fabriquer la voix")).toBeEnabled();
    expect(setTarget).not.toHaveBeenCalled();
  });

  it("ferme le bouton pendant l'assemblage, pour ne pas forger deux fois", async () => {
    let finish!: (value: string) => void;
    vi.mocked(forgeVoice).mockReturnValue(new Promise<string>((ok) => (finish = ok)));
    await prepare();

    await userEvent.click(button("Fabriquer la voix"));

    expect(button("Fabriquer la voix")).toBeDisabled();
    expect(screen.getByText("assemblage et décalage…")).toBeInTheDocument();

    finish("barman.wav");
    await waitFor(() => expect(button("Fabriquer la voix")).toBeEnabled());
  });
});
