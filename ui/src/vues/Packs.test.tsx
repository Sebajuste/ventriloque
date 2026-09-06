// Ce que les paquets doivent tenir.
//
// UNE CHAINE VIDE VEUT DIRE « ANNULE », pas « rien installe ». Rust rend le compte des fichiers
// quand l'installation a eu lieu, et une chaine vide quand le selecteur a ete ferme : confondre
// les deux relirait le disque pour rien a chaque fois qu'on renonce.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import Packs from "./Packs";
import {
  avancementPack,
  desinstallerPack,
  fabriquerPack,
  installerPack,
  type AvancePack,
  type Etat,
} from "../api";
import { etatDeTest } from "../tests/donnees";

vi.mock("../api", async (original) => ({
  ...(await original<typeof import("../api")>()),
  installerPack: vi.fn(),
  fabriquerPack: vi.fn(),
  desinstallerPack: vi.fn(),
  avancementPack: vi.fn(),
}));

const REPOS: AvancePack = { actif: false, faits: 0, total: 0, quoi: "", journal: [] };

const relire = vi.fn<() => Promise<void>>();
const monter = (etat: Etat = etatDeTest()) => render(<Packs etat={etat} relire={relire} />);
const installer = () => screen.getByRole("button", { name: "Installer un paquet…" });

const manifeste = (sur: Partial<Etat["packs"][number]> = {}) => ({
  nom: "Voix de Night City",
  version: "1.2",
  description: "Douze voix clonées",
  auteur: "sebajuste",
  recette: "",
  jeu: "",
  produit: "",
  fichiers: ["voix/judy.wav", "voix/panam.wav"],
  installe_le: "2026-09-05",
  fabrique_le: "",
  ...sur,
});

// Un paquet-recette : il n'apporte aucune voix tant qu'il n'a pas tourne.
const aFabriquer = (sur: Partial<Etat["packs"][number]> = {}) =>
  manifeste({
    nom: "StarCraft II",
    recette: "starcraft2.rhai",
    jeu: "StarCraft II",
    produit: "sc2",
    fichiers: [],
    ...sur,
  });

beforeEach(() => {
  relire.mockReset().mockResolvedValue(undefined);
  vi.mocked(installerPack).mockReset().mockResolvedValue("");
  vi.mocked(fabriquerPack).mockReset().mockResolvedValue("");
  vi.mocked(desinstallerPack).mockReset().mockResolvedValue("");
  vi.mocked(avancementPack).mockReset().mockResolvedValue(REPOS);
});

// LES COMMANDES NE RENDENT LA MAIN QU'À LA FIN. Une installation de 258 Mo n'a donc rien à dire
// pendant une minute si personne ne bat la mesure : c'est ce battement qui fait vivre la barre.
describe("l'avancement", () => {
  const enCours = () => screen.getByRole("progressbar");

  it("ne montre aucune barre au repos", () => {
    monter(etatDeTest({ packs: [manifeste()] }));

    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("montre une proportion quand le total est connu", async () => {
    let finir!: (v: string) => void;
    vi.mocked(installerPack).mockReturnValue(new Promise<string>((ok) => (finir = ok)));
    vi.mocked(avancementPack).mockResolvedValue({
      actif: true,
      faits: 3,
      total: 6,
      quoi: "voix/judy.wav",
      journal: [],
    });
    monter();

    await userEvent.click(installer());

    await waitFor(() => expect(screen.getByText("voix/judy.wav")).toBeInTheDocument());
    expect(enCours()).toHaveAttribute("aria-valuenow", "3");
    expect(enCours()).toHaveAttribute("aria-valuemax", "6");
    expect(screen.getByText("installation — 50 %")).toBeInTheDocument();

    finir("");
  });

  // Une fabrication ne compte pas ses étapes d'avance : donner un pourcentage serait mentir.
  it("s'abstient de proportion quand le total est inconnu", async () => {
    let finir!: (v: string) => void;
    vi.mocked(fabriquerPack).mockReturnValue(new Promise<string>((ok) => (finir = ok)));
    vi.mocked(avancementPack).mockResolvedValue({
      actif: true,
      faits: 0,
      total: 0,
      quoi: "recensement : 779862 entrees",
      journal: ["stockage ouvert : produit s2"],
    });
    monter(etatDeTest({ packs: [aFabriquer()] }));

    await userEvent.click(screen.getByRole("button", { name: "Fabriquer…" }));

    await waitFor(() =>
      expect(screen.getByText("recensement : 779862 entrees")).toBeInTheDocument(),
    );
    expect(enCours()).not.toHaveAttribute("aria-valuenow");
    expect(screen.getByText("fabrication en cours, quelques minutes…")).toBeInTheDocument();

    finir("");
  });

  it("montre le journal pendant l'opération, sans attendre la fin", async () => {
    let finir!: (v: string) => void;
    vi.mocked(fabriquerPack).mockReturnValue(new Promise<string>((ok) => (finir = ok)));
    vi.mocked(avancementPack).mockResolvedValue({
      actif: true,
      faits: 0,
      total: 0,
      quoi: "sc2_nova.wav — 36.3s",
      journal: ["stockage ouvert : produit s2", "recensement : 779862 entrees"],
    });
    monter(etatDeTest({ packs: [aFabriquer()] }));

    await userEvent.click(screen.getByRole("button", { name: "Fabriquer…" }));

    await waitFor(() => expect(screen.getByText(/recensement/)).toHaveClass("journal"));
    expect(screen.getByText(/stockage ouvert/)).toBeInTheDocument();

    finir("");
  });

  // Le journal en direct disparaît quand le compte rendu arrive : deux blocs pour la même chose
  // se seraient contredits, l'un figé sur l'avant-dernière ligne.
  it("cède la place au compte rendu une fois l'opération finie", async () => {
    vi.mocked(fabriquerPack).mockResolvedValue("sc2_nova.wav — 36.3s\nsc2_karax.wav — 35.0s");
    monter(etatDeTest({ packs: [aFabriquer()] }));

    await userEvent.click(screen.getByRole("button", { name: "Fabriquer…" }));

    await waitFor(() => expect(screen.getByText(/sc2_karax/)).toHaveClass("journal"));
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("range la barre dès que l'opération rend la main", async () => {
    let finir!: (v: string) => void;
    vi.mocked(installerPack).mockReturnValue(new Promise<string>((ok) => (finir = ok)));
    vi.mocked(avancementPack).mockResolvedValue({
      actif: true,
      faits: 1,
      total: 2,
      quoi: "x",
      journal: [],
    });
    monter();

    await userEvent.click(installer());
    await waitFor(() => expect(enCours()).toBeInTheDocument());

    finir("essai installe : 2 fichier(s)");

    await waitFor(() => expect(screen.queryByRole("progressbar")).not.toBeInTheDocument());
  });
});

// UN CLIC N'EFFACE RIEN. Le premier arme, le second efface : une boîte de dialogue de plus se
// clique sans la lire, un bouton qui change de texte se voit.
describe("le retrait", () => {
  const retirer = () => screen.getByRole("button", { name: "Retirer…" });
  const confirmer = () => screen.getByRole("button", { name: "Confirmer le retrait" });

  it("n'appelle rien au premier clic", async () => {
    monter(etatDeTest({ packs: [manifeste()] }));

    await userEvent.click(retirer());

    expect(desinstallerPack).not.toHaveBeenCalled();
    expect(confirmer()).toBeInTheDocument();
    expect(screen.getByText("2 fichiers seront effacés")).toBeInTheDocument();
  });

  it("efface au second clic, puis relit le disque", async () => {
    vi.mocked(desinstallerPack).mockResolvedValue("Voix de Night City retiré : 18 fichier(s) effacé(s)");
    monter(etatDeTest({ packs: [manifeste()] }));

    await userEvent.click(retirer());
    await userEvent.click(confirmer());

    await waitFor(() =>
      expect(screen.getByText("Voix de Night City retiré : 18 fichier(s) effacé(s)")).toBeInTheDocument(),
    );
    expect(vi.mocked(desinstallerPack)).toHaveBeenCalledWith("Voix de Night City");
    expect(relire).toHaveBeenCalledOnce();
  });

  // Retirer les modèles rend l'application muette : ça se dit avant, pas après.
  it("prévient quand le paquet porte les modèles", async () => {
    monter(etatDeTest({ packs: [manifeste({ fichiers: ["modeles/flow_lm_main_int8.onnx"] })] }));

    await userEvent.click(retirer());

    expect(
      screen.getByText("ceci retire les modèles : Ventriloque redeviendra muet"),
    ).toBeInTheDocument();
  });

  it("désarme dès qu'un autre geste commence", async () => {
    monter(etatDeTest({ packs: [manifeste()] }));
    await userEvent.click(retirer());

    await userEvent.click(installer());

    await waitFor(() => expect(retirer()).toBeInTheDocument());
    expect(screen.queryByRole("button", { name: "Confirmer le retrait" })).not.toBeInTheDocument();
  });
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

// Un script venu d'ailleurs ne tourne que sur un geste. L'onglet doit donc dire clairement
// lequel des deux boutons fait quoi, et ne jamais proposer de fabriquer un paquet sans recette.
describe("les paquets-recettes", () => {
  const fabriquer = () => screen.getByRole("button", { name: /abriquer…$/ });

  it("ne propose rien a fabriquer pour un paquet qui porte deja ses voix", () => {
    monter(etatDeTest({ packs: [manifeste()] }));

    expect(screen.queryByRole("button", { name: /abriquer…$/ })).not.toBeInTheDocument();
  });

  it("nomme le jeu attendu tant que la recette n'a pas tourne", () => {
    monter(etatDeTest({ packs: [aFabriquer()] }));

    expect(fabriquer()).toHaveTextContent("Fabriquer…");
    expect(screen.getByText("à fabriquer depuis une copie de StarCraft II")).toBeInTheDocument();
  });

  it("propose de refaire une fabrication deja passee, en disant sa date", () => {
    monter(etatDeTest({ packs: [aFabriquer({ fabrique_le: "2026-09-06" })] }));

    expect(fabriquer()).toHaveTextContent("Refabriquer…");
    expect(screen.getByText("fabriqué le 2026-09-06")).toBeInTheDocument();
  });

  // Le dossier du jeu se cherche tout seul : `false` veut dire « ne demande rien tant que la
  // detection trouve l'installation ».
  it("laisse Rust chercher le dossier du jeu", async () => {
    monter(etatDeTest({ packs: [aFabriquer()] }));

    await userEvent.click(fabriquer());

    await waitFor(() => expect(fabriquerPack).toHaveBeenCalledWith("StarCraft II", false));
  });

  // La sortie de secours : deux installations du meme jeu, ou aucune trouvee.
  it("force le selecteur quand on demande un autre dossier", async () => {
    monter(etatDeTest({ packs: [aFabriquer()] }));

    await userEvent.click(screen.getByRole("button", { name: "Autre dossier…" }));

    await waitFor(() => expect(fabriquerPack).toHaveBeenCalledWith("StarCraft II", true));
  });

  it("relit le disque quand la fabrication a pose quelque chose", async () => {
    vi.mocked(fabriquerPack).mockResolvedValue("20 voix, 20 fiches");
    monter(etatDeTest({ packs: [aFabriquer()] }));

    await userEvent.click(fabriquer());

    await waitFor(() => expect(screen.getByText("20 voix, 20 fiches")).toBeInTheDocument());
    expect(vi.mocked(fabriquerPack)).toHaveBeenCalledWith("StarCraft II", false);
    expect(relire).toHaveBeenCalledOnce();
  });

  // Le journal fait vingt lignes : il descend sous la liste plutot que dans la barre d'etat.
  it("met un compte rendu multiligne dans le journal", async () => {
    vi.mocked(fabriquerPack).mockResolvedValue("sc2_kerrigan.wav — 36.9s\nsc2_raynor.wav — 36.2s");
    monter(etatDeTest({ packs: [aFabriquer()] }));

    await userEvent.click(fabriquer());

    await waitFor(() => expect(screen.getByText(/sc2_kerrigan/)).toHaveClass("journal"));
  });

  it("ferme les deux boutons pendant la fabrication", async () => {
    let finir!: (v: string) => void;
    vi.mocked(fabriquerPack).mockReturnValue(new Promise<string>((ok) => (finir = ok)));
    monter(etatDeTest({ packs: [aFabriquer()] }));

    await userEvent.click(fabriquer());
    expect(fabriquer()).toBeDisabled();
    expect(installer()).toBeDisabled();

    finir("");
    await waitFor(() => expect(fabriquer()).toBeEnabled());
  });
});
