// Ce que l'atelier doit tenir.
//
// La boucle de l'atelier passe par le player : une voix qui vient d'etre forgee doit se
// retrouver CHOISIE, sans que l'on ait a la chercher dans la liste. C'est ce qui fait de
// « forger, entendre, corriger » un geste et pas une navigation.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import Atelier from "./Atelier";
import { choisirFichiers, forger } from "../api";

vi.mock("../api", async (original) => ({
  ...(await original<typeof import("../api")>()),
  choisirFichiers: vi.fn(),
  forger: vi.fn(),
}));

const relire = vi.fn<() => Promise<void>>();
const setChoisie = vi.fn();
const monter = () => render(<Atelier relire={relire} setChoisie={setChoisie} />);

const champNom = () => screen.getByLabelText(/Nom de la voix/);
const bouton = (nom: string) => screen.getByRole("button", { name: nom });

// PAS `getByRole`. Ce bouton-la vit dans un `<label>`, et un `<button>` est un element
// etiquetable : son nom accessible devient donc celui du label, « Sons du personnage », et pas
// son propre texte. On le prend par ce qu'on y lit.
const boutonFichiers = () => screen.getByText("Choisir des fichiers…");

beforeEach(() => {
  relire.mockReset().mockResolvedValue(undefined);
  setChoisie.mockReset();
  vi.mocked(choisirFichiers).mockReset().mockResolvedValue([]);
  vi.mocked(forger).mockReset().mockResolvedValue("barman.wav");
});

describe("choix des sons", () => {
  it("passe par le selecteur de Rust, qui seul rend de vrais chemins", async () => {
    vi.mocked(choisirFichiers).mockResolvedValue(["D:/sons/a.wav", "D:/sons/b.wav"]);
    monter();

    await userEvent.click(boutonFichiers());

    await waitFor(() => expect(screen.getByText("2 fichiers")).toBeInTheDocument());
    expect(screen.getByText("D:/sons/a.wav")).toBeInTheDocument();
    expect(screen.getByText("D:/sons/b.wav")).toBeInTheDocument();
  });

  it("accorde le compte au singulier", async () => {
    vi.mocked(choisirFichiers).mockResolvedValue(["D:/sons/a.wav"]);
    monter();

    await userEvent.click(boutonFichiers());

    await waitFor(() => expect(screen.getByText("1 fichier")).toBeInTheDocument());
  });

  it("ne touche a rien si le selecteur est annule", async () => {
    vi.mocked(choisirFichiers).mockResolvedValue(["D:/sons/a.wav"]);
    monter();
    await userEvent.click(boutonFichiers());
    await waitFor(() => expect(screen.getByText("1 fichier")).toBeInTheDocument());

    vi.mocked(choisirFichiers).mockResolvedValue([]);
    await userEvent.click(boutonFichiers());

    // La liste d'avant tient : annuler n'est pas vider.
    expect(screen.getByText("D:/sons/a.wav")).toBeInTheDocument();
    expect(screen.getByText("1 fichier")).toBeInTheDocument();
  });
});

describe("refus", () => {
  it("exige un nom", async () => {
    vi.mocked(choisirFichiers).mockResolvedValue(["D:/sons/a.wav"]);
    monter();
    await userEvent.click(boutonFichiers());

    await userEvent.click(bouton("Fabriquer la voix"));

    expect(screen.getByText("il faut nommer la voix")).toHaveClass("rate");
    expect(forger).not.toHaveBeenCalled();
  });

  it("ne prend pas un nom fait d'espaces", async () => {
    monter();
    await userEvent.type(champNom(), "   ");

    await userEvent.click(bouton("Fabriquer la voix"));

    expect(screen.getByText("il faut nommer la voix")).toHaveClass("rate");
    expect(forger).not.toHaveBeenCalled();
  });

  it("exige au moins un son", async () => {
    monter();
    await userEvent.type(champNom(), "barman");

    await userEvent.click(bouton("Fabriquer la voix"));

    expect(screen.getByText("il faut au moins un fichier son")).toHaveClass("rate");
    expect(forger).not.toHaveBeenCalled();
  });
});

describe("fabrication", () => {
  const prepare = async () => {
    vi.mocked(choisirFichiers).mockResolvedValue(["D:/sons/a.wav"]);
    monter();
    await userEvent.type(champNom(), "barman");
    await userEvent.click(boutonFichiers());
    await waitFor(() => expect(screen.getByText("1 fichier")).toBeInTheDocument());
  };

  it("forge avec les deux curseurs a plat par defaut", async () => {
    await prepare();

    await userEvent.click(bouton("Fabriquer la voix"));

    await waitFor(() => expect(forger).toHaveBeenCalledWith("barman", ["D:/sons/a.wav"], 0, 0));
  });

  it("depose la voix neuve dans le player, sans son extension", async () => {
    await prepare();

    await userEvent.click(bouton("Fabriquer la voix"));

    await waitFor(() =>
      expect(setChoisie).toHaveBeenCalledWith({
        nom: "barman",
        reference: "barman.wav",
        palier: "clone",
        repliques: [],
      }),
    );
    expect(relire).toHaveBeenCalledOnce();
  });

  it("montre la panne et laisse le bouton reprenable", async () => {
    vi.mocked(forger).mockRejectedValue("le moteur de parole n'a pas démarré");
    await prepare();

    await userEvent.click(bouton("Fabriquer la voix"));

    await waitFor(() =>
      expect(screen.getByText("le moteur de parole n'a pas démarré")).toHaveClass("rate"),
    );
    expect(bouton("Fabriquer la voix")).toBeEnabled();
    expect(setChoisie).not.toHaveBeenCalled();
  });

  it("ferme le bouton pendant l'assemblage, pour ne pas forger deux fois", async () => {
    let finir!: (v: string) => void;
    vi.mocked(forger).mockReturnValue(new Promise<string>((ok) => (finir = ok)));
    await prepare();

    await userEvent.click(bouton("Fabriquer la voix"));

    expect(bouton("Fabriquer la voix")).toBeDisabled();
    expect(screen.getByText("assemblage et décalage…")).toBeInTheDocument();

    finir("barman.wav");
    await waitFor(() => expect(bouton("Fabriquer la voix")).toBeEnabled());
  });
});
