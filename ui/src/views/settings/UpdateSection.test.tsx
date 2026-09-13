// Ce que la section de mise a jour doit tenir.
//
// L'essentiel : RIEN NE SE CHERCHE NI NE S'INSTALLE TOUT SEUL. Une recherche coute un
// aller-retour reseau, une installation coupe la parole et relance l'application -- les deux
// demandent un clic, et le second n'apparait que s'il y a quelque chose a installer.

import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import UpdateSection from "./UpdateSection";
import { checkUpdate, installUpdate, updateProgress } from "../../ipc";

vi.mock("../../ipc", async (original) => ({
  ...(await original<typeof import("../../ipc")>()),
  checkUpdate: vi.fn(),
  installUpdate: vi.fn(),
  updateProgress: vi.fn(),
}));

const button = (name: string) => screen.getByRole("button", { name });

const nothingNew = { version: "", current: "0.1.0", notes: "" };
const available = { version: "0.2.0", current: "0.1.0", notes: "Une voix de plus." };

beforeEach(() => {
  vi.mocked(checkUpdate).mockReset().mockResolvedValue(nothingNew);
  vi.mocked(installUpdate).mockReset().mockResolvedValue(undefined);
  vi.mocked(updateProgress)
    .mockReset()
    .mockResolvedValue({ active: false, downloaded: 0, total: 0 });
});

describe("la recherche", () => {
  it("ne part pas au montage", () => {
    render(<UpdateSection />);
    expect(checkUpdate).not.toHaveBeenCalled();
  });

  it("dit qu'on est a jour, et ne propose rien a installer", async () => {
    render(<UpdateSection />);
    await userEvent.click(button("Chercher une mise à jour"));

    await waitFor(() => expect(screen.getByText(/est à jour/)).toBeInTheDocument());
    expect(screen.queryByRole("button", { name: "Installer et relancer" })).toBeNull();
  });

  it("annonce la version trouvee et ses notes", async () => {
    vi.mocked(checkUpdate).mockResolvedValue(available);
    render(<UpdateSection />);
    await userEvent.click(button("Chercher une mise à jour"));

    await waitFor(() => expect(screen.getByText(/0\.2\.0 disponible/)).toBeInTheDocument());
    expect(screen.getByText("Une voix de plus.")).toBeInTheDocument();
    expect(button("Installer et relancer")).toBeEnabled();
  });

  it("montre la panne plutot que de la taire", async () => {
    vi.mocked(checkUpdate).mockRejectedValue("pas de reseau");
    render(<UpdateSection />);
    await userEvent.click(button("Chercher une mise à jour"));

    await waitFor(() => expect(screen.getByText("pas de reseau")).toBeInTheDocument());
  });
});

describe("l'installation", () => {
  it("previent que la parole est coupee AVANT le clic", async () => {
    vi.mocked(checkUpdate).mockResolvedValue(available);
    render(<UpdateSection />);
    await userEvent.click(button("Chercher une mise à jour"));

    await waitFor(() => expect(screen.getByText(/le moteur de parole est coupé/)).toBeInTheDocument());
  });

  it("ne part que sur un clic, et montre alors une barre", async () => {
    vi.mocked(checkUpdate).mockResolvedValue(available);
    // L'installation ne se resout pas : en vrai, l'application est relancee sous la fenetre.
    vi.mocked(installUpdate).mockImplementation(() => new Promise(() => {}));
    render(<UpdateSection />);
    await userEvent.click(button("Chercher une mise à jour"));
    await waitFor(() => expect(button("Installer et relancer")).toBeEnabled());

    expect(installUpdate).not.toHaveBeenCalled();
    await userEvent.click(button("Installer et relancer"));

    expect(installUpdate).toHaveBeenCalledOnce();
    await waitFor(() =>
      expect(screen.getByRole("progressbar", { name: /Téléchargement/ })).toBeInTheDocument(),
    );
  });

  it("rend la main sur une panne, au lieu de laisser la barre figee", async () => {
    vi.mocked(checkUpdate).mockResolvedValue(available);
    vi.mocked(installUpdate).mockRejectedValue("signature refusee");
    render(<UpdateSection />);
    await userEvent.click(button("Chercher une mise à jour"));
    await waitFor(() => expect(button("Installer et relancer")).toBeEnabled());
    await userEvent.click(button("Installer et relancer"));

    await waitFor(() => expect(screen.getByText("signature refusee")).toBeInTheDocument());
    expect(screen.queryByRole("progressbar")).toBeNull();
    expect(button("Chercher une mise à jour")).toBeEnabled();
  });
});
