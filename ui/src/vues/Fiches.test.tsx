// Ce que les fiches doivent tenir.
//
// L'essentiel tient dans un seul appel : `ecrire_fiche` recoit une fiche SANS identifiant et,
// a cote, l'identifiant d'AVANT. C'est ce qui permet de renommer un personnage sans laisser un
// doublon sur le disque, et rien dans la fenetre ne le montre.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import Fiches from "./Fiches";
import { ecrireFiche, supprimerFiche, type Etat } from "../api";
import { etatDeTest, fiche, voix } from "../tests/donnees";

vi.mock("../api", async (original) => ({
  ...(await original<typeof import("../api")>()),
  ecrireFiche: vi.fn(),
  supprimerFiche: vi.fn(),
}));

const relire = vi.fn<() => Promise<void>>();
const monter = (etat: Etat) => render(<Fiches etat={etat} relire={relire} />);

const champNom = () => screen.getByLabelText(/^Nom/);
const champUnivers = () => screen.getByLabelText(/^Univers/);
const champRepliques = () => screen.getByLabelText(/Répliques favorites/);
const bouton = (nom: string) => screen.getByRole("button", { name: nom });

beforeEach(() => {
  relire.mockReset().mockResolvedValue(undefined);
  vi.mocked(ecrireFiche)
    .mockReset()
    .mockImplementation(async (f) => ({ ...f, id: f.nom.toLowerCase().replace(/\s+/g, "_") }));
  vi.mocked(supprimerFiche).mockReset().mockResolvedValue(undefined);
});

describe("enregistrement", () => {
  it("envoie la fiche sans identifiant, et l'identifiant d'avant a cote", async () => {
    monter(etatDeTest({ fiches: [fiche("Judy Alvarez")] }));

    await userEvent.click(screen.getByText("Judy Alvarez"));
    await userEvent.clear(champNom());
    await userEvent.type(champNom(), "Judy A.");
    await userEvent.click(bouton("Enregistrer"));

    await waitFor(() =>
      expect(ecrireFiche).toHaveBeenCalledWith(
        expect.objectContaining({ id: "", nom: "Judy A." }),
        "judy_alvarez",
      ),
    );
  });

  it("relit le disque et reprend la fiche telle que Rust l'a rendue", async () => {
    monter(etatDeTest());

    await userEvent.type(champNom(), "Le barman");
    await userEvent.click(bouton("Enregistrer"));

    await waitFor(() => expect(screen.getByText("enregistrée")).toBeInTheDocument());
    expect(relire).toHaveBeenCalledOnce();

    // L'identifiant attribue par Rust est revenu dans le brouillon : un second enregistrement
    // le passera en `ancien` au lieu de creer une deuxieme fiche.
    await userEvent.click(bouton("Enregistrer"));
    await waitFor(() => expect(ecrireFiche).toHaveBeenLastCalledWith(expect.anything(), "le_barman"));
  });

  it("montre la panne de Rust sans rien relire", async () => {
    vi.mocked(ecrireFiche).mockRejectedValue("un personnage porte deja ce nom");
    monter(etatDeTest());

    await userEvent.type(champNom(), "Judy");
    await userEvent.click(bouton("Enregistrer"));

    await waitFor(() =>
      expect(screen.getByText("un personnage porte deja ce nom")).toHaveClass("rate"),
    );
    expect(relire).not.toHaveBeenCalled();
  });

  it("decoupe les repliques sur les sauts de ligne", async () => {
    monter(etatDeTest());

    await userEvent.type(champNom(), "Le barman");
    await userEvent.type(champRepliques(), "Trois couronnes.{enter}Vous êtes pas d'ici.");
    await userEvent.click(bouton("Enregistrer"));

    await waitFor(() =>
      expect(ecrireFiche).toHaveBeenCalledWith(
        expect.objectContaining({
          repliques: ["Trois couronnes.", "Vous êtes pas d'ici."],
        }),
        "",
      ),
    );
  });
});

describe("suppression", () => {
  it("ne supprime rien depuis un brouillon neuf", async () => {
    monter(etatDeTest());

    await userEvent.click(bouton("Supprimer"));

    expect(screen.getByText("rien à supprimer")).toHaveClass("rate");
    expect(supprimerFiche).not.toHaveBeenCalled();
  });

  it("vide le formulaire une fois la fiche partie", async () => {
    monter(etatDeTest({ fiches: [fiche("Judy Alvarez")] }));

    await userEvent.click(screen.getByText("Judy Alvarez"));
    expect(champNom()).toHaveValue("Judy Alvarez");

    await userEvent.click(bouton("Supprimer"));

    await waitFor(() => expect(screen.getByText("supprimée")).toBeInTheDocument());
    expect(supprimerFiche).toHaveBeenCalledWith("judy_alvarez");
    expect(champNom()).toHaveValue("");
    expect(relire).toHaveBeenCalledOnce();
  });
});

describe("la liste", () => {
  it("charge une fiche dans le formulaire et efface le message d'avant", async () => {
    monter(etatDeTest({ fiches: [fiche("Judy Alvarez", { univers: "Cyberpunk 2077" })] }));

    await userEvent.click(bouton("Supprimer"));
    expect(screen.getByText("rien à supprimer")).toBeInTheDocument();

    await userEvent.click(screen.getByText("Judy Alvarez"));

    expect(champUnivers()).toHaveValue("Cyberpunk 2077");
    expect(screen.queryByText("rien à supprimer")).not.toBeInTheDocument();
  });

  it("laisse le nom seul quand la fiche n'a pas d'univers", () => {
    const { container } = monter(etatDeTest({ fiches: [fiche("Anonyme", { univers: "" })] }));

    expect(container.querySelector(".liste .palier")).toBeNull();
  });

  it("propose les voix connues, avec leur palier", () => {
    monter(
      etatDeTest({
        voix: [voix("Judy", { palier: "clone" }), voix("Jean", { palier: "catalogue" })],
      }),
    );

    expect(screen.getByRole("option", { name: "Judy (clone)" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Jean (catalogue)" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "— aucune —" })).toBeInTheDocument();
  });
});
