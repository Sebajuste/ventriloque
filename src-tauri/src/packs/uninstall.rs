// Retirer ce qu'un paquet a pose, puis son inscription.
//
// LA LISTE DU MANIFESTE N'EST PAS CRUE SUR PAROLE. C'est un fichier sur le disque, modifiable a
// la main : chaque entree repasse par `safe_entry` avant qu'on efface quoi que ce soit, tout
// comme une entree de zip a l'installation. Un manifeste bricole ne peut donc pas faire effacer
// autre chose que ce qu'un paquet aurait eu le droit de poser.
//
// CE QU'UN AUTRE PAQUET REVENDIQUE RESTE. Deux paquets peuvent livrer le meme fichier -- deux
// jeux d'un meme univers, un modele partage. Retirer l'un ne doit pas casser l'autre.

use anyhow::{Result, anyhow};
use std::path::{Path, PathBuf};

use crate::paths;

use super::entry::safe_entry;
use super::job::Job;
use super::store;

/// Rend le nombre de fichiers effaces.
pub fn uninstall(root: &Path, name: &str, job: &Job) -> Result<usize> {
    let all = store::installed(root);
    let manifest = all
        .iter()
        .find(|m| m.name == name)
        .ok_or_else(|| anyhow!("paquet inconnu : {name}"))?;

    let claimed_elsewhere: Vec<&String> = all
        .iter()
        .filter(|m| m.name != name)
        .flat_map(|m| m.files.iter())
        .collect();

    job.start(&manifest.name, manifest.files.len());
    let mut removed = 0usize;
    for (index, file) in manifest.files.iter().enumerate() {
        job.advance(index);
        job.set_step(file);
        let Some(relative) = safe_entry(file) else { continue };
        if claimed_elsewhere.contains(&file) {
            continue;
        }
        if std::fs::remove_file(root.join(&relative)).is_ok() {
            removed += 1;
            job.log(file);
        }
        remove_clone_cache(root, &relative);
    }

    job.advance(manifest.files.len());
    store::forget(root, manifest)?;
    Ok(removed)
}

/// Le cache de clonage vit sous le seul radical du fichier. Le laisser derriere serait le piege
/// documente : un paquet ulterieur qui livre le meme nom heriterait d'un etat qui n'est pas le
/// sien.
fn remove_clone_cache(root: &Path, relative: &PathBuf) {
    if !relative.extension().is_some_and(|e| e.eq_ignore_ascii_case("wav")) {
        return;
    }
    let Some(stem) = relative.file_stem() else { return };
    for extension in ["emb", "kv"] {
        let _ = std::fs::remove_file(
            root.join(paths::VOICES).join(".cache").join(stem).with_extension(extension),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::install::install;
    use crate::packs::testing::zip_with;
    use crate::testing::TempDir;

    #[test]
    fn desinstalle_ce_qui_a_ete_pose_et_le_cache_qui_va_avec() {
        let root = TempDir::new("uninstall");
        let zip = zip_with(
            &root,
            "essai.zip",
            &[
                ("pack.json", r#"{"name":"essai"}"#),
                ("voices/barman.wav", "RIFF"),
                ("characters/barman.json", "{}"),
            ],
        );
        install(root.path(), &zip, &Job::default()).unwrap();

        // Le moteur aurait laisse ceci a cote de la reference.
        let cache = root.path().join("voices/.cache");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("barman.emb"), "x").unwrap();

        assert_eq!(uninstall(root.path(), "essai", &Job::default()).unwrap(), 2);
        assert!(!root.path().join("voices/barman.wav").exists());
        assert!(!root.path().join("characters/barman.json").exists());
        assert!(!cache.join("barman.emb").exists());
        assert!(!root.path().join("packs/essai.json").exists());
        assert!(store::installed(root.path()).is_empty());
    }

    // Deux paquets peuvent livrer le meme fichier. Retirer l'un ne doit pas desarmer l'autre.
    #[test]
    fn laisse_ce_qu_un_autre_paquet_revendique() {
        let root = TempDir::new("uninstall-partage");
        let shared = zip_with(
            &root,
            "commun.zip",
            &[("pack.json", r#"{"name":"commun"}"#), ("voices/partagee.wav", "RIFF")],
        );
        let other = zip_with(
            &root,
            "autre.zip",
            &[("pack.json", r#"{"name":"autre"}"#), ("voices/partagee.wav", "RIFF")],
        );
        install(root.path(), &shared, &Job::default()).unwrap();
        install(root.path(), &other, &Job::default()).unwrap();

        assert_eq!(uninstall(root.path(), "commun", &Job::default()).unwrap(), 0);
        assert!(root.path().join("voices/partagee.wav").is_file());
        assert_eq!(store::installed(root.path()).len(), 1);
    }

    // Le manifeste est un fichier ordinaire : quelqu'un peut y ecrire ce qu'il veut.
    #[test]
    fn un_manifeste_bricole_ne_fait_pas_sortir_de_la_racine() {
        let root = TempDir::new("uninstall-bricole");
        let zip = zip_with(
            &root,
            "mechant.zip",
            &[("pack.json", r#"{"name":"mechant"}"#), ("voices/vrai.wav", "RIFF")],
        );
        install(root.path(), &zip, &Job::default()).unwrap();

        let witness = root.path().join("ne-pas-toucher.txt");
        std::fs::write(&witness, "je reste").unwrap();
        std::fs::write(
            root.path().join("packs/mechant.json"),
            r#"{"name":"mechant","files":["../ne-pas-toucher.txt","voices/../ne-pas-toucher.txt","voices/vrai.wav"]}"#,
        )
        .unwrap();

        assert_eq!(uninstall(root.path(), "mechant", &Job::default()).unwrap(), 1);
        assert!(witness.is_file(), "un chemin qui remonte a ete suivi");
    }

    #[test]
    fn un_paquet_inconnu_se_plaint_au_lieu_d_effacer() {
        let root = TempDir::new("uninstall-inconnu");
        assert!(uninstall(root.path(), "fantome", &Job::default()).is_err());
    }
}
