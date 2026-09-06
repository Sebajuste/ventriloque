// Ce que la fenetre doit tenir.
//
// LE TEST QUI COMPTE EST CELUI DES ONGLETS. Une version precedente posait `hidden` sur chaque
// vue, et une regle `main { display: flex }` l'emportait sur le `display: none` de l'attribut :
// les quatre pages s'empilaient sur une seule. `not.toBeInTheDocument()` est ecrit exactement
// pour ca -- une vue seulement cachee serait encore dans le document et le test tomberait.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import App from "./App";
import { choisirPeripherique, lireEtat } from "./api";
import { etatDeTest, fiche, voix } from "./tests/donnees";

vi.mock("./api", async (original) => ({
  ...(await original<typeof import("./api")>()),
  lireEtat: vi.fn(),
  choisirPeripherique: vi.fn(),
  parler: vi.fn(),
  taire: vi.fn(),
  prechauffer: vi.fn(),
  choisirFichiers: vi.fn(),
  forger: vi.fn(),
  ecrireFiche: vi.fn(),
  supprimerFiche: vi.fn(),
  installerPack: vi.fn(),
}));

const onglet = (nom: string) => screen.getByRole("button", { name: nom });

/** Un element propre a chaque vue, pour savoir laquelle est reellement montee. */
const marqueurs = {
  player: () => screen.queryByPlaceholderText(/Ce que dit le PNJ/),
  fiches: () => screen.queryByRole("button", { name: "Nouveau personnage" }),
  // Par le texte : ce bouton vit dans un `<label>`, qui lui vole son nom accessible.
  atelier: () => screen.queryByText("Choisir des fichiers…"),
  packs: () => screen.queryByRole("button", { name: "Installer un paquet…" }),
};

beforeEach(() => {
  vi.mocked(lireEtat).mockReset().mockResolvedValue(etatDeTest());
  vi.mocked(choisirPeripherique).mockReset().mockResolvedValue(undefined);
});

describe("demarrage", () => {
  it("lit l'etat du disque une fois, et ouvre sur le player", async () => {
    render(<App />);

    await waitFor(() => expect(lireEtat).toHaveBeenCalledOnce());
    expect(marqueurs.player()).toBeInTheDocument();
  });

  it("tient sans rien casser avant que le disque ait repondu", () => {
    vi.mocked(lireEtat).mockReturnValue(new Promise(() => {}));

    render(<App />);

    expect(marqueurs.player()).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Ventriloque" })).toBeInTheDocument();
  });
});

describe("onglets", () => {
  it("l'onglet inactif n'est pas cache, il n'existe pas", async () => {
    render(<App />);
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());
    expect(marqueurs.player()).toBeInTheDocument();

    await userEvent.click(onglet("Fiches"));

    expect(marqueurs.fiches()).toBeInTheDocument();
    expect(marqueurs.player()).not.toBeInTheDocument();
    expect(marqueurs.atelier()).not.toBeInTheDocument();
    expect(marqueurs.packs()).not.toBeInTheDocument();
  });

  it("n'en monte jamais qu'un, quel que soit le chemin parcouru", async () => {
    render(<App />);
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());

    for (const nom of ["Atelier", "Packs", "Fiches", "Player", "Packs"]) {
      await userEvent.click(onglet(nom));
      const montes = Object.values(marqueurs).filter((m) => m() !== null);
      expect(montes).toHaveLength(1);
    }
  });

  it("marque l'onglet courant, et lui seul", async () => {
    render(<App />);
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());

    await userEvent.click(onglet("Atelier"));

    expect(onglet("Atelier")).toHaveClass("onglet", "actif");
    expect(onglet("Player")).toHaveClass("onglet");
    expect(onglet("Player")).not.toHaveClass("actif");
  });
});

describe("panne du moteur", () => {
  it("se tait quand tout va bien", async () => {
    render(<App />);
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());

    expect(screen.queryByText(/n'a pas démarré/)).not.toBeInTheDocument();
  });

  it("dit la panne ET ou la reparer", async () => {
    vi.mocked(lireEtat).mockResolvedValue(
      etatDeTest({ pret: false, panne: "modèles introuvables" }),
    );
    render(<App />);

    await waitFor(() =>
      expect(screen.getByText(/modèles introuvables/)).toBeInTheDocument(),
    );
    expect(screen.getByText(/onglet Packs/)).toBeInTheDocument();
  });
});

describe("sortie audio", () => {
  it("liste les peripheriques de la machine et marque celui qui sert", async () => {
    vi.mocked(lireEtat).mockResolvedValue(
      etatDeTest({ peripheriques: ["Haut-parleurs", "Casque USB"], peripherique: 1 }),
    );
    render(<App />);

    await waitFor(() => expect(screen.getByLabelText("Sortie")).toHaveValue("1"));
  });

  it("bouge tout de suite sans attendre Rust, puis le previent", async () => {
    vi.mocked(lireEtat).mockResolvedValue(
      etatDeTest({ peripheriques: ["Haut-parleurs", "Casque USB"], peripherique: 0 }),
    );
    // Rust ne repond jamais : la fenetre doit avoir bouge quand meme.
    vi.mocked(choisirPeripherique).mockReturnValue(new Promise(() => {}));
    render(<App />);
    await waitFor(() => expect(screen.getByLabelText("Sortie")).toBeInTheDocument());

    await userEvent.selectOptions(screen.getByLabelText("Sortie"), "1");

    expect(screen.getByLabelText("Sortie")).toHaveValue("1");
    expect(choisirPeripherique).toHaveBeenCalledWith(1);
  });
});

describe("l'etat est relu, pas tenu", () => {
  it("une fiche enregistree passe par le disque avant de revenir dans la fenetre", async () => {
    const { ecrireFiche } = await import("./api");
    vi.mocked(ecrireFiche).mockResolvedValue(fiche("Le barman"));
    vi.mocked(lireEtat)
      .mockResolvedValueOnce(etatDeTest({ voix: [voix("Judy")] }))
      .mockResolvedValue(etatDeTest({ voix: [voix("Judy")], fiches: [fiche("Le barman")] }));

    render(<App />);
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());
    await userEvent.click(onglet("Fiches"));

    await userEvent.type(screen.getByLabelText(/^Nom/), "Le barman");
    await userEvent.click(screen.getByRole("button", { name: "Enregistrer" }));

    // La fiche n'apparait pas parce que la vue l'a ajoutee a une liste locale, mais parce que
    // le disque a ete relu : c'est ce qui permet de deposer un .json a la main dans `pnj/`.
    await waitFor(() => expect(lireEtat).toHaveBeenCalledTimes(2));
    expect(await screen.findByText("Le barman")).toBeInTheDocument();
  });
});
