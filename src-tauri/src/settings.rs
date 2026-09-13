// Les deux fichiers de la racine : le repere, et les reglages du moteur.
//
// DEUX FICHIERS PARCE QU'ILS N'ONT PAS LE MEME PROPRIETAIRE, et c'est la seule chose a retenir
// d'ici. `ventriloque.json` est ECRIT PAR L'UTILISATEUR et versionne dans le depot : il marque la
// racine et peut renvoyer les modeles ailleurs. `tuning.json` est ECRIT PAR L'APPLICATION, a
// chaque « Appliquer » de la page des reglages, et il est ignore par git.
//
// Les reglages ont d'abord vecu sous une cle du repere. Consequence : regler une temperature en
// seance salissait le depot, et le reglage local d'un poste partait dans un commit. Un fichier
// que l'application reecrit n'a pas a etre versionne, et un fichier versionne n'a pas a etre
// reecrit sous les pieds de celui qui l'edite -- `migration` sort la cle des installations qui
// l'ont encore.
//
// PAR DEFAUT, `models\` A COTE DES DONNEES -- ce que le paquet moteur remplit, et le seul
// endroit qui existe sur une machine neuve. Une installation se copie donc telle quelle.
//
// Le fichier peut nommer un autre dossier, ce qui sert pendant le developpement pour partager un
// demi-gigaoctet entre plusieurs copies du depot. MAIS CE N'EST QU'UN INDICE : un chemin qui ne
// repond pas est ignore, pas suivi. Sans cela, un `ventriloque.json` copie d'une machine a
// l'autre rendrait l'application muette en pointant un dossier qui n'existe que chez l'autre --
// et le message parlerait d'un chemin inconnu de celui qui le lit.
//
// CHAQUE REGLAGE SE LIT A PART : un champ mal forme retombe sur sa valeur d'origine sans
// emporter les autres avec lui.

use anyhow::Result;
use serde::{Deserialize, Serialize};
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

// CE QUE LE MOTEUR RECOIT EN LIGNE DE COMMANDE, et rien d'autre : il ne lit ces valeurs qu'a son
// lancement, donc les changer veut dire le relancer.
//
// Les valeurs d'origine sont celles que le mod ai_npc a validees en jeu
// (`ptt_create(..., "int8", 0.7f, 1, 0)`). Ce sont les seules qu'on ait entendues tourner : une
// installation sans `tuning.json` doit donc les redonner exactement.
//
// `f64` ET PAS `f32` : la valeur passe par JSON, et un `0.6f32` s'y ecrit `0.6000000238418579`
// -- dans le fichier comme dans la fenetre.
#[derive(Serialize, Deserialize, specta::Type, Clone, Copy, Debug, PartialEq)]
#[serde(default)]
pub struct EngineSettings {
    /// La variance du bruit tire a chaque trame. Plus bas, la voix colle mieux a la reference et
    /// varie moins d'une replique a l'autre ; plus haut, elle vit davantage et derive plus.
    pub temperature: f64,
    /// Les pas de l'echantillonneur par trame. Un pas est le plus rapide ; plusieurs corrigent
    /// mieux un tirage malheureux, au prix du calcul.
    pub lsd_steps: u32,
    /// Borne du bruit tire, en ecarts-types. 0 la retire.
    pub noise_clamp: f64,
    /// Le seuil au-dela duquel le moteur juge la phrase finie. Plus bas, il coupe plus tot.
    pub eos_threshold: f64,
    /// Les trames laissees courir apres la fin de phrase. `None` : ce que dit le paquet de modeles.
    pub eos_extra: Option<i32>,
    /// Les fils de calcul. 0 : la moitie des coeurs -- un seul fil est le pire reglage possible.
    pub threads: u32,
}

impl Default for EngineSettings {
    fn default() -> Self {
        Self {
            temperature: 0.7,
            lsd_steps: 1,
            noise_clamp: 0.0,
            eos_threshold: -4.0,
            eos_extra: None,
            threads: 0,
        }
    }
}

impl EngineSettings {
    /// Ramene chaque valeur dans ce que le moteur supporte. Le fichier s'edite a la main : un
    /// `lsd_steps` a zero y ferait diviser par zero, une temperature negative prendre la racine
    /// d'un negatif.
    pub fn sanitized(self) -> Self {
        let origin = Self::default();
        let within = |value: f64, low: f64, high: f64, fallback: f64| {
            if value.is_finite() { value.clamp(low, high) } else { fallback }
        };
        Self {
            temperature: within(self.temperature, 0.05, 1.5, origin.temperature),
            lsd_steps: self.lsd_steps.clamp(1, 10),
            noise_clamp: within(self.noise_clamp, 0.0, 5.0, origin.noise_clamp),
            eos_threshold: within(self.eos_threshold, -10.0, 0.0, origin.eos_threshold),
            eos_extra: self.eos_extra.map(|frames| frames.clamp(0, 30)),
            threads: self.threads.min(64),
        }
    }
}

/// Les reglages du moteur, ou ceux d'origine pour ce qui manque ou ne se lit pas.
///
/// UN `tuning.json` ILLISIBLE REND LES VALEURS D'ORIGINE au lieu d'une erreur, et la prochaine
/// ecriture l'ecrase. Il ne porte que ces six nombres : il n'y a rien a sauver dedans, et refuser
/// d'ecrire laisserait la page des reglages sans issue jusqu'a une reparation a la main. Le
/// repere, lui, porte le texte de l'utilisateur -- c'est pourquoi `migration` ne l'ecrase pas.
pub fn engine(root: &Path) -> EngineSettings {
    std::fs::read_to_string(root.join(paths::TUNING))
        .ok()
        .and_then(|text| serde_json::from_str::<EngineSettings>(&text).ok())
        .unwrap_or_default()
        .sanitized()
}

/// Ecrit les reglages du moteur, et rend ce qui a ete ecrit -- borne, donc pas toujours ce qui
/// etait demande.
pub fn save_engine(root: &Path, settings: EngineSettings) -> Result<EngineSettings> {
    let settings = settings.sanitized();
    let text = serde_json::to_string_pretty(&settings)? + "\n";
    std::fs::write(root.join(paths::TUNING), text)?;
    Ok(settings)
}

// Un chemin de modeles qui ne repond pas ne doit pas rendre l'application muette : c'est ce qui
// arrive des qu'un `ventriloque.json` passe d'une machine a l'autre.
#[cfg(test)]
mod tests {
    use super::EngineSettings;
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

    // Le repere peut exister sans rien regler : c'est son cas le plus courant.
    #[test]
    fn un_fichier_vide_reste_un_reperage_valable() {
        let root = TempDir::new("settings-vide");
        std::fs::write(root.path().join("ventriloque.json"), "{}").unwrap();

        assert_eq!(super::models_dir(root.path()), root.path().join("models"));
        assert_eq!(super::engine(root.path()), EngineSettings::default());
    }

    // CE QUI MOTIVE LES DEUX FICHIERS : le repere est versionne, regler le moteur ne doit pas
    // le salir.
    #[test]
    fn regler_le_moteur_ne_touche_pas_au_repere() {
        let root = TempDir::new("settings-intact");
        let marker = root.path().join("ventriloque.json");
        let before = r#"{"_": "un mot", "models": "D:/ailleurs"}"#;
        std::fs::write(&marker, before).unwrap();

        super::save_engine(root.path(), EngineSettings { temperature: 0.55, ..Default::default() })
            .unwrap();

        assert_eq!(std::fs::read_to_string(&marker).unwrap(), before);
        assert!(root.path().join("tuning.json").is_file());
    }

    // Les valeurs d'origine sont la configuration entendue en jeu : un fichier qui ne dit rien
    // doit la rendre telle quelle.
    #[test]
    fn sans_cle_engine_le_moteur_tourne_comme_en_jeu() {
        let root = TempDir::new("settings-origine");
        let origin = super::engine(root.path());

        assert_eq!(origin.temperature, 0.7);
        assert_eq!(origin.lsd_steps, 1);
        assert_eq!(origin.eos_extra, None);
    }

    #[test]
    fn un_reglage_absent_garde_sa_valeur_d_origine() {
        let root = TempDir::new("settings-partiel");
        std::fs::write(root.path().join("tuning.json"), r#"{"temperature": 0.5}"#).unwrap();

        let read = super::engine(root.path());
        assert_eq!(read.temperature, 0.5);
        assert_eq!(read.lsd_steps, 1);
    }

    #[test]
    fn ce_qui_est_ecrit_se_relit_tel_quel() {
        let root = TempDir::new("settings-ecrit");
        let wanted = EngineSettings { temperature: 0.55, eos_extra: Some(6), ..Default::default() };

        super::save_engine(root.path(), wanted).unwrap();

        let text = std::fs::read_to_string(root.path().join("tuning.json")).unwrap();
        // Pas de `0.550000011920929` : la valeur se relit comme on l'a tapee.
        assert!(text.contains("0.55"), "{text}");
        assert_eq!(super::engine(root.path()), wanted);
    }

    // `tuning.json` ne porte que ces six nombres : il n'y a rien a sauver dedans, et refuser
    // d'ecrire laisserait la page des reglages sans issue.
    #[test]
    fn un_tuning_illisible_rend_les_valeurs_d_origine_et_se_laisse_reecrire() {
        let root = TempDir::new("settings-illisible");
        let path = root.path().join("tuning.json");
        std::fs::write(&path, "{ pas du json").unwrap();

        assert_eq!(super::engine(root.path()), EngineSettings::default());
        assert!(super::save_engine(root.path(), EngineSettings::default()).is_ok());
        assert_eq!(super::engine(root.path()), EngineSettings::default());
    }

    // A la main, on peut ecrire n'importe quoi : le moteur ne doit recevoir que ce qu'il tient.
    #[test]
    fn une_valeur_hors_bornes_est_ramenee() {
        let wild = EngineSettings {
            temperature: 9.0,
            lsd_steps: 0,
            noise_clamp: -1.0,
            eos_threshold: f64::NAN,
            eos_extra: Some(-3),
            threads: 500,
        }
        .sanitized();

        assert_eq!(wild.temperature, 1.5);
        assert_eq!(wild.lsd_steps, 1);
        assert_eq!(wild.noise_clamp, 0.0);
        assert_eq!(wild.eos_threshold, -4.0);
        assert_eq!(wild.eos_extra, Some(0));
        assert_eq!(wild.threads, 64);
    }
}
