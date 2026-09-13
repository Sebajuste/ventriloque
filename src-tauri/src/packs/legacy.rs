// Les manifestes ecrits sous les anciens noms de champs.
//
// POURQUOI UN TYPE A PART, ET PAS DES `#[serde(alias)]`. Meme raison que pour les fiches :
// `specta` decrit un champ a deux noms par une union, et le contrat TypeScript de `Manifest`
// devenait une alternative de dix-sept termes que plus aucune vue ne savait manipuler. Le format
// DE LECTURE d'un fichier et le format DE L'IPC sont deux choses.
//
// CE FICHIER SERT AUX PAQUETS DEJA DISTRIBUES. Un zip parti chez les gens porte les anciens noms
// pour toujours : il doit s'installer, et son inscription se reecrit au format d'aujourd'hui.

use serde::Deserialize;

use super::manifest::Manifest;

#[derive(Deserialize)]
struct LegacyManifest {
    #[serde(default)]
    nom: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    auteur: String,
    #[serde(default)]
    recette: String,
    #[serde(default)]
    jeu: String,
    #[serde(default)]
    produit: String,
    #[serde(default)]
    fichiers: Vec<String>,
    #[serde(default)]
    installe_le: String,
    #[serde(default)]
    fabrique_le: String,
}

impl From<LegacyManifest> for Manifest {
    fn from(old: LegacyManifest) -> Self {
        Manifest {
            name: old.nom,
            version: old.version,
            description: old.description,
            author: old.auteur,
            recipe: old.recette,
            game: old.jeu,
            product: old.produit,
            marker: String::new(),
            files: old.fichiers,
            installed_on: old.installe_le,
            built_on: old.fabrique_le,
        }
    }
}

/// Lit un manifeste, dans le format d'aujourd'hui ou celui d'avant.
///
/// L'ORDRE COMPTE : le format d'aujourd'hui d'abord. Un manifeste recent n'a pas de champ `nom`,
/// donc l'ancien format le lirait en un paquet sans nom plutot que d'echouer.
pub fn parse(text: &str) -> Option<Manifest> {
    if let Ok(manifest) = serde_json::from_str::<Manifest>(text)
        && !manifest.name.trim().is_empty()
    {
        return Some(manifest);
    }
    serde_json::from_str::<LegacyManifest>(text)
        .ok()
        .map(Manifest::from)
        .filter(|m| !m.name.trim().is_empty())
}

#[cfg(test)]
mod tests {
    #[test]
    fn un_manifeste_d_aujourd_hui_se_lit() {
        let m = super::parse(r#"{"name":"essai","version":"1","files":["voices/a.wav"]}"#).unwrap();
        assert_eq!(m.name, "essai");
        assert_eq!(m.version, "1");
        assert_eq!(m.files, vec!["voices/a.wav".to_string()]);
    }

    // Un paquet deja distribue porte les champs francais.
    #[test]
    fn un_manifeste_d_avant_se_lit_aussi() {
        let m = super::parse(
            r#"{"nom":"StarCraft II","auteur":"moi","recette":"sc2.rhai","jeu":"StarCraft II",
                "produit":"sc2","fichiers":["voix/a.wav"],"installe_le":"2026-09-06",
                "fabrique_le":"2026-09-06"}"#,
        )
        .unwrap();

        assert_eq!(m.name, "StarCraft II");
        assert_eq!(m.author, "moi");
        assert_eq!(m.recipe, "sc2.rhai");
        assert_eq!(m.game, "StarCraft II");
        assert_eq!(m.product, "sc2");
        assert_eq!(m.files, vec!["voix/a.wav".to_string()]);
        assert_eq!(m.installed_on, "2026-09-06");
        assert_eq!(m.built_on, "2026-09-06");
    }

    #[test]
    fn ce_qui_ne_nomme_aucun_paquet_n_est_pas_un_manifeste() {
        assert!(super::parse("{}").is_none());
        assert!(super::parse(r#"{"name":"  "}"#).is_none());
        assert!(super::parse("{ pas du json").is_none());
    }
}
