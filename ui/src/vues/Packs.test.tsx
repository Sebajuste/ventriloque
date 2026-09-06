// Ce que les paquets doivent tenir.
//
// UNE CHAINE VIDE VEUT DIRE « ANNULE », pas « rien installe ». Rust rend le compte des fichiers
// quand l'installation a eu lieu, et une chaine vide quand le selecteur a ete ferme : confondre
// les deux relirait le disque pour rien a chaque fois qu'on renonce.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import Packs from "./Packs";
import { installerPack, type Etat } from "../api";
import { etatDeTest } from "../tests/donnees";

vi.mock("../api", async (original) => ({
  ...(await original<typeof import("../api")>()),
  installerPack: vi.fn(),
}));

const relire = vi.fn<() => Promise<void>>();
const monter = (etat: Etat = etatDeTest()) => render(<Packs etat={etat} relire={relire} />);
const installer = () => screen.getByRole("button", { name: "Installer un paquet…" });

const manifeste = (sur: Partial<Etat["packs"][number]> = {}) => ({
  nom: "Voix de Night City",
  version: "1.2",
  description: "Douze voix clonées",
  auteur: "sebajuste",
  fichiers: ["voix/judy.wav", "voix/panam.wav"],
  installe_le: "2026-09-05",
  ...sur,
});

beforeEach(() => {
  relire.mockReset().mockResolvedValue(undefined);
  vi.mocked(installerPack).mockReset().mockResolvedValue("");
});

describe("installation", () => {
  it("relit le disque quand un paquet est entre", async () => {
    vi.mocked(installerPack).mockResolvedValue("Voix de Night City installe : 12 fichier(s)");
    monter();

    await userEvent.click(installer());

    await waitFor(() =>
      expect(screen.getByText("Voix de Night City installe : 12 fichier(s)")).toBeInTheDocument(),
    );
    expect(relire).toHaveBeenCalledOnce();
  });

  it("ne relit rien quand le selecteur a ete ferme", async () => {
    monter();

    await userEvent.click(installer());

    await waitFor(() => expect(screen.getByText("annulé")).toBeInTheDocument());
    expect(relire).not.toHaveBeenCalled();
  });

  it("montre la panne sans la confondre avec un abandon", async () => {
    vi.mocked(installerPack).mockRejectedValue("ce zip ne porte pas de pack.json");
    monter();

    await userEvent.click(installer());

    await waitFor(() =>
      expect(screen.getByText("ce zip ne porte pas de pack.json")).toHaveClass("rate"),
    );
    expect(relire).not.toHaveBeenCalled();
  });

  it("ferme le bouton pendant l'extraction, puis le rend", async () => {
    let finir!: (v: string) => void;
    vi.mocked(installerPack).mockReturnValue(new Promise<string>((ok) => (finir = ok)));
    monter();

    await userEvent.click(installer());
    expect(installer()).toBeDisabled();

    finir("");
    await waitFor(() => expect(installer()).toBeEnabled());
  });

  it("efface la panne d'avant des qu'on retente", async () => {
    vi.mocked(installerPack).mockRejectedValueOnce("ce zip ne porte pas de pack.json");
    monter();
    await userEvent.click(installer());
    // La chaine exacte : la note de la page contient elle aussi un `<code>pack.json</code>`,
    // et une expression trop large trouverait deux elements au lieu de zero ou un.
    await waitFor(() =>
      expect(screen.getByText("ce zip ne porte pas de pack.json")).toBeInTheDocument(),
    );

    let finir!: (v: string) => void;
    vi.mocked(installerPack).mockReturnValue(new Promise<string>((ok) => (finir = ok)));
    await userEvent.click(installer());

    expect(screen.queryByText("ce zip ne porte pas de pack.json")).not.toBeInTheDocument();
    finir("");
  });
});

describe("la liste des paquets", () => {
  it("dit le vide plutot que de ne rien dire", () => {
    monter();

    expect(screen.getByText("Aucun paquet installé.")).toBeInTheDocument();
  });

  it("porte le nom seul quand le paquet n'a pas de version", () => {
    monter(etatDeTest({ packs: [manifeste({ version: "" })] }));

    expect(screen.getByText("Voix de Night City")).toBeInTheDocument();
  });

  it("colle la version au nom quand il y en a une", () => {
    monter(etatDeTest({ packs: [manifeste()] }));

    expect(screen.getByText("Voix de Night City 1.2")).toBeInTheDocument();
  });

  it("saute les champs vides du detail au lieu de laisser des separateurs orphelins", () => {
    monter(
      etatDeTest({ packs: [manifeste({ description: "", auteur: "", installe_le: "" })] }),
    );

    expect(screen.getByText("2 fichiers")).toBeInTheDocument();
  });

  it("accorde le compte de fichiers au singulier", () => {
    monter(etatDeTest({ packs: [manifeste({ fichiers: ["voix/judy.wav"] })] }));

    expect(screen.getByText(/1 fichier ·/)).toBeInTheDocument();
  });
});
