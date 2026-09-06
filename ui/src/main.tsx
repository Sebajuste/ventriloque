// HASHROUTER ET PAS BROWSERROUTER. Tauri sert la fenêtre depuis un protocole à lui
// (`tauri://localhost`), pas depuis un serveur : l'API History écrirait des adresses que rien
// ne sait resservir, et un rechargement sur `/fiches` ouvrirait une fenêtre blanche. Le hash ne
// quitte jamais la page, donc il survit au rechargement comme au protocole.

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { HashRouter } from "react-router";
import { Routage } from "./routes";
import "./styles.css";

const racine = document.getElementById("racine");
if (racine === null) throw new Error("la page n'a pas de point d'accroche");

createRoot(racine).render(
  <StrictMode>
    <HashRouter>
      <Routage />
    </HashRouter>
  </StrictMode>,
);
