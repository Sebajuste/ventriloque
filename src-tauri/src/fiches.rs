// Les fiches de PNJ : un fichier par personnage, sur le disque.
//
// UN FICHIER CHACUN, et pas un catalogue unique. Une fiche se lit, se corrige a la main, se
// copie d'un univers a l'autre et se met dans un depot sans que les autres bougent. Un seul
// gros fichier aurait fait de chaque modification une reecriture de tout.
//
// CE QUE PORTE UNE FICHE, et rien de plus : qui parle, avec quelle voix, et les repliques qu'on
// redit souvent. Le reste -- l'histoire du personnage, ses liens, ses secrets -- est deja dans
// les notes du MJ et n'a pas a etre ressaisi ici.

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug, specta::Type)]
pub struct Fiche {
    // Deduit du nom, et c'est aussi le nom du fichier.
    #[serde(default)]
    pub id: String,
    pub nom: String,
    #[serde(default)]
    pub univers: String,
    // La reference telle que le moteur l'attend : `judy.wav` pour un clone, `jean` pour une voix
    // de catalogue. La fiche ne sait pas lequel des deux c'est, et n'a pas besoin de le savoir.
    #[serde(default)]
    pub voix: String,
    // Ce qu'on redit souvent : un prix, une menace, une formule d'accueil. Cliquer la replique
    // la fait dire tout de suite -- c'est le geste qui rend l'outil utilisable en pleine partie,
    // ou l'on n'a pas le temps de taper.
    #[serde(default)]
    pub repliques: Vec<String>,
}

// Le nom devient un nom de fichier. Tout ce qui n'est pas une lettre ou un chiffre devient un
// souligne, ce qui evite d'un coup les accents dans les chemins, les separateurs, et les noms
// reserves de Windows.
pub fn identifiant(nom: &str) -> String {
    let brut: String = nom
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    brut.trim_matches('_').to_string()
}

fn chemin(dossier: &Path, id: &str) -> PathBuf {
    dossier.join(format!("{id}.json"))
}

pub fn toutes(dossier: &Path) -> Vec<Fiche> {
    let mut fiches: Vec<Fiche> = std::fs::read_dir(dossier)
        .map(|entrees| {
            entrees
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                .filter_map(|e| {
                    let texte = std::fs::read_to_string(e.path()).ok()?;
                    let mut fiche: Fiche = serde_json::from_str(&texte).ok()?;
                    // Le nom du fichier fait foi : une fiche copiee a la main sous un autre nom
                    // ne se retrouve pas avec l'identifiant de celle dont elle vient.
                    fiche.id = e.path().file_stem()?.to_str()?.to_string();
                    Some(fiche)
                })
                .collect()
        })
        .unwrap_or_default();
    fiches.sort_by(|a, b| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()));
    fiches
}

// Ecrit la fiche, et renomme le fichier si le nom du personnage a change.
pub fn ecrire(dossier: &Path, mut fiche: Fiche, ancien: &str) -> Result<Fiche> {
    if fiche.nom.trim().is_empty() {
        return Err(anyhow!("il faut nommer le personnage"));
    }
    fiche.id = identifiant(&fiche.nom);
    if fiche.id.is_empty() {
        return Err(anyhow!("ce nom ne donne aucun nom de fichier utilisable"));
    }
    fiche.repliques.retain(|r| !r.trim().is_empty());

    std::fs::create_dir_all(dossier).ok();
    let texte = serde_json::to_string_pretty(&fiche).context("mise en forme de la fiche")?;
    std::fs::write(chemin(dossier, &fiche.id), texte).context("ecriture de la fiche")?;

    // Renommer laisse un doublon sous l'ancien nom si on ne le retire pas.
    if !ancien.is_empty() && ancien != fiche.id {
        let _ = std::fs::remove_file(chemin(dossier, ancien));
    }
    Ok(fiche)
}

pub fn supprimer(dossier: &Path, id: &str) -> Result<()> {
    std::fs::remove_file(chemin(dossier, id)).context("suppression de la fiche")
}
