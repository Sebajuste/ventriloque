// `.build.info` : le fichier a colonnes qu'une installation Blizzard porte a sa racine.
//
// Deux lignes utiles -- l'entete et la premiere valeur --, separees par des barres verticales.
// Le nom de colonne porte son type derriere un `!` : « CDN Path!STRING:0 ».

use std::path::Path;

/// Le nom du fichier qui signe une installation Blizzard.
pub const FILE: &str = ".build.info";

/// Le code produit d'une installation, lu dans son `.build.info`. `None` si le dossier n'en
/// porte pas, ou si le fichier ne dit rien d'exploitable.
pub fn product_at(folder: &Path) -> Option<String> {
    let text = std::fs::read_to_string(folder.join(FILE)).ok()?;
    let mut lines = text.lines();
    let headers: Vec<&str> = lines.next()?.split('|').collect();
    let values: Vec<&str> = lines.next()?.split('|').collect();

    let column = |wanted: &str| {
        headers
            .iter()
            .position(|h| h.split('!').next() == Some(wanted))
            .and_then(|i| values.get(i))
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
    };

    // `Product` existe dans l'entete mais reste vide sur les deux installations mesurees : c'est
    // `CDN Path` qui porte l'information, sous la forme `tpr/<code>`.
    column("Product")
        .or_else(|| column("CDN Path").and_then(|c| c.rsplit('/').next()))
        .map(|c| c.to_lowercase())
}

#[cfg(test)]
mod tests {
    use crate::testing::TempDir;

    const SC2: &str =
        "Branch!STRING:0|CDN Path!STRING:0|Product!STRING:0\nfr_FR|tpr/sc2|\n";

    fn folder_with(content: Option<&str>) -> TempDir {
        let dir = TempDir::new("build-info");
        if let Some(text) = content {
            std::fs::write(dir.path().join(super::FILE), text).unwrap();
        }
        dir
    }

    #[test]
    fn le_code_produit_sort_du_chemin_de_cdn() {
        let dir = folder_with(Some(SC2));
        assert_eq!(super::product_at(dir.path()).as_deref(), Some("sc2"));
    }

    // Quand la colonne `Product` est remplie, c'est elle qui prime.
    #[test]
    fn la_colonne_produit_prime_si_elle_dit_quelque_chose() {
        let dir = folder_with(Some("CDN Path!STRING:0|Product!STRING:0\ntpr/autre|fenris\n"));
        assert_eq!(super::product_at(dir.path()).as_deref(), Some("fenris"));
    }

    #[test]
    fn un_dossier_sans_build_info_ne_dit_rien() {
        let dir = folder_with(None);
        assert_eq!(super::product_at(dir.path()), None);
    }

    #[test]
    fn un_build_info_tronque_ne_fait_pas_paniquer() {
        let dir = folder_with(Some("Branch!STRING:0|CDN Path!STRING:0\n"));
        assert_eq!(super::product_at(dir.path()), None);
    }
}
