// Ce que la fenetre doit tenir, routeur compris.
//
// LE TEST QUI COMPTE EST CELUI DES PAGES. Une version precedente posait `hidden` sur chaque vue,
// et une regle `main { display: flex }` l'emportait sur le `display: none` de l'attribut : les
// quatre pages s'empilaient sur une seule. `not.toBeInTheDocument()` est ecrit exactement pour
// ca -- une vue seulement cachee serait encore dans le document et le test tomberait.
//
// On monte `Routage` et pas `Fenetre` : c'est le vrai arbre de routes qui doit etre eprouve, pas
// une copie ecrite dans le test. Le `MemoryRouter` part d'ou l'on veut et ne laisse pas d'adresse
// derriere lui d'un test a l'autre, la ou le `HashRouter` de `main.tsx` les ferait fuiter.

import { act } from "react";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { MemoryRouter, useNavigate } from "react-router";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { Routage } from "./routes";
import { choisirFichiers, choisirPeripherique, forger, lireEtat } from "./api";
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

/** Donne au test la main sur l'historique du routeur monte, sans en construire un deuxieme. */
let aller: ReturnType<typeof useNavigate>;
function Sonde() {
  aller = useNavigate();
  return null;
}

const monter = (depart = "/") =>
  render(
    <MemoryRouter initialEntries={[depart]}>
      <Sonde />
      <Routage />
    </MemoryRouter>,
  );

const onglet = (nom: string) => screen.getByRole("link", { name: nom });

/** Un element propre a chaque page, pour savoir laquelle est reellement montee. */
const marqueurs = {
  player: () => screen.queryByPlaceholderText(/Ce que dit le PNJ/),
  fiches: () => screen.queryByRole("button", { name: "Nouveau personnage" }),
  // Par le texte : ce bouton vit dans un `<label>`, qui lui vole son nom accessible.
  atelier: () => screen.queryByText("Choisir des fichiers…"),
  packs: () => screen.queryByRole("button", { name: "Installer un paquet…" }),
};

const seuleMontee = () => Object.entries(marqueurs).filter(([, m]) => m() !== null).map(([n]) => n);

beforeEach(() => {
  vi.mocked(lireEtat).mockReset().mockResolvedValue(etatDeTest());
  vi.mocked(choisirPeripherique).mockReset().mockResolvedValue(undefined);
  vi.mocked(choisirFichiers).mockReset().mockResolvedValue([]);
  vi.mocked(forger).mockReset().mockResolvedValue("barman.wav");
});

describe("demarrage", () => {
  it("lit l'etat du disque une fois, et ouvre sur le player", async () => {
    monter();

    await waitFor(() => expect(lireEtat).toHaveBeenCalledOnce());
    expect(marqueurs.player()).toBeInTheDocument();
  });

  it("tient sans rien casser avant que le disque ait repondu", () => {
    vi.mocked(lireEtat).mockReturnValue(new Promise(() => {}));

    monter();

    expect(marqueurs.player()).toBeInTheDocument();
    expect(screen.getByRole("heading", { name: "Ventriloque" })).toBeInTheDocument();
  });
});

describe("routes", () => {
  it("ouvre directement la page demandee par l'adresse", async () => {
    monter("/atelier");

    await waitFor(() => expect(lireEtat).toHaveBeenCalled());
    expect(seuleMontee()).toEqual(["atelier"]);
  });

  it("ramene au player plutot que de laisser une fenetre vide", async () => {
    monter("/il-n-y-a-rien-ici");

    await waitFor(() => expect(lireEtat).toHaveBeenCalled());
    expect(seuleMontee()).toEqual(["player"]);
  });

  it("la page inactive n'est pas cachee, elle n'existe pas", async () => {
    monter();
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());
    expect(marqueurs.player()).toBeInTheDocument();

    await userEvent.click(onglet("Fiches"));

    expect(seuleMontee()).toEqual(["fiches"]);
  });

  it("n'en monte jamais qu'une, quel que soit le chemin parcouru", async () => {
    monter();
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());

    for (const nom of ["Atelier", "Packs", "Fiches", "Player", "Packs"]) {
      await userEvent.click(onglet(nom));
      expect(seuleMontee()).toHaveLength(1);
    }
  });

  it("revient sur ses pas, et repart en avant", async () => {
    monter();
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());

    await userEvent.click(onglet("Fiches"));
    await userEvent.click(onglet("Packs"));
    expect(seuleMontee()).toEqual(["packs"]);

    await act(async () => void aller(-1));
    expect(seuleMontee()).toEqual(["fiches"]);

    await act(async () => void aller(-1));
    expect(seuleMontee()).toEqual(["player"]);

    await act(async () => void aller(1));
    expect(seuleMontee()).toEqual(["fiches"]);
  });

  it("marque l'onglet courant, et lui seul", async () => {
    monter("/atelier");
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());

    expect(onglet("Atelier")).toHaveClass("onglet", "actif");
    expect(onglet("Player")).toHaveClass("onglet");
    expect(onglet("Player")).not.toHaveClass("actif");
  });

  it("ne marque « Player » que sur le player, bien qu'il soit la racine", async () => {
    // Ce test tient sans `end` : react-router exige que le caractere suivant le prefixe soit un
    // `/`, donc « / » ne marque pas « /packs ». Il est ici pour la racine elle-meme, qui est le
    // seul onglet dont l'adresse prefixe toutes les autres.
    monter("/packs");
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());

    expect(onglet("Player")).not.toHaveClass("actif");
    expect(onglet("Packs")).toHaveClass("actif");
  });

  it("garde l'en-tete d'une page a l'autre", async () => {
    monter();
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());

    await userEvent.click(onglet("Packs"));

    expect(screen.getByRole("heading", { name: "Ventriloque" })).toBeInTheDocument();
    expect(screen.getByLabelText("Sortie")).toBeInTheDocument();
    // Le disque n'est pas relu a chaque page : changer d'onglet n'est pas recharger.
    expect(lireEtat).toHaveBeenCalledOnce();
  });
});

describe("ce qui traverse les routes", () => {
  it("une voix forgee a l'atelier est deja choisie en arrivant au player", async () => {
    vi.mocked(choisirFichiers).mockResolvedValue(["D:/sons/a.wav"]);
    monter("/atelier");
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());

    await userEvent.type(screen.getByLabelText(/Nom de la voix/), "barman");
    await userEvent.click(screen.getByText("Choisir des fichiers…"));
    await waitFor(() => expect(screen.getByText("1 fichier")).toBeInTheDocument());
    await userEvent.click(screen.getByRole("button", { name: "Fabriquer la voix" }));
    await waitFor(() => expect(forger).toHaveBeenCalled());

    await userEvent.click(onglet("Player"));

    // C'est ce qui fait de « forger, entendre, corriger » un geste et pas une navigation.
    expect(screen.getByText("barman")).toBeInTheDocument();
    expect(screen.getByText(/voix clonée/)).toBeInTheDocument();
  });
});

describe("panne du moteur", () => {
  it("se tait quand tout va bien", async () => {
    monter();
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());

    expect(screen.queryByText(/n'a pas démarré/)).not.toBeInTheDocument();
  });

  it("dit la panne ET ou la reparer, sur toutes les pages", async () => {
    vi.mocked(lireEtat).mockResolvedValue(
      etatDeTest({ pret: false, panne: "modèles introuvables" }),
    );
    monter();

    await waitFor(() => expect(screen.getByText(/modèles introuvables/)).toBeInTheDocument());
    expect(screen.getByText(/onglet Packs/)).toBeInTheDocument();

    await userEvent.click(onglet("Atelier"));
    expect(screen.getByText(/modèles introuvables/)).toBeInTheDocument();
  });
});

describe("sortie audio", () => {
  it("liste les peripheriques de la machine et marque celui qui sert", async () => {
    vi.mocked(lireEtat).mockResolvedValue(
      etatDeTest({ peripheriques: ["Haut-parleurs", "Casque USB"], peripherique: 1 }),
    );
    monter();

    await waitFor(() => expect(screen.getByLabelText("Sortie")).toHaveValue("1"));
  });

  it("bouge tout de suite sans attendre Rust, puis le previent", async () => {
    vi.mocked(lireEtat).mockResolvedValue(
      etatDeTest({ peripheriques: ["Haut-parleurs", "Casque USB"], peripherique: 0 }),
    );
    // Rust ne repond jamais : la fenetre doit avoir bouge quand meme.
    vi.mocked(choisirPeripherique).mockReturnValue(new Promise(() => {}));
    monter();
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

    monter("/fiches");
    await waitFor(() => expect(lireEtat).toHaveBeenCalled());

    await userEvent.type(screen.getByLabelText(/^Nom/), "Le barman");
    await userEvent.click(screen.getByRole("button", { name: "Enregistrer" }));

    // La fiche n'apparait pas parce que la vue l'a ajoutee a une liste locale, mais parce que
    // le disque a ete relu : c'est ce qui permet de deposer un .json a la main dans `pnj/`.
    await waitFor(() => expect(lireEtat).toHaveBeenCalledTimes(2));
    expect(await screen.findByText("Le barman")).toBeInTheDocument();
  });
});
