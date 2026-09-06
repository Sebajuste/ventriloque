// `ventriloque.json` : le fichier qui marque la racine, et le seul reglage qu'il porte.
//
// PAR DEFAUT, `models\` A COTE DES DONNEES -- ce que le paquet moteur remplit, et le seul
// endroit qui existe sur une machine neuve. Une installation se copie donc telle quelle.
//
// Le fichier peut nommer un autre dossier, ce qui sert pendant le developpement pour partager un
// demi-gigaoctet entre plusieurs copies du depot. MAIS CE N'EST QU'UN INDICE : un chemin qui ne
// repond pas est ignore, pas suivi. Sans cela, un `ventriloque.json` copie d'une machine a
// l'autre rendrait l'application muette en pointant un dossier qui n'existe que chez l'autre --
// et le message parlerait d'un chemin inconnu de celui qui le lit.

use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::paths;

#[derive(Deserialize, Default)]
struct Settings {
    // `modeles` est le nom d'avant : le lire en second laisse marcher les fichiers deja poses
    // sur les machines de developpement.
    #[serde(default, alias = "modeles")]
    models: String,
}

/// Ou sont les modeles : ce que le fichier indique s'il repond, `models\` sinon.
pub fn models_dir(root: &Path) -> PathBuf {
    let fallback = root.join(paths::MODELS);
    let hint = std::fs::read_to_string(root.join(paths::MARKER))
        .ok()
        .and_then(|text| serde_json::from_str::<Settings>(&text).ok())
        .map(|s| s.models)
        .filter(|m| !m.trim().is_empty())
        .map(PathBuf::from);

    match hint {
        Some(elsewhere) if elsewhere.is_dir() => elsewhere,
        _ => fallback,
    }
}

// Un chemin de modeles qui ne repond pas ne doit pas rendre l'application muette : c'est ce qui
// arrive des qu'un `ventriloque.json` passe d'une machine a l'autre.
#[cfg(test)]
mod tests {
    use crate::testing::TempDir;

    #[test]
    fn sans_reglages_ce_sont_les_modeles_du_dossier() {
        let root = TempDir::new("settings-nu");
        assert_eq!(super::models_dir(root.path()), root.path().join("models"));
    }

    #[test]
    fn un_renvoi_vivant_est_suivi() {
        let root = TempDir::new("settings-vivant");
        let elsewhere = root.path().join("autre-part");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::fs::write(
            root.path().join("ventriloque.json"),
            format!("{{\"models\": {:?}}}", elsewhere.to_string_lossy()),
        )
        .unwrap();

        assert_eq!(super::models_dir(root.path()), elsewhere);
    }

    // Le nom d'avant, pour les fichiers deja poses.
    #[test]
    fn l_ancien_nom_de_champ_est_encore_lu() {
        let root = TempDir::new("settings-ancien");
        let elsewhere = root.path().join("ailleurs");
        std::fs::create_dir_all(&elsewhere).unwrap();
        std::fs::write(
            root.path().join("ventriloque.json"),
            format!("{{\"modeles\": {:?}}}", elsewhere.to_string_lossy()),
        )
        .unwrap();

        assert_eq!(super::models_dir(root.path()), elsewhere);
    }

    #[test]
    fn un_renvoi_mort_est_ignore_au_lieu_d_etre_suivi() {
        let root = TempDir::new("settings-mort");
        std::fs::write(
            root.path().join("ventriloque.json"),
            r#"{"models": "Z:/une/machine/qui/n/est/pas/celle-ci"}"#,
        )
        .unwrap();

        assert_eq!(super::models_dir(root.path()), root.path().join("models"));
    }

    // Le fichier sert aussi de reperage de la racine : il peut exister sans rien regler.
    #[test]
    fn un_fichier_vide_reste_un_reperage_valable() {
        let root = TempDir::new("settings-vide");
        std::fs::write(root.path().join("ventriloque.json"), "{}").unwrap();

        assert_eq!(super::models_dir(root.path()), root.path().join("models"));
    }
}
