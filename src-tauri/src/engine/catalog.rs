// Les deux paliers de voix, lus sur le disque.
//
// LE CATALOGUE ne coute aucun clonage -- une voix de catalogue EST un etat deja calcule, elle se
// charge au lieu de se conditionner -- et c'est le palier qui parle quand on n'a encore rien
// fabrique.
//
// LES CLONES sont les references fabriquees par l'atelier ou posees par un paquet.

use std::path::Path;

/// Les noms de fichier d'un dossier qui portent `extension`, tries. `stem` dit si l'on garde le
/// nom sans son extension (`jean`) ou avec (`judy.wav`) : le moteur attend l'un pour le
/// catalogue, l'autre pour un clone.
fn names_with_extension(folder: &Path, extension: &str, stem: bool) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .filter_map(|entry| {
                    let path = entry.path();
                    if path.extension()? != extension {
                        return None;
                    }
                    let name = if stem { path.file_stem()? } else { path.file_name()? };
                    name.to_str().map(str::to_owned)
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// Les voix de catalogue livrees avec les modeles.
pub fn catalog_voices(models: &Path) -> Vec<String> {
    names_with_extension(&models.join("catalogue"), "kv", true)
}

/// Les references fabriquees, c'est-a-dire les voix clonees disponibles.
pub fn cloned_voices(voices: &Path) -> Vec<String> {
    names_with_extension(voices, "wav", false)
}

#[cfg(test)]
mod tests {
    use crate::testing::TempDir;

    #[test]
    fn un_clone_garde_son_extension_et_une_voix_de_catalogue_la_perd() {
        let root = TempDir::new("catalog");
        let catalogue = root.path().join("catalogue");
        std::fs::create_dir_all(&catalogue).unwrap();
        std::fs::write(catalogue.join("jean.kv"), "x").unwrap();
        std::fs::write(catalogue.join("eve.kv"), "x").unwrap();
        // Ce qui n'est pas une voix ne se retrouve pas dans la liste.
        std::fs::write(catalogue.join("notes.txt"), "x").unwrap();
        std::fs::write(root.path().join("judy.wav"), "RIFF").unwrap();

        assert_eq!(super::catalog_voices(root.path()), vec!["eve", "jean"]);
        assert_eq!(super::cloned_voices(root.path()), vec!["judy.wav"]);
    }

    #[test]
    fn un_dossier_absent_ne_rend_rien_plutot_que_de_paniquer() {
        let root = TempDir::new("catalog-vide");
        assert!(super::catalog_voices(root.path()).is_empty());
        assert!(super::cloned_voices(&root.path().join("nulle-part")).is_empty());
    }
}
