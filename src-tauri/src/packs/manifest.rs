// Le manifeste d'un paquet : ce que son auteur declare, et ce que l'installation y ajoute.
//
// LE FORMAT D'AVANT SE LIT ENCORE. Les manifestes portaient des champs francais, et les paquets
// deja distribues en sont pleins : ils s'installent encore -- voir `legacy` -- et leur
// inscription se reecrit sous les nouveaux noms.

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, Default, specta::Type)]
pub struct Manifest {
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    /// Le script d'extraction, nomme sans son dossier : « starcraft2.rhai ». Vide pour un paquet
    /// qui porte deja ses voix.
    #[serde(default)]
    pub recipe: String,
    /// Ce que la recette attend comme jeu, pour le dire a l'utilisateur avant qu'il cherche le
    /// dossier : « StarCraft II ».
    #[serde(default)]
    pub game: String,
    /// Le code du jeu tel que `.build.info` le porte -- « sc2 », « fenris » --, qui permet de
    /// trouver l'installation sans rien demander. Voir `games` : ce n'est PAS le code que rend
    /// CascLib, qui dit « s2 » pour le meme jeu.
    #[serde(default)]
    pub product: String,
    /// Le fichier dont la presence signe l'installation, pour les jeux qui ne portent pas de
    /// `.build.info` : « bin/x64/Cyberpunk2077.exe ». Chemin RELATIF au dossier du jeu.
    ///
    /// C'est le paquet qui l'apporte, pas l'application : une recette pour un nouveau jeu ne
    /// demande donc aucune modification du code.
    #[serde(default)]
    pub marker: String,
    /// Rempli a l'installation, pas par l'auteur du paquet.
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub installed_on: String,
    /// Rempli par la fabrication. Vide tant qu'un paquet-recette n'a pas tourne.
    #[serde(default)]
    pub built_on: String,
}

impl Manifest {
    /// L'identifiant de fichier sous lequel le paquet est inscrit dans `packs\`.
    pub fn id(&self) -> String {
        let raw: String = self
            .name
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        raw.trim_matches('_').to_string()
    }

    /// Un paquet-recette qui n'a pas encore tourne : il n'apporte rien tant qu'on ne l'a pas
    /// fabrique depuis une copie du jeu.
    pub fn needs_build(&self) -> bool {
        !self.recipe.is_empty() && self.built_on.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn l_identifiant_tient_dans_un_nom_de_fichier() {
        let named = |name: &str| Manifest { name: name.into(), ..Manifest::default() }.id();
        assert_eq!(named("StarCraft II"), "starcraft_ii");
        assert_eq!(named("Moteur de parole francais"), "moteur_de_parole_francais");
        assert_eq!(named("  --Bord--  "), "bord");
    }

    #[test]
    fn un_paquet_recette_qui_n_a_pas_tourne_reste_a_fabriquer() {
        let mut m = Manifest { recipe: "sc2.rhai".into(), ..Manifest::default() };
        assert!(m.needs_build());
        m.built_on = "2026-09-06".into();
        assert!(!m.needs_build());
        // Un paquet qui porte ses voix n'a jamais rien a fabriquer.
        assert!(!Manifest::default().needs_build());
    }
}
