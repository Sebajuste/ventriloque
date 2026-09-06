// Ce qui a ete installe, garde a cote des donnees.
//
// Sert a l'affichage, et rend une desinstallation possible plus tard sans deviner ce qui
// appartient a quoi : sans cette inscription, retirer un paquet demanderait de comparer le
// contenu d'un zip qu'on n'a plus a un dossier qui a bouge depuis.

use anyhow::{Context, Result};
use std::path::Path;

use crate::paths;

use super::legacy;
use super::manifest::Manifest;

/// Les paquets inscrits, tries par nom.
pub fn installed(root: &Path) -> Vec<Manifest> {
    let mut list: Vec<Manifest> = std::fs::read_dir(root.join(paths::PACKS))
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                .filter_map(|e| legacy::parse(&std::fs::read_to_string(e.path()).ok()?))
                .collect()
        })
        .unwrap_or_default();
    list.sort_by(|a: &Manifest, b: &Manifest| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    list
}

/// Inscrit ou reinscrit un paquet.
pub fn record(root: &Path, manifest: &Manifest) -> Result<()> {
    let folder = root.join(paths::PACKS);
    std::fs::create_dir_all(&folder).ok();
    let text = serde_json::to_string_pretty(manifest)?;
    std::fs::write(folder.join(format!("{}.json", manifest.id())), text)
        .context("inscription du paquet")
}

/// Retire l'inscription d'un paquet.
pub fn forget(root: &Path, manifest: &Manifest) -> Result<()> {
    std::fs::remove_file(root.join(paths::PACKS).join(format!("{}.json", manifest.id())))
        .context("retrait de l'inscription du paquet")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    fn named(name: &str) -> Manifest {
        Manifest { name: name.into(), ..Manifest::default() }
    }

    #[test]
    fn ce_qui_est_inscrit_se_relit_trie() {
        let root = TempDir::new("store");
        record(root.path(), &named("StarCraft II")).unwrap();
        record(root.path(), &named("Cyberpunk 2077")).unwrap();

        let list = installed(root.path());
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].name, "Cyberpunk 2077");
        assert_eq!(list[1].name, "StarCraft II");
    }

    #[test]
    fn oublier_retire_l_inscription() {
        let root = TempDir::new("store-oubli");
        let m = named("essai");
        record(root.path(), &m).unwrap();
        forget(root.path(), &m).unwrap();
        assert!(installed(root.path()).is_empty());
    }

    // Un dossier absent n'est pas une erreur : c'est l'etat d'une installation neuve.
    #[test]
    fn sans_dossier_de_paquets_la_liste_est_vide() {
        let root = TempDir::new("store-neuf");
        assert!(installed(root.path()).is_empty());
    }

    // Un fichier illisible ne doit pas emporter la liste entiere avec lui.
    #[test]
    fn un_fichier_bricole_est_saute_sans_perdre_les_autres() {
        let root = TempDir::new("store-bricole");
        record(root.path(), &named("bon")).unwrap();
        std::fs::write(root.path().join("packs/casse.json"), "{ ceci n'est pas du json").unwrap();

        let list = installed(root.path());
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "bon");
    }
}
