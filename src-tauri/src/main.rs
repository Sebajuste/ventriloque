// Ventriloque : un atelier qui fabrique des voix de PNJ, un player qui les joue en seance.
//
// CE FICHIER NE FAIT QUE DEMARRER. L'etat est dans `app.rs`, les commandes dans `ipc/`, le
// travail dans les modules qu'elles appellent : ici il n'y a que l'ordre d'allumage et le
// branchement de l'extinction.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod audio;
mod characters;
mod clock;
mod embedded;
mod engine;
mod games;
mod ipc;
mod migration;
mod packs;
mod paths;
mod settings;
mod voice;

#[cfg(test)]
mod testing;

use app::AppState;
use tauri::Manager;

fn main() {
    let contract = ipc::contract();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|handle| {
            let state = AppState::new();
            state.start_engine();
            handle.manage(state);
            Ok(())
        })
        .invoke_handler(contract.invoke_handler())
        .build(tauri::generate_context!())
        .expect("Ventriloque n'a pas pu demarrer")
        // Fermer la fenetre doit tuer le moteur, et rien ne le fait tout seul : voir
        // `AppState::shutdown`.
        .run(|handle, event| {
            if let tauri::RunEvent::Exit = event
                && let Some(state) = handle.try_state::<AppState>()
            {
                state.shutdown();
            }
        });
}
