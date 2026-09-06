// Les fiches de personnages, ecrites et retirees depuis la fenetre.

use tauri::State;

use crate::app::AppState;
use crate::characters::{self, Character};

/// `previous_id` porte l'identifiant d'avant, pour qu'un renommage ne laisse pas de doublon.
#[tauri::command]
#[specta::specta]
pub fn write_character(
    app: State<AppState>,
    character: Character,
    previous_id: String,
) -> Result<Character, String> {
    characters::write(&app.characters_dir, character, &previous_id).map_err(|e| e.to_string())
}

#[tauri::command]
#[specta::specta]
pub fn delete_character(app: State<AppState>, id: String) -> Result<(), String> {
    characters::delete(&app.characters_dir, &id).map_err(|e| e.to_string())
}
