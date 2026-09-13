// Les reglages du moteur : les lire, et les appliquer.
//
// APPLIQUER, C'EST RELANCER. Le moteur ne lit ses reglages qu'en ligne de commande, a son
// lancement ; il n'y a pas de demi-mesure ou une valeur changerait sans redemarrage. La relance
// coute ~2,5 s, et les voix clonees n'y perdent rien : leur cache vit sur le disque.

use serde::Serialize;
use tauri::State;

use crate::app::AppState;
use crate::settings::{self, EngineSettings};

#[derive(Serialize, specta::Type)]
pub struct EngineTuning {
    pub current: EngineSettings,
    /// Les valeurs validees en jeu : ce que le bouton « valeurs d'origine » remet.
    pub defaults: EngineSettings,
}

#[tauri::command]
#[specta::specta]
pub fn engine_settings(app: State<AppState>) -> EngineTuning {
    EngineTuning { current: settings::engine(&app.root), defaults: EngineSettings::default() }
}

/// Rend les reglages tels qu'ils ont ete ecrits -- ramenes dans leurs bornes si besoin.
#[tauri::command]
#[specta::specta]
pub async fn apply_engine_settings(
    app: State<'_, AppState>,
    settings: EngineSettings,
) -> Result<EngineSettings, String> {
    let written = settings::save_engine(&app.root, settings).map_err(|e| e.to_string())?;
    app.start_engine();
    let failure = app.failure_message();
    if !failure.is_empty() {
        return Err(format!("reglages enregistres, mais le moteur n'a pas redemarre : {failure}"));
    }
    Ok(written)
}
