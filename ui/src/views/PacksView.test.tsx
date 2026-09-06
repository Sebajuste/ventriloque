// Ce que les paquets doivent tenir.
//
// UNE CHAINE VIDE VEUT DIRE « ANNULE », pas « rien installe ». Rust rend le compte des fichiers
// quand l'installation a eu lieu, et une chaine vide quand le selecteur a ete ferme : confondre
// les deux relirait le disque pour rien a chaque fois qu'on renonce.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import PacksView from "./PacksView";
import {
  buildPack,
  installPack,
  packProgress,
  uninstallPack,
  type Manifest,
  type PackProgress,
  type Snapshot,
} from "../ipc";
import { aPack, snapshotOf } from "../test/fixtures";

vi.mock("../ipc", async (original) => ({
  ...(await original<typeof import("../ipc")>()),
  installPack: vi.fn(),
  buildPack: vi.fn(),
  uninstallPack: vi.fn(),
  packProgress: vi.fn(),
}));

const IDLE: PackProgress = { active: false, done: 0, total: 0, step: "", log: [] };

const reload = vi.fn<() => Promise<void>>();
const mount = (snapshot: Snapshot = snapshotOf()) =>
  render(<PacksView snapshot={snapshot} reload={reload} />);
const installButton = () => screen.getByRole("button", { name: "Installer un paquet…" });

const voicePack = (over: Partial<Manifest> = {}) =>
  aPack("Voix de Night City", {
    version: "1.2",
    description: "Douze voix clonées",
    author: "sebajuste",
    files: ["voices/judy.wav", "voices/panam.wav"],
    installed_on: "2026-09-05",
    ...over,
  });

// Un paquet-recette : il n'apporte aucune voix tant qu'il n'a pas tourne.
const recipePack = (over: Partial<Manifest> = {}) =>
  aPack("StarCraft II", {
    recipe: "starcraft2.rhai",
    game: "StarCraft II",
    product: "sc2",
    files: [],
    ...over,
  });

/** Une promesse que le test denoue quand il veut, pour garder une operation en cours. */
const pending = () => {
  let finish!: (value: string) => void;
  const promise = new Promise<string>((ok) => (finish = ok));
  return { promise, finish };
};

beforeEach(() => {
  reload.mockReset().mockResolvedValue(undefined);
  vi.mocked(installPack).mockReset().mockResolvedValue("");
  vi.mocked(buildPack).mockReset().mockResolvedValue("");
  vi.mocked(uninstallPack).mockReset().mockResolvedValue("");
  vi.mocked(packProgress).mockReset().mockResolvedValue(IDLE);
});

// LES COMMANDES NE RENDENT LA MAIN QU'À LA FIN. Une installation de 258 Mo n'a donc rien à dire
// pendant une minute si personne ne bat la mesure : c'est ce battement qui fait vivre la barre.
describe("l'avancement", () => {
  const bar = () => screen.getByRole("progressbar");

  it("ne montre aucune barre au repos", () => {
    mount(snapshotOf({ packs: [voicePack()] }));

    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("montre une proportion quand le total est connu", async () => {
    const install = pending();
    vi.mocked(installPack).mockReturnValue(install.promise);
    vi.mocked(packProgress).mockResolvedValue({
      active: true,
      done: 3,
      total: 6,
      step: "voices/judy.wav",
      log: [],
    });
    mount();

    await userEvent.click(installButton());

    await waitFor(() => expect(screen.getByText("voices/judy.wav")).toBeInTheDocument());
    expect(bar()).toHaveAttribute("aria-valuenow", "3");
    expect(bar()).toHaveAttribute("aria-valuemax", "6");
    expect(screen.getByText("installation — 50 %")).toBeInTheDocument();

    install.finish("");
  });

  // Une fabrication ne compte pas ses étapes d'avance : donner un pourcentage serait mentir.
  it("s'abstient de proportion quand le total est inconnu", async () => {
    const build = pending();
    vi.mocked(buildPack).mockReturnValue(build.promise);
    vi.mocked(packProgress).mockResolvedValue({
      active: true,
      done: 0,
      total: 0,
      step: "recensement : 779862 entrees",
      log: ["stockage ouvert : produit s2"],
    });
    mount(snapshotOf({ packs: [recipePack()] }));

    await userEvent.click(screen.getByRole("button", { name: "Fabriquer…" }));

    await waitFor(() =>
      expect(screen.getByText("recensement : 779862 entrees")).toBeInTheDocument(),
    );
    expect(bar()).not.toHaveAttribute("aria-valuenow");
    expect(screen.getByText("fabrication en cours, quelques minutes…")).toBeInTheDocument();

    build.finish("");
  });

  it("montre le journal pendant l'opération, sans attendre la fin", async () => {
    const build = pending();
    vi.mocked(buildPack).mockReturnValue(build.promise);
    vi.mocked(packProgress).mockResolvedValue({
      active: true,
      done: 0,
      total: 0,
      step: "sc2_nova.wav — 36.3s",
      log: ["stockage ouvert : produit s2", "recensement : 779862 entrees"],
    });
    mount(snapshotOf({ packs: [recipePack()] }));

    await userEvent.click(screen.getByRole("button", { name: "Fabriquer…" }));

    await waitFor(() => expect(screen.getByText(/recensement/)).toHaveClass("log"));
    expect(screen.getByText(/stockage ouvert/)).toBeInTheDocument();

    build.finish("");
  });

  // Le journal en direct disparaît quand le compte rendu arrive : deux blocs pour la même chose
  // se seraient contredits, l'un figé sur l'avant-dernière ligne.
  it("cède la place au compte rendu une fois l'opération finie", async () => {
    vi.mocked(buildPack).mockResolvedValue("sc2_nova.wav — 36.3s\nsc2_karax.wav — 35.0s");
    mount(snapshotOf({ packs: [recipePack()] }));

    await userEvent.click(screen.getByRole("button", { name: "Fabriquer…" }));

    await waitFor(() => expect(screen.getByText(/sc2_karax/)).toHaveClass("log"));
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("range la barre dès que l'opération rend la main", async () => {
    const install = pending();
    vi.mocked(installPack).mockReturnValue(install.promise);
    vi.mocked(packProgress).mockResolvedValue({
      active: true,
      done: 1,
      total: 2,
      step: "x",
      log: [],
    });
    mount();

    await userEvent.click(installButton());
    await waitFor(() => expect(bar()).toBeInTheDocument());

    install.finish("essai installe : 2 fichier(s)");

    await waitFor(() => expect(screen.queryByRole("progressbar")).not.toBeInTheDocument());
  });
});

// UN CLIC N'EFFACE RIEN. Le premier arme, le second efface : une boîte de dialogue de plus se
// clique sans la lire, un bouton qui change de texte se voit.
describe("le retrait", () => {
  const remove = () => screen.getByRole("button", { name: "Retirer…" });
  const confirm = () => screen.getByRole("button", { name: "Confirmer le retrait" });

  it("n'appelle rien au premier clic", async () => {
    mount(snapshotOf({ packs: [voicePack()] }));

    await userEvent.click(remove());

    expect(uninstallPack).not.toHaveBeenCalled();
    expect(confirm()).toBeInTheDocument();
    expect(screen.getByText("2 fichiers seront effacés")).toBeInTheDocument();
  });

  it("efface au second clic, puis relit le disque", async () => {
    vi.mocked(uninstallPack).mockResolvedValue(
      "Voix de Night City retiré : 18 fichier(s) effacé(s)",
    );
    mount(snapshotOf({ packs: [voicePack()] }));

    await userEvent.click(remove());
    await userEvent.click(confirm());

    await waitFor(() =>
      expect(
        screen.getByText("Voix de Night City retiré : 18 fichier(s) effacé(s)"),
      ).toBeInTheDocument(),
    );
    expect(vi.mocked(uninstallPack)).toHaveBeenCalledWith("Voix de Night City");
    expect(reload).toHaveBeenCalledOnce();
  });

  // Retirer les modèles rend l'application muette : ça se dit avant, pas après.
  it("prévient quand le paquet porte les modèles", async () => {
    mount(snapshotOf({ packs: [voicePack({ files: ["models/flow_lm_main_int8.onnx"] })] }));

    await userEvent.click(remove());

    expect(
      screen.getByText("ceci retire les modèles : Ventriloque redeviendra muet"),
    ).toBeInTheDocument();
  });

  it("désarme dès qu'un autre geste commence", async () => {
    mount(snapshotOf({ packs: [voicePack()] }));
    await userEvent.click(remove());

    await userEvent.click(installButton());

    await waitFor(() => expect(remove()).toBeInTheDocument());
    expect(screen.queryByRole("button", { name: "Confirmer le retrait" })).not.toBeInTheDocument();
  });
});

describe("installation", () => {
  it("relit le disque quand un paquet est entre", async () => {
    vi.mocked(installPack).mockResolvedValue("Voix de Night City installe : 12 fichier(s)");
    mount();

    await userEvent.click(installButton());

    await waitFor(() =>
      expect(screen.getByText("Voix de Night City installe : 12 fichier(s)")).toBeInTheDocument(),
    );
    expect(reload).toHaveBeenCalledOnce();
  });

  it("ne relit rien quand le selecteur a ete ferme", async () => {
    mount();

    await userEvent.click(installButton());

    await waitFor(() => expect(screen.getByText("annulé")).toBeInTheDocument());
    expect(reload).not.toHaveBeenCalled();
  });

  it("montre la panne sans la confondre avec un abandon", async () => {
    vi.mocked(installPack).mockRejectedValue("ce zip ne porte pas de pack.json");
    mount();

    await userEvent.click(installButton());

    await waitFor(() =>
      expect(screen.getByText("ce zip ne porte pas de pack.json")).toHaveClass("failed"),
    );
    expect(reload).not.toHaveBeenCalled();
  });

  it("ferme le bouton pendant l'extraction, puis le rend", async () => {
    const install = pending();
    vi.mocked(installPack).mockReturnValue(install.promise);
    mount();

    await userEvent.click(installButton());
    expect(installButton()).toBeDisabled();

    install.finish("");
    await waitFor(() => expect(installButton()).toBeEnabled());
  });

  it("efface la panne d'avant des qu'on retente", async () => {
    vi.mocked(installPack).mockRejectedValueOnce("ce zip ne porte pas de pack.json");
    mount();
    await userEvent.click(installButton());
    // La chaine exacte : la note de la page contient elle aussi un `<code>pack.json</code>`,
    // et une expression trop large trouverait deux elements au lieu de zero ou un.
    await waitFor(() =>
      expect(screen.getByText("ce zip ne porte pas de pack.json")).toBeInTheDocument(),
    );

    const retry = pending();
    vi.mocked(installPack).mockReturnValue(retry.promise);
    await userEvent.click(installButton());

    expect(screen.queryByText("ce zip ne porte pas de pack.json")).not.toBeInTheDocument();
    retry.finish("");
  });
});

describe("la liste des paquets", () => {
  it("dit le vide plutot que de ne rien dire", () => {
    mount();

    expect(screen.getByText("Aucun paquet installé.")).toBeInTheDocument();
  });

  it("porte le nom seul quand le paquet n'a pas de version", () => {
    mount(snapshotOf({ packs: [voicePack({ version: "" })] }));

    expect(screen.getByText("Voix de Night City")).toBeInTheDocument();
  });

  it("colle la version au nom quand il y en a une", () => {
    mount(snapshotOf({ packs: [voicePack()] }));

    expect(screen.getByText("Voix de Night City 1.2")).toBeInTheDocument();
  });

  it("saute les champs vides du detail au lieu de laisser des separateurs orphelins", () => {
    mount(
      snapshotOf({ packs: [voicePack({ description: "", author: "", installed_on: "" })] }),
    );

    expect(screen.getByText("2 fichiers")).toBeInTheDocument();
  });

  it("accorde le compte de fichiers au singulier", () => {
    mount(snapshotOf({ packs: [voicePack({ files: ["voices/judy.wav"] })] }));

    expect(screen.getByText(/1 fichier ·/)).toBeInTheDocument();
  });
});

// Un script venu d'ailleurs ne tourne que sur un geste. L'onglet doit donc dire clairement
// lequel des deux boutons fait quoi, et ne jamais proposer de fabriquer un paquet sans recette.
describe("les paquets-recettes", () => {
  const build = () => screen.getByRole("button", { name: /abriquer…$/ });

  it("ne propose rien a fabriquer pour un paquet qui porte deja ses voix", () => {
    mount(snapshotOf({ packs: [voicePack()] }));

    expect(screen.queryByRole("button", { name: /abriquer…$/ })).not.toBeInTheDocument();
  });

  it("nomme le jeu attendu tant que la recette n'a pas tourne", () => {
    mount(snapshotOf({ packs: [recipePack()] }));

    expect(build()).toHaveTextContent("Fabriquer…");
    expect(screen.getByText("à fabriquer depuis une copie de StarCraft II")).toBeInTheDocument();
  });

  it("propose de refaire une fabrication deja passee, en disant sa date", () => {
    mount(snapshotOf({ packs: [recipePack({ built_on: "2026-09-06" })] }));

    expect(build()).toHaveTextContent("Refabriquer…");
    expect(screen.getByText("fabriqué le 2026-09-06")).toBeInTheDocument();
  });

  // Le dossier du jeu se cherche tout seul : `false` veut dire « ne demande rien tant que la
  // detection trouve l'installation ».
  it("laisse Rust chercher le dossier du jeu", async () => {
    mount(snapshotOf({ packs: [recipePack()] }));

    await userEvent.click(build());

    await waitFor(() => expect(buildPack).toHaveBeenCalledWith("StarCraft II", false));
  });

  // La sortie de secours : deux installations du meme jeu, ou aucune trouvee.
  it("force le selecteur quand on demande un autre dossier", async () => {
    mount(snapshotOf({ packs: [recipePack()] }));

    await userEvent.click(screen.getByRole("button", { name: "Autre dossier…" }));

    await waitFor(() => expect(buildPack).toHaveBeenCalledWith("StarCraft II", true));
  });

  it("relit le disque quand la fabrication a pose quelque chose", async () => {
    vi.mocked(buildPack).mockResolvedValue("20 voix, 20 fiches");
    mount(snapshotOf({ packs: [recipePack()] }));

    await userEvent.click(build());

    await waitFor(() => expect(screen.getByText("20 voix, 20 fiches")).toBeInTheDocument());
    expect(vi.mocked(buildPack)).toHaveBeenCalledWith("StarCraft II", false);
    expect(reload).toHaveBeenCalledOnce();
  });

  // Le journal fait vingt lignes : il descend sous la liste plutot que dans la barre d'etat.
  it("met un compte rendu multiligne dans le journal", async () => {
    vi.mocked(buildPack).mockResolvedValue("sc2_kerrigan.wav — 36.9s\nsc2_raynor.wav — 36.2s");
    mount(snapshotOf({ packs: [recipePack()] }));

    await userEvent.click(build());

    await waitFor(() => expect(screen.getByText(/sc2_kerrigan/)).toHaveClass("log"));
  });

  it("ferme les deux boutons pendant la fabrication", async () => {
    const running = pending();
    vi.mocked(buildPack).mockReturnValue(running.promise);
    mount(snapshotOf({ packs: [recipePack()] }));

    await userEvent.click(build());
    expect(build()).toBeDisabled();
    expect(installButton()).toBeDisabled();

    running.finish("");
    await waitFor(() => expect(build()).toBeEnabled());
  });
});
