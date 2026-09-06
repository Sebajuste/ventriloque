// Ce que les fiches doivent tenir.
//
// L'essentiel tient dans un seul appel : `writeCharacter` recoit une fiche SANS identifiant et,
// a cote, l'identifiant d'AVANT. C'est ce qui permet de renommer un personnage sans laisser un
// doublon sur le disque, et rien dans la fenetre ne le montre.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import CharactersView from "./CharactersView";
import { deleteCharacter, writeCharacter, type Snapshot } from "../ipc";
import { aCharacter, aVoice, snapshotOf } from "../test/fixtures";

vi.mock("../ipc", async (original) => ({
  ...(await original<typeof import("../ipc")>()),
  writeCharacter: vi.fn(),
  deleteCharacter: vi.fn(),
}));

const reload = vi.fn<() => Promise<void>>();
const mount = (snapshot: Snapshot) =>
  render(<CharactersView snapshot={snapshot} reload={reload} />);

const nameField = () => screen.getByLabelText(/^Nom/);
const universeField = () => screen.getByLabelText(/^Univers/);
const linesField = () => screen.getByLabelText(/Répliques favorites/);
const button = (name: string) => screen.getByRole("button", { name });

beforeEach(() => {
  reload.mockReset().mockResolvedValue(undefined);
  vi.mocked(writeCharacter)
    .mockReset()
    .mockImplementation(async (c) => ({ ...c, id: c.name.toLowerCase().replace(/\s+/g, "_") }));
  vi.mocked(deleteCharacter).mockReset().mockResolvedValue(undefined);
});

describe("enregistrement", () => {
  it("envoie la fiche sans identifiant, et l'identifiant d'avant a cote", async () => {
    mount(snapshotOf({ characters: [aCharacter("Judy Alvarez")] }));

    await userEvent.click(screen.getByText("Judy Alvarez"));
    await userEvent.clear(nameField());
    await userEvent.type(nameField(), "Judy A.");
    await userEvent.click(button("Enregistrer"));

    await waitFor(() =>
      expect(writeCharacter).toHaveBeenCalledWith(
        expect.objectContaining({ id: "", name: "Judy A." }),
        "judy_alvarez",
      ),
    );
  });

  it("relit le disque et reprend la fiche telle que Rust l'a rendue", async () => {
    mount(snapshotOf());

    await userEvent.type(nameField(), "Le barman");
    await userEvent.click(button("Enregistrer"));

    await waitFor(() => expect(screen.getByText("enregistrée")).toBeInTheDocument());
    expect(reload).toHaveBeenCalledOnce();

    // L'identifiant attribue par Rust est revenu dans le brouillon : un second enregistrement
    // le passera en `previousId` au lieu de creer une deuxieme fiche.
    await userEvent.click(button("Enregistrer"));
    await waitFor(() =>
      expect(writeCharacter).toHaveBeenLastCalledWith(expect.anything(), "le_barman"),
    );
  });

  it("montre la panne de Rust sans rien relire", async () => {
    vi.mocked(writeCharacter).mockRejectedValue("un personnage porte deja ce nom");
    mount(snapshotOf());

    await userEvent.type(nameField(), "Judy");
    await userEvent.click(button("Enregistrer"));

    await waitFor(() =>
      expect(screen.getByText("un personnage porte deja ce nom")).toHaveClass("failed"),
    );
    expect(reload).not.toHaveBeenCalled();
  });

  it("decoupe les repliques sur les sauts de ligne", async () => {
    mount(snapshotOf());

    await userEvent.type(nameField(), "Le barman");
    await userEvent.type(linesField(), "Trois couronnes.{enter}Vous êtes pas d'ici.");
    await userEvent.click(button("Enregistrer"));

    await waitFor(() =>
      expect(writeCharacter).toHaveBeenCalledWith(
        expect.objectContaining({ lines: ["Trois couronnes.", "Vous êtes pas d'ici."] }),
        "",
      ),
    );
  });
});

describe("suppression", () => {
  it("ne supprime rien depuis un brouillon neuf", async () => {
    mount(snapshotOf());

    await userEvent.click(button("Supprimer"));

    expect(screen.getByText("rien à supprimer")).toHaveClass("failed");
    expect(deleteCharacter).not.toHaveBeenCalled();
  });

  it("vide le formulaire une fois la fiche partie", async () => {
    mount(snapshotOf({ characters: [aCharacter("Judy Alvarez")] }));

    await userEvent.click(screen.getByText("Judy Alvarez"));
    expect(nameField()).toHaveValue("Judy Alvarez");

    await userEvent.click(button("Supprimer"));

    await waitFor(() => expect(screen.getByText("supprimée")).toBeInTheDocument());
    expect(deleteCharacter).toHaveBeenCalledWith("judy_alvarez");
    expect(nameField()).toHaveValue("");
    expect(reload).toHaveBeenCalledOnce();
  });
});

describe("la liste", () => {
  it("charge une fiche dans le formulaire et efface le message d'avant", async () => {
    mount(snapshotOf({ characters: [aCharacter("Judy Alvarez", { universe: "Cyberpunk 2077" })] }));

    await userEvent.click(button("Supprimer"));
    expect(screen.getByText("rien à supprimer")).toBeInTheDocument();

    await userEvent.click(screen.getByText("Judy Alvarez"));

    expect(universeField()).toHaveValue("Cyberpunk 2077");
    expect(screen.queryByText("rien à supprimer")).not.toBeInTheDocument();
  });

  it("laisse le nom seul quand la fiche n'a pas d'univers", () => {
    const { container } = mount(
      snapshotOf({ characters: [aCharacter("Anonyme", { universe: "" })] }),
    );

    expect(container.querySelector(".list .tier")).toBeNull();
  });

  it("propose les voix connues, avec leur palier", () => {
    mount(
      snapshotOf({
        voices: [aVoice("Judy", { kind: "clone" }), aVoice("Jean", { kind: "catalog" })],
      }),
    );

    expect(screen.getByRole("option", { name: "Judy (clone)" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "Jean (catalog)" })).toBeInTheDocument();
    expect(screen.getByRole("option", { name: "— aucune —" })).toBeInTheDocument();
  });
});
