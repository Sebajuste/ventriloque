// Ce que la fenetre doit tenir, routeur compris.
//
// LE TEST QUI COMPTE EST CELUI DES PAGES. Une version precedente posait `hidden` sur chaque vue,
// et une regle `main { display: flex }` l'emportait sur le `display: none` de l'attribut : les
// quatre pages s'empilaient sur une seule. `not.toBeInTheDocument()` est ecrit exactement pour
// ca -- une vue seulement cachee serait encore dans le document et le test tomberait.
//
// On monte `AppRoutes` et pas `AppShell` : c'est le vrai arbre de routes qui doit etre eprouve,
// pas une copie ecrite dans le test. Le `MemoryRouter` part d'ou l'on veut et ne laisse pas
// d'adresse derriere lui d'un test a l'autre, la ou le `HashRouter` de `main.tsx` les ferait
// fuiter.

import { act } from "react";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useNavigate } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { AppRoutes } from "./routes";
import { forgeVoice, pickAudioFiles, readSnapshot, selectDevice, writeCharacter } from "./ipc";
import { aCharacter, aVoice, snapshotOf } from "./test/fixtures";

vi.mock("./ipc", async (original) => ({
  ...(await original<typeof import("./ipc")>()),
  readSnapshot: vi.fn(),
  selectDevice: vi.fn(),
  speak: vi.fn(),
  silence: vi.fn(),
  warmUp: vi.fn(),
  pickAudioFiles: vi.fn(),
  forgeVoice: vi.fn(),
  writeCharacter: vi.fn(),
  deleteCharacter: vi.fn(),
  installPack: vi.fn(),
}));

/** Donne au test la main sur l'historique du routeur monte, sans en construire un deuxieme. */
let go: ReturnType<typeof useNavigate>;
function Probe() {
  go = useNavigate();
  return null;
}

const mount = (from = "/") =>
  render(
    <MemoryRouter initialEntries={[from]}>
      <Probe />
      <AppRoutes />
    </MemoryRouter>,
  );

const tab = (name: string) => screen.getByRole("link", { name });

/** Un element propre a chaque page, pour savoir laquelle est reellement montee. */
const markers = {
  player: () => screen.queryByPlaceholderText(/Ce que dit le PNJ/),
  characters: () => screen.queryByRole("button", { name: "Nouveau personnage" }),
  // Par le texte : ce bouton vit dans un `<label>`, qui lui vole son nom accessible.
  workshop: () => screen.queryByText("Choisir des fichiers…"),
  packs: () => screen.queryByRole("button", { name: "Installer un paquet…" }),
};

const mounted = () =>
  Object.entries(markers)
    .filter(([, marker]) => marker() !== null)
    .map(([name]) => name);

beforeEach(() => {
  vi.mocked(readSnapshot).mockReset().mockResolvedValue(snapshotOf());
  vi.mocked(selectDevice).mockReset().mockResolvedValue(undefined);
  vi.mocked(pickAudioFiles).mockReset().mockResolvedValue([]);
  vi.mocked(forgeVoice).mockReset().mockResolvedValue("barman.wav");
});

describe("demarrage", () => {
  it("lit l'etat du disque une fois, et ouvre sur le player", async () => {
    mount();

    await waitFor(() => expect(readSnapshot).toHaveBeenCalledOnce());
    expect(markers.player()).toBeInTheDocument();
  });

  it("tient sans rien casser avant que le disque ait repondu", () => {
    vi.mocked(readSnapshot).mockReturnValue(new Promise(() => {}));

    mount();

    expect(markers.player()).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Ventriloque" })).toBeInTheDocument();
  });
});

describe("routes", () => {
  it("ouvre directement la page demandee par l'adresse", async () => {
    mount("/atelier");

    await waitFor(() => expect(readSnapshot).toHaveBeenCalled());
    expect(mounted()).toEqual(["workshop"]);
  });

  it("ramene au player plutot que de laisser une fenetre vide", async () => {
    mount("/il-n-y-a-rien-ici");

    await waitFor(() => expect(readSnapshot).toHaveBeenCalled());
    expect(mounted()).toEqual(["player"]);
  });

  it("la page inactive n'est pas cachee, elle n'existe pas", async () => {
    mount();
    await waitFor(() => expect(readSnapshot).toHaveBeenCalled());
    expect(markers.player()).toBeInTheDocument();

    await userEvent.click(tab("Fiches"));

    expect(mounted()).toEqual(["characters"]);
  });

  it("n'en monte jamais qu'une, quel que soit le chemin parcouru", async () => {
    mount();
    await waitFor(() => expect(readSnapshot).toHaveBeenCalled());

    for (const name of ["Atelier", "Packs", "Fiches", "Player", "Packs"]) {
      await userEvent.click(tab(name));
      expect(mounted()).toHaveLength(1);
    }
  });

  it("revient sur ses pas, et repart en avant", async () => {
    mount();
    await waitFor(() => expect(readSnapshot).toHaveBeenCalled());

    await userEvent.click(tab("Fiches"));
    await userEvent.click(tab("Packs"));
    expect(mounted()).toEqual(["packs"]);

    await act(async () => void go(-1));
    expect(mounted()).toEqual(["characters"]);

    await act(async () => void go(-1));
    expect(mounted()).toEqual(["player"]);

    await act(async () => void go(1));
    expect(mounted()).toEqual(["characters"]);
  });

  it("marque l'onglet courant, et lui seul", async () => {
    mount("/atelier");
    await waitFor(() => expect(readSnapshot).toHaveBeenCalled());

    expect(tab("Atelier")).toHaveClass("tab", "active");
    expect(tab("Player")).toHaveClass("tab");
    expect(tab("Player")).not.toHaveClass("active");
  });

  it("ne marque « Player » que sur le player, bien qu'il soit la racine", async () => {
    // Ce test tient sans `end` : react-router exige que le caractere suivant le prefixe soit un
    // `/`, donc « / » ne marque pas « /packs ». Il est ici pour la racine elle-meme, qui est le
    // seul onglet dont l'adresse prefixe toutes les autres.
    mount("/packs");
    await waitFor(() => expect(readSnapshot).toHaveBeenCalled());

    expect(tab("Player")).not.toHaveClass("active");
    expect(tab("Packs")).toHaveClass("active");
  });

  it("garde l'en-tete d'une page a l'autre", async () => {
    mount();
    await waitFor(() => expect(readSnapshot).toHaveBeenCalled());

    await userEvent.click(tab("Packs"));

    expect(screen.getByRole("heading", { name: "Ventriloque" })).toBeInTheDocument();
    expect(screen.getByLabelText("Sortie")).toBeInTheDocument();
    // Le disque n'est pas relu a chaque page : changer d'onglet n'est pas recharger.
    expect(readSnapshot).toHaveBeenCalledOnce();
  });
});

describe("ce qui traverse les routes", () => {
  it("une voix forgee a l'atelier est deja choisie en arrivant au player", async () => {
    vi.mocked(pickAudioFiles).mockResolvedValue(["D:/sons/a.wav"]);
    mount("/atelier");
    await waitFor(() => expect(readSnapshot).toHaveBeenCalled());

    await userEvent.type(screen.getByLabelText(/Nom de la voix/), "barman");
    await userEvent.click(screen.getByText("Choisir des fichiers…"));
    await waitFor(() => expect(screen.getByText("1 fichier")).toBeInTheDocument());
    await userEvent.click(screen.getByRole("button", { name: "Fabriquer la voix" }));
    await waitFor(() => expect(forgeVoice).toHaveBeenCalled());

    await userEvent.click(tab("Player"));

    // C'est ce qui fait de « forger, entendre, corriger » un geste et pas une navigation.
    expect(screen.getByText("barman")).toBeInTheDocument();
    expect(screen.getByText(/voix clonée/)).toBeInTheDocument();
  });
});

describe("panne du moteur", () => {
  it("se tait quand tout va bien", async () => {
    mount();
    await waitFor(() => expect(readSnapshot).toHaveBeenCalled());

    expect(screen.queryByText(/n'a pas démarré/)).not.toBeInTheDocument();
  });

  it("dit la panne ET ou la reparer, sur toutes les pages", async () => {
    vi.mocked(readSnapshot).mockResolvedValue(
      snapshotOf({ ready: false, failure: "modèles introuvables" }),
    );
    mount();

    await waitFor(() => expect(screen.getByText(/modèles introuvables/)).toBeInTheDocument());
    expect(screen.getByText(/onglet Packs/)).toBeInTheDocument();

    await userEvent.click(tab("Atelier"));
    expect(screen.getByText(/modèles introuvables/)).toBeInTheDocument();
  });
});

describe("sortie audio", () => {
  it("liste les peripheriques de la machine et marque celui qui sert", async () => {
    vi.mocked(readSnapshot).mockResolvedValue(
      snapshotOf({ devices: ["Haut-parleurs", "Casque USB"], device: 1 }),
    );
    mount();

    await waitFor(() => expect(screen.getByLabelText("Sortie")).toHaveValue("1"));
  });

  it("bouge tout de suite sans attendre Rust, puis le previent", async () => {
    vi.mocked(readSnapshot).mockResolvedValue(
      snapshotOf({ devices: ["Haut-parleurs", "Casque USB"], device: 0 }),
    );
    // Rust ne repond jamais : la fenetre doit avoir bouge quand meme.
    vi.mocked(selectDevice).mockReturnValue(new Promise(() => {}));
    mount();
    await waitFor(() => expect(screen.getByLabelText("Sortie")).toBeInTheDocument());

    await userEvent.selectOptions(screen.getByLabelText("Sortie"), "1");

    expect(screen.getByLabelText("Sortie")).toHaveValue("1");
    expect(selectDevice).toHaveBeenCalledWith(1);
  });
});

describe("l'etat est relu, pas tenu", () => {
  it("une fiche enregistree passe par le disque avant de revenir dans la fenetre", async () => {
    vi.mocked(writeCharacter).mockResolvedValue(aCharacter("Le barman"));
    vi.mocked(readSnapshot)
      .mockResolvedValueOnce(snapshotOf({ voices: [aVoice("Judy")] }))
      .mockResolvedValue(
        snapshotOf({ voices: [aVoice("Judy")], characters: [aCharacter("Le barman")] }),
      );

    mount("/fiches");
    await waitFor(() => expect(readSnapshot).toHaveBeenCalled());

    await userEvent.type(screen.getByLabelText(/^Nom/), "Le barman");
    await userEvent.click(screen.getByRole("button", { name: "Enregistrer" }));

    // La fiche n'apparait pas parce que la vue l'a ajoutee a une liste locale, mais parce que
    // le disque a ete relu : c'est ce qui permet de deposer un .json a la main dans
    // `characters/`.
    await waitFor(() => expect(readSnapshot).toHaveBeenCalledTimes(2));
    expect(await screen.findByText("Le barman")).toBeInTheDocument();
  });
});
