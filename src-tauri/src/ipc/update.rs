// La mise a jour, pilotee depuis la fenetre : chercher, poser, dire ou l'on en est.
//
// TROIS COMMANDES ET PAS LES COMMANDES DU PLUGIN. Le plugin updater en expose au frontend, mais
// les cabler demanderait d'ouvrir ses permissions au webview. Tout passe par les commandes de
// l'application, comme le reste du chassis : la fenetre demande et affiche, la decision est
// dans `update`.
//
// LA RECHERCHE EST DEMANDEE PAR LA FENETRE, pas faite au demarrage. Elle coute un aller-retour
// reseau que le lancement n'a pas a porter, et une seance hors ligne ne doit pas attendre un
// delai d'expiration avant de parler.

use serde::Serialize;
use tauri::State;

use crate::update::Updates;

/// Ce que la recherche a trouve. `version` vide veut dire « rien de neuf » : c'est un retour
/// plus simple a lire pour la fenetre qu'un `Option` de plus a demeler.
#[derive(Serialize, specta::Type, Default)]
pub struct UpdateFound {
    pub version: String,
    pub current: String,
    /// Les notes de publication de la release, telles quelles. Vide si elle n'en porte pas.
    pub notes: String,
}

// Les octets en `u32` : specta rend un `u64` en `bigint`, que la fenetre devrait demeler pour
// afficher des megaoctets. Un installateur depasse le compte de quatre milliards le jour ou il
// pese quatre gigaoctets ; celui-ci en pese cinquante.
#[derive(Serialize, specta::Type, Default)]
pub struct UpdateProgress {
    pub active: bool,
    #[specta(type = u32)]
    pub downloaded: u64,
    #[specta(type = u32)]
    pub total: u64,
}

#[tauri::command]
#[specta::specta]
pub async fn check_update(
    app: tauri::AppHandle,
    updates: State<'_, Updates>,
) -> Result<UpdateFound, String> {
    match updates.look(&app).await? {
        Some(found) => {
            Ok(UpdateFound { version: found.version, current: found.current, notes: found.notes })
        }
        None => Ok(UpdateFound::default()),
    }
}

/// Ne rend jamais la main quand elle reussit : l'application est relancee.
#[tauri::command]
#[specta::specta]
pub async fn install_update(
    app: tauri::AppHandle,
    updates: State<'_, Updates>,
) -> Result<(), String> {
    updates.install(&app).await
}

#[tauri::command]
#[specta::specta]
pub fn update_progress(updates: State<Updates>) -> UpdateProgress {
    let read = updates.progress();
    UpdateProgress { active: read.active, downloaded: read.downloaded, total: read.total }
}
