// Les paquets, pilotes depuis la fenetre : installer, fabriquer, retirer, et dire ou l'on en est.
//
// TOUTES ASYNCHRONES, ET C'EST LA MEME RAISON A CHAQUE FOIS. Une commande synchrone tourne sur
// le fil de la fenetre : le battement qui lit l'avancement ne tournerait pas, et la barre
// resterait figee jusqu'a ce que tout soit fini. `async` la met sur l'executeur, et
// `spawn_blocking` sort le travail long des fils de l'executeur -- qui ne sont pas faits pour ca.

use serde::Serialize;
use tauri::State;
use tauri_plugin_dialog::DialogExt;

use crate::app::AppState;
use crate::{embedded, games, packs};

// Ou en est l'operation sur les paquets, lue par la fenetre a intervalle.
//
// `total` a zero veut dire « en cours, sans compte connu » : c'est le cas d'une fabrication, dont
// on ignore le nombre d'etapes avant qu'elle les annonce.
#[derive(Serialize, specta::Type, Default)]
pub struct PackProgress {
    pub active: bool,
    #[specta(type = u32)]
    pub done: usize,
    #[specta(type = u32)]
    pub total: usize,
    /// Le fichier ou l'etape en cours, tel quel : la fenetre l'affiche sans l'interpreter.
    pub step: String,
    /// Ce qui a ete fait jusqu'ici, ligne a ligne. La fenetre le montre pendant l'operation, et
    /// pas seulement a la fin : sur une fabrication de trois minutes, c'est la seule preuve que
    /// quelque chose avance.
    pub log: Vec<String>,
}

#[tauri::command]
#[specta::specta]
pub fn pack_progress(app: State<AppState>) -> PackProgress {
    let state = app.job.read();
    PackProgress {
        active: state.active,
        done: state.done,
        total: state.total,
        step: state.step,
        log: app.job.lines(),
    }
}

// Installer un paquet : le zip est choisi ici, comme les sons de l'atelier, parce que la page
// n'a pas de quoi ouvrir un selecteur de fichiers.
//
// Le selecteur reste sur le fil de la fenetre, avant le travail : c'est une fenetre modale du
// systeme, elle n'a rien a faire dans un fil de travail.
#[tauri::command]
#[specta::specta]
pub async fn install_pack(
    app: State<'_, AppState>,
    window: tauri::Window,
) -> Result<String, String> {
    let Some(zip) = window.dialog().file().add_filter("paquets", &["zip"]).blocking_pick_file()
    else {
        return Ok(String::new());
    };
    let path = zip.into_path().map_err(|e| e.to_string())?;

    let root = app.root.clone();
    let job = app.job.clone();
    let manifest = tauri::async_runtime::spawn_blocking(move || {
        let outcome = packs::install(&root, &path, &job);
        // Quoi qu'il arrive : la fenetre ne doit pas rester sur une barre qui n'avance plus.
        job.finish();
        outcome.map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    // Un paquet de modeles rend souvent parlant ce qui ne l'etait pas : on retente tout de suite.
    let brings_models = manifest.files.iter().any(|f| f.starts_with(&format!("{}/", crate::paths::MODELS)));
    if brings_models && !app.is_ready() {
        app.start_engine();
    }
    // Un paquet-recette n'apporte rien tant qu'on ne l'a pas fabrique : le dire tout de suite
    // evite de chercher des voix qui n'existent pas encore.
    if manifest.needs_build() {
        return Ok(format!(
            "{} installe : recette posee, reste a fabriquer depuis une copie du jeu",
            manifest.name
        ));
    }
    Ok(format!("{} installe : {} fichier(s)", manifest.name, manifest.files.len()))
}

// Fabriquer un paquet-recette : le faire tourner sur une copie installee du jeu.
//
// LE GESTE EST EXPLICITE, ET C'EST LE POINT. Un paquet-recette s'installe sans que rien ne
// s'execute ; c'est ce bouton, et lui seul, qui lance un script venu d'ailleurs. On demande donc
// d'abord ou est le jeu -- ce qui fait aussi office de confirmation.
//
// Bloquant, deux a quatre minutes : c'est la meme facon de faire que `forge_voice` et `warm_up`,
// qui rendent leur compte rendu a la fin plutot que de tenir un fil de progression.
#[tauri::command]
#[specta::specta]
pub async fn build_pack(
    app: State<'_, AppState>,
    window: tauri::Window,
    pack: String,
    choose_folder: bool,
) -> Result<String, String> {
    let mut manifest = packs::installed(&app.root)
        .into_iter()
        .find(|m| m.name == pack)
        .ok_or_else(|| format!("paquet inconnu : {pack}"))?;
    if manifest.recipe.is_empty() {
        return Err(format!("« {pack} » ne porte pas de recette a fabriquer"));
    }

    // Le dossier du jeu : trouve tout seul quand c'est possible, demande sinon. `choose_folder`
    // force la question -- c'est la sortie de secours quand la detection tombe sur la mauvaise
    // installation, ou sur aucune.
    let game = match (choose_folder, games::find_install(&manifest.product)) {
        (false, Some(found)) => found,
        _ => {
            let Some(folder) = window.dialog().file().blocking_pick_folder() else {
                return Ok(String::new());
            };
            folder.into_path().map_err(|e| e.to_string())?
        }
    };

    let root = app.root.clone();
    let builder = embedded::builder_path(&app.root);
    let job = app.job.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        // Sans compte connu : le fils annonce ses etapes au fur et a mesure, il ne les compte pas
        // d'avance. La fenetre montre donc la derniere ligne, pas une proportion.
        job.start("recensement du stockage…", 0);
        job.log(&format!("jeu lu dans {}", game.display()));
        let outcome = packs::build(&root, &builder, &mut manifest, &game, &job);
        job.finish();
        outcome.map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;

    Ok(format!(
        "{}\n{} fichier(s) posé(s) : {}",
        report.log.join("\n"),
        report.produced.len(),
        report.produced.join(", ")
    ))
}

// Retirer un paquet : ses fichiers et son inscription.
//
// Il n'y a pas de confirmation ici. Elle est dans la page, ou un premier clic arme le bouton et
// un second efface : une boite de dialogue de plus se clique sans la lire.
#[tauri::command]
#[specta::specta]
pub async fn uninstall_pack(app: State<'_, AppState>, pack: String) -> Result<String, String> {
    let root = app.root.clone();
    let job = app.job.clone();
    let name = pack.clone();
    let removed = tauri::async_runtime::spawn_blocking(move || {
        let outcome = packs::uninstall(&root, &name, &job);
        job.finish();
        outcome.map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(format!("{pack} retiré : {removed} fichier(s) effacé(s)"))
}
