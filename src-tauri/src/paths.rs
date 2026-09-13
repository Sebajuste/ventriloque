// Ou vivent les fichiers de Ventriloque, et sous quels noms.
//
// UN SEUL ENDROIT NOMME LES DOSSIERS. Ils apparaissent dans une entree de zip, dans un manifeste
// de paquet, dans la moisson d'une fabrication et dans la migration d'une ancienne installation :
// quatre endroits qui doivent dire le meme mot, et qui le disaient chacun a leur facon avant.

use std::path::{Path, PathBuf};

/// Les references de clonage, en `.wav`.
pub const VOICES: &str = "voices";
/// Les fiches de personnages, en `.json`.
pub const CHARACTERS: &str = "characters";
/// Le moteur de parole et ses poids -- un demi-gigaoctet, livre en paquet.
pub const MODELS: &str = "models";
/// Les scripts d'extraction poses par un paquet-recette.
pub const RECIPES: &str = "recipes";
/// L'inscription de ce qui a ete installe.
pub const PACKS: &str = "packs";
/// Les binaires deposes a cote de l'executable au premier lancement.
pub const ENGINE: &str = "engine";
/// Le dossier de travail jetable d'une fabrication.
pub const WORK: &str = ".build";

/// Le fichier qui marque la racine des donnees. IL NE PORTE QUE CE QUI EST A L'UTILISATEUR --
/// voir `TUNING` pour ce que l'application ecrit, et `settings` pour pourquoi les deux sont
/// separes.
pub const MARKER: &str = "ventriloque.json";

/// Les reglages du moteur, ecrits par l'application a chaque « Appliquer ».
pub const TUNING: &str = "tuning.json";

/// Les seuls dossiers qu'un paquet a le droit de remplir.
pub const PACK_DIRS: [&str; 4] = [VOICES, CHARACTERS, MODELS, RECIPES];

/// Ou vivent les fichiers, en partant de l'executable.
///
/// LE REPERE EST `ventriloque.json`, pas le dossier de l'executable. En developpement le binaire
/// est dans `src-tauri\target\release\` et les reglages sont quatre crans plus haut ; une fois
/// empaquete les deux sont cote a cote. Un seul repere couvre les deux cas, la ou une regle par
/// cas se serait trompee sur l'un des deux.
pub fn data_root() -> PathBuf {
    // `VENTRILOQUE_RACINE` est le nom d'avant : lu en second, il laisse en place les raccourcis
    // et les scripts d'essai qui le posaient deja.
    for name in ["VENTRILOQUE_ROOT", "VENTRILOQUE_RACINE"] {
        if let Ok(forced) = std::env::var(name) {
            return PathBuf::from(forced);
        }
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let start = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut folder = start.clone();
    for _ in 0..6 {
        if folder.join(MARKER).is_file() {
            return folder;
        }
        match folder.parent() {
            Some(p) => folder = p.to_path_buf(),
            None => break,
        }
    }
    start
}
