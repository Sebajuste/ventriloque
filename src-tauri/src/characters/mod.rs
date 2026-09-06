// Les fiches de personnages : un fichier par PNJ, sur le disque.
//
// UN FICHIER CHACUN, et pas un catalogue unique. Une fiche se lit, se corrige a la main, se
// copie d'un univers a l'autre et se met dans un depot sans que les autres bougent. Un seul
// gros fichier aurait fait de chaque modification une reecriture de tout.
//
// CE QUE PORTE UNE FICHE, et rien de plus : qui parle, avec quelle voix, et les repliques qu'on
// redit souvent. Le reste -- l'histoire du personnage, ses liens, ses secrets -- est deja dans
// les notes du MJ et n'a pas a etre ressaisi ici.
//
// LE FORMAT D'AVANT SE LIT ENCORE. Les fiches portaient des champs francais ; celles qui
// trainent sur un disque ou dans un paquet deja distribue se lisent toujours -- voir `legacy` --
// et se reecrivent sous les nouveaux noms des qu'on les enregistre.

mod legacy;

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, Default, specta::Type)]
pub struct Character {
    /// Deduit du nom, et c'est aussi le nom du fichier.
    #[serde(default)]
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub universe: String,
    /// La reference telle que le moteur l'attend : `judy.wav` pour un clone, `jean` pour une voix
    /// de catalogue. La fiche ne sait pas lequel des deux c'est, et n'a pas besoin de le savoir.
    #[serde(default)]
    pub voice: String,
    /// Ce qu'on redit souvent : un prix, une menace, une formule d'accueil. Cliquer la replique
    /// la fait dire tout de suite -- c'est le geste qui rend l'outil utilisable en pleine partie,
    /// ou l'on n'a pas le temps de taper.
    #[serde(default)]
    pub lines: Vec<String>,
}

/// Le nom devient un nom de fichier. Tout ce qui n'est pas une lettre ou un chiffre devient un
/// souligne, ce qui evite d'un coup les accents dans les chemins, les separateurs, et les noms
/// reserves de Windows.
pub fn slug(name: &str) -> String {
    let raw: String = name
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    raw.trim_matches('_').to_string()
}

fn path_of(folder: &Path, id: &str) -> PathBuf {
    folder.join(format!("{id}.json"))
}

/// Toutes les fiches d'un dossier, triees par nom.
pub fn all(folder: &Path) -> Vec<Character> {
    let mut characters: Vec<Character> = std::fs::read_dir(folder)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                .filter_map(|e| {
                    let text = std::fs::read_to_string(e.path()).ok()?;
                    let mut character = legacy::parse(&text)?;
                    // Le nom du fichier fait foi : une fiche copiee a la main sous un autre nom
                    // ne se retrouve pas avec l'identifiant de celle dont elle vient.
                    character.id = e.path().file_stem()?.to_str()?.to_string();
                    Some(character)
                })
                .collect()
        })
        .unwrap_or_default();
    characters.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    characters
}

/// Ecrit la fiche, et renomme le fichier si le nom du personnage a change.
pub fn write(folder: &Path, mut character: Character, previous_id: &str) -> Result<Character> {
    if character.name.trim().is_empty() {
        return Err(anyhow!("il faut nommer le personnage"));
    }
    character.id = slug(&character.name);
    if character.id.is_empty() {
        return Err(anyhow!("ce nom ne donne aucun nom de fichier utilisable"));
    }
    character.lines.retain(|line| !line.trim().is_empty());

    std::fs::create_dir_all(folder).ok();
    let text = serde_json::to_string_pretty(&character).context("mise en forme de la fiche")?;
    std::fs::write(path_of(folder, &character.id), text).context("ecriture de la fiche")?;

    // Renommer laisse un doublon sous l'ancien nom si on ne le retire pas.
    if !previous_id.is_empty() && previous_id != character.id {
        let _ = std::fs::remove_file(path_of(folder, previous_id));
    }
    Ok(character)
}

pub fn delete(folder: &Path, id: &str) -> Result<()> {
    std::fs::remove_file(path_of(folder, id)).context("suppression de la fiche")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    #[test]
    fn le_nom_devient_un_nom_de_fichier_sans_surprise() {
        assert_eq!(slug("Sarah Kerrigan"), "sarah_kerrigan");
        assert_eq!(slug("  Judy Álvarez  "), "judy_álvarez");
        assert_eq!(slug("A/B\\C"), "a_b_c");
        assert_eq!(slug("---"), "");
    }

    #[test]
    fn une_fiche_ecrite_se_relit() {
        let dir = TempDir::new("characters");
        let written = write(
            dir.path(),
            Character {
                name: "Le Barman".into(),
                universe: "Warhammer".into(),
                voice: "barman.wav".into(),
                lines: vec!["Trois couronnes.".into()],
                ..Character::default()
            },
            "",
        )
        .unwrap();

        assert_eq!(written.id, "le_barman");
        let all = all(dir.path());
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].name, "Le Barman");
        assert_eq!(all[0].lines, vec!["Trois couronnes.".to_string()]);
    }

    // Renommer laisserait un doublon sous l'ancien nom si rien ne le retirait.
    #[test]
    fn renommer_ne_laisse_pas_de_doublon() {
        let dir = TempDir::new("characters-renomme");
        let first = write(
            dir.path(),
            Character { name: "Judy".into(), ..Character::default() },
            "",
        )
        .unwrap();

        write(
            dir.path(),
            Character { name: "Judy Alvarez".into(), ..Character::default() },
            &first.id,
        )
        .unwrap();

        let all = all(dir.path());
        assert_eq!(all.len(), 1, "{all:?}");
        assert_eq!(all[0].id, "judy_alvarez");
    }

    #[test]
    fn une_fiche_sans_nom_est_refusee() {
        let dir = TempDir::new("characters-sans-nom");
        assert!(write(dir.path(), Character::default(), "").is_err());
        // Un nom qui ne donne aucun fichier utilisable non plus.
        let mute = Character { name: "///".into(), ..Character::default() };
        assert!(write(dir.path(), mute, "").is_err());
    }

    #[test]
    fn les_repliques_vides_ne_sont_pas_gardees() {
        let dir = TempDir::new("characters-repliques");
        let written = write(
            dir.path(),
            Character {
                name: "Nova".into(),
                lines: vec!["Vraie".into(), "   ".into(), String::new()],
                ..Character::default()
            },
            "",
        )
        .unwrap();

        assert_eq!(written.lines, vec!["Vraie".to_string()]);
    }

    #[test]
    fn supprimer_retire_le_fichier() {
        let dir = TempDir::new("characters-suppression");
        write(dir.path(), Character { name: "Nova".into(), ..Character::default() }, "").unwrap();

        delete(dir.path(), "nova").unwrap();

        assert!(all(dir.path()).is_empty());
        assert!(delete(dir.path(), "nova").is_err(), "une fiche absente doit se plaindre");
    }
}
