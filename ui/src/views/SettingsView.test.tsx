// Ce que la page des reglages doit tenir.
//
// L'essentiel : RIEN NE PART AU MOTEUR TANT QU'ON N'A PAS APPLIQUE. Appliquer le relance, et une
// relance coupe la parole quelques secondes -- un curseur frole ne doit pas le faire.

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

import SettingsView from "./SettingsView";
import { applyEngineSettings, engineSettings } from "../ipc";
import { ENGINE_DEFAULTS, tuningOf } from "../test/fixtures";

vi.mock("../ipc", async (original) => ({
  ...(await original<typeof import("../ipc")>()),
  engineSettings: vi.fn(),
  applyEngineSettings: vi.fn(),
}));

const reload = vi.fn<() => Promise<void>>();
const mount = () => render(<SettingsView reload={reload} />);

const slider = (name: RegExp) => screen.getByLabelText(name) as HTMLInputElement;
const button = (name: string) => screen.getByRole("button", { name });
const move = (input: HTMLInputElement, value: number) =>
  fireEvent.change(input, { target: { value: String(value) } });

beforeEach(() => {
  reload.mockReset().mockResolvedValue(undefined);
  vi.mocked(engineSettings).mockReset().mockResolvedValue(tuningOf());
  vi.mocked(applyEngineSettings)
    .mockReset()
    .mockImplementation(async (s) => s);
});

describe("lecture", () => {
  it("montre les reglages en vigueur, pas ceux d'origine", async () => {
    vi.mocked(engineSettings).mockResolvedValue(tuningOf({ temperature: 0.55 }));
    mount();

    await waitFor(() => expect(slider(/Température/)).toHaveValue("0.55"));
    expect(screen.getByText("0.55")).toBeInTheDocument();
  });

  it("n'a rien a appliquer tant que rien n'a bouge", async () => {
    mount();

    await waitFor(() => expect(button("Appliquer")).toBeDisabled());
    expect(button("Valeurs d'origine")).toBeDisabled();
  });
});

describe("application", () => {
  it("un curseur deplace n'appelle pas le moteur", async () => {
    mount();
    await waitFor(() => expect(slider(/Température/)).toBeInTheDocument());

    move(slider(/Température/), 0.5);

    expect(applyEngineSettings).not.toHaveBeenCalled();
    expect(button("Appliquer")).toBeEnabled();
  });

  it("envoie le brouillon entier, puis relit l'etat de la fenetre", async () => {
    mount();
    await waitFor(() => expect(slider(/Température/)).toBeInTheDocument());

    move(slider(/Température/), 0.5);
    move(slider(/Écrêtage/), 2.5);
    await userEvent.click(button("Appliquer"));

    await waitFor(() =>
      expect(applyEngineSettings).toHaveBeenCalledWith({
        ...ENGINE_DEFAULTS,
        temperature: 0.5,
        noise_clamp: 2.5,
      }),
    );
    await waitFor(() => expect(screen.getByText(/appliqué/)).toBeInTheDocument());
    expect(reload).toHaveBeenCalledOnce();
    // Ce qui tourne est maintenant le brouillon : plus rien a appliquer.
    expect(button("Appliquer")).toBeDisabled();
  });

  it("reprend les valeurs telles que Rust les a bornees", async () => {
    vi.mocked(applyEngineSettings).mockImplementation(async (s) => ({ ...s, temperature: 1.5 }));
    mount();
    await waitFor(() => expect(slider(/Température/)).toBeInTheDocument());

    move(slider(/Température/), 1.2);
    await userEvent.click(button("Appliquer"));

    await waitFor(() => expect(slider(/Température/)).toHaveValue("1.5"));
  });

  it("dit la panne et relit quand meme, pour que l'en-tete la montre", async () => {
    vi.mocked(applyEngineSettings).mockRejectedValue("le moteur n'a pas redemarre");
    mount();
    await waitFor(() => expect(slider(/Température/)).toBeInTheDocument());

    move(slider(/Température/), 0.5);
    await userEvent.click(button("Appliquer"));

    await waitFor(() =>
      expect(screen.getByText("le moteur n'a pas redemarre")).toHaveClass("failed"),
    );
    expect(reload).toHaveBeenCalledOnce();
  });
});

describe("valeurs d'origine", () => {
  it("les remet dans le brouillon, sans les appliquer", async () => {
    vi.mocked(engineSettings).mockResolvedValue(tuningOf({ temperature: 0.4, lsd_steps: 4 }));
    mount();
    await waitFor(() => expect(slider(/Température/)).toHaveValue("0.4"));

    await userEvent.click(button("Valeurs d'origine"));

    expect(slider(/Température/)).toHaveValue("0.7");
    expect(slider(/Étapes/)).toHaveValue("1");
    expect(applyEngineSettings).not.toHaveBeenCalled();
    expect(button("Appliquer")).toBeEnabled();
  });
});

describe("trames apres la fin", () => {
  it("suivent le paquet de modeles par defaut, sans curseur", async () => {
    mount();

    await waitFor(() => expect(screen.getByRole("checkbox")).toBeChecked());
    expect(screen.queryAllByRole("slider")).toHaveLength(5);
  });

  it("prennent une valeur choisie une fois la case decochee", async () => {
    mount();
    await waitFor(() => expect(screen.getByRole("checkbox")).toBeInTheDocument());

    await userEvent.click(screen.getByRole("checkbox"));
    expect(screen.queryAllByRole("slider")).toHaveLength(6);
    await userEvent.click(button("Appliquer"));

    await waitFor(() =>
      expect(applyEngineSettings).toHaveBeenCalledWith(
        expect.objectContaining({ eos_extra: 8 }),
      ),
    );
  });
});
