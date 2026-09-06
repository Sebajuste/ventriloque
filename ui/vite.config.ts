import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

// Tauri sert les fichiers batis depuis `dist`, et rien d'autre n'est publie : pas de serveur,
// pas de reseau. `cargo build` les embarque dans l'executable, donc il faut avoir lance
// `npm run build` avant de compiler la partie Rust.
//
// La configuration des tests vit ici et pas dans un `vitest.config.ts` separe : les tests
// doivent traverser le meme plugin React et les memes reglages que la construction, sans quoi
// ils valident un code que personne n'execute.
export default defineConfig({
  plugins: [react()],
  build: { outDir: "dist", emptyOutDir: true, target: "esnext" },
  // Port fixe et refus de se rabattre ailleurs : `tauri dev` pointe cette adresse en dur, et une
  // fenetre ouverte sur un port vide serait un ecran blanc sans explication.
  server: { port: 1420, strictPort: true },
  clearScreen: false,
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/tests/preparation.ts"],
    include: ["src/**/*.test.ts?(x)"],
    restoreMocks: true,
  },
});
