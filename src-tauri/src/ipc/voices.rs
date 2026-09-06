// L'atelier, en deux commandes : choisir des sons, en faire une voix nommee.

use tauri::State;

use crate::app::AppState;
use crate::voice::{self, Recipe};

/// Les formats que le selecteur propose. Tout ce que rodio sait decoder.
const AUDIO_FILTERS: [&str; 6] = ["wav", "mp3", "flac", "ogg", "m4a", "opus"];

/// Trente secondes suffisent -- les references du mod en font 30 a 35 -- et au-dela on paie du
/// temps de clonage sans rien gagner. Deux de marge pour ne pas couper un mot.
const MAX_REFERENCE_SECONDS: f32 = 32.0;

// Le nom sert de nom de fichier. Reforger sous le meme nom est sur : le moteur invalide son
// cache sur la date du .wav, verifie, donc la voix suivante sera bien la nouvelle.
#[tauri::command]
#[specta::specta]
pub fn forge_voice(
    app: State<AppState>,
    name: String,
    files: Vec<String>,
    pitch: f32,
    formants: f32,
) -> Result<String, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("il faut nommer la voix".into());
    }
    if files.is_empty() {
        return Err("il faut au moins un fichier son".into());
    }

    let paths: Vec<std::path::PathBuf> = files.iter().map(std::path::PathBuf::from).collect();
    let raw = voice::assemble(&paths, MAX_REFERENCE_SECONDS).map_err(|e| e.to_string())?;

    let recipe = Recipe {
        source: paths.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(" ; "),
        from: None,
        to: None,
        pitch,
        formants,
    };
    let made = voice::render(&raw, &recipe).map_err(|e| e.to_string())?;

    let file = format!("{}.wav", file_stem(name));
    let target = app.voices_dir.join(&file);
    voice::write_wav(&made, &target).map_err(|e| e.to_string())?;

    // La recette a cote du resultat. Sans elle, une voix qui plait a quatre-vingt-dix pour cent
    // est intouchable : on ne peut que la refaire de zero.
    let _ = std::fs::write(
        target.with_extension("json"),
        serde_json::to_string_pretty(&recipe).unwrap_or_default(),
    );
    Ok(file)
}

/// Le nom devient un nom de fichier : on ne garde que ce qui en fait un sans surprise.
fn file_stem(name: &str) -> String {
    name.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect()
}

// Le selecteur de fichiers vit en Rust et pas dans la page.
//
// Un `<input type="file">` ne donnerait pas les chemins reels, dont l'atelier a besoin pour
// aller lire les sons.
#[tauri::command]
#[specta::specta]
pub fn pick_audio_files(window: tauri::Window) -> Vec<String> {
    use tauri_plugin_dialog::DialogExt;
    window
        .dialog()
        .file()
        .add_filter("sons", &AUDIO_FILTERS)
        .blocking_pick_files()
        .unwrap_or_default()
        .into_iter()
        .map(|f| f.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn le_nom_de_voix_devient_un_nom_de_fichier_sans_surprise() {
        assert_eq!(super::file_stem("barman"), "barman");
        assert_eq!(super::file_stem("Le Barman"), "Le_Barman");
        assert_eq!(super::file_stem("a/b\\c"), "a_b_c");
    }
}
