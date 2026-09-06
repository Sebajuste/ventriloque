//! Les recettes du depot compilent.
//!
//! CE QUI EST TESTE ICI, ET CE QUI NE L'EST PAS. Rhai resout les appels a l'execution : ce test
//! prouve la syntaxe, pas qu'un verbe existe. Il n'en est pas moins le seul filet qu'une recette
//! ait -- sans lui, une virgule oubliee ne se voit qu'apres quatre minutes de fabrication, sur la
//! machine de quelqu'un qui a le jeu installe.
//!
//! Un test d'integration, et pas un test unitaire : il lit les fichiers du depot, qui sont des
//! donnees et pas du code.

use std::path::{Path, PathBuf};

fn recipes_dir() -> PathBuf {
    // `CARGO_MANIFEST_DIR` pointe `tools/pack-builder` ; les recettes sont a cote.
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().join("recipes")
}

#[test]
fn les_recettes_du_depot_compilent() {
    let engine = rhai::Engine::new_raw();
    let mut seen = 0;

    for entry in std::fs::read_dir(recipes_dir()).expect("dossier des recettes").flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|e| e != "rhai") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("recette lisible");
        engine
            .compile(&source)
            .unwrap_or_else(|e| panic!("{} ne compile pas : {e}", path.display()));
        seen += 1;
    }

    assert!(seen > 0, "aucune recette trouvee dans {}", recipes_dir().display());
}
