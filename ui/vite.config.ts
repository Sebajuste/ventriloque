import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri sert les fichiers batis depuis `dist`, et rien d'autre n'est publie : pas de serveur,
// pas de reseau. `cargo build` les embarque dans l'executable, donc il faut avoir lance
// `npm run build` avant de compiler la partie Rust.
export default defineConfig({
  plugins: [react()],
  build: { outDir: "dist", emptyOutDir: true, target: "esnext" },
  clearScreen: false,
});
