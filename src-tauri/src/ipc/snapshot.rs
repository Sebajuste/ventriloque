// Tout ce que la fenetre doit savoir, en un appel.
//
// L'ETAT EST RELU, PAS TENU. Voix, fiches et paquets vivent sur le disque, et on peut les y
// deposer a la main : la fenetre redemande cet instantane apres chaque commande qui les modifie
// plutot que de tenir un miroir qui divergerait du dossier.

use serde::Serialize;
use tauri::State;

use crate::app::AppState;
use crate::characters::Character;
use crate::packs::Manifest;
use crate::{audio, characters, engine, packs};

// D'OU VIENT LA VOIX -- un vrai type, et pas une chaine libre.
//
// L'interface s'appuie dessus pour dire ce qu'elle affiche, et une chaine se serait exportee en
// `string` : une faute de frappe cote Rust n'aurait rien casse a la compilation, elle aurait
// juste affiche une etiquette vide en seance. En enum, le contrat porte les deux seules valeurs
// possibles jusqu'a TypeScript.
#[derive(Serialize, specta::Type, Clone, Copy)]
#[serde(rename_all = "lowercase")]
pub enum VoiceKind {
    Clone,
    Catalog,
}

#[derive(Serialize, specta::Type)]
pub struct Voice {
    pub name: String,
    /// Ce qu'on envoie au moteur : `judy.wav` pour un clone, `jean` pour une voix de catalogue.
    pub reference: String,
    pub kind: VoiceKind,
}

#[derive(Serialize, specta::Type)]
pub struct Snapshot {
    pub ready: bool,
    pub characters: Vec<Character>,
    pub packs: Vec<Manifest>,
    pub failure: String,
    pub root: String,
    pub models: String,
    pub voices: Vec<Voice>,
    pub devices: Vec<String>,
    // RENDU EN `number`, PAS EN `bigint`. Specta refuse `usize` par defaut parce qu'au-dela de
    // 2^53 un entier perdrait des chiffres en silence cote JavaScript. C'est un rang dans la
    // liste des sorties audio de la machine : on est a quinze ordres de grandeur de la limite,
    // et l'IPC de Tauri passe par du JSON, qui ne transporte de toute facon pas de `bigint`.
    #[specta(type = u32)]
    pub device: usize,
}

fn available_voices(app: &AppState) -> Vec<Voice> {
    let mut voices: Vec<Voice> = engine::cloned_voices(&app.voices_dir)
        .into_iter()
        .map(|file| Voice {
            name: file.trim_end_matches(".wav").to_string(),
            reference: file,
            kind: VoiceKind::Clone,
        })
        .collect();
    voices.extend(engine::catalog_voices(&app.models_dir).into_iter().map(|name| Voice {
        name: name.clone(),
        reference: name,
        kind: VoiceKind::Catalog,
    }));
    voices
}

#[tauri::command]
#[specta::specta]
pub fn snapshot(app: State<AppState>) -> Snapshot {
    Snapshot {
        ready: app.is_ready(),
        characters: characters::all(&app.characters_dir),
        packs: packs::installed(&app.root),
        failure: app.failure_message(),
        root: app.root.display().to_string(),
        models: app.models_dir.display().to_string(),
        voices: available_voices(&app),
        devices: audio::device_names(),
        device: *app.device.lock().unwrap(),
    }
}
