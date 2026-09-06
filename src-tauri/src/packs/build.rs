// La fabrication : faire tourner le script d'un paquet sur une copie installee d'un jeu.
//
// LE SCRIPT NE TOURNE PAS ICI. Il tourne dans `fabriquer.exe`, un processus separe, lie a notre
// vie par un job object comme le moteur de parole. Deux codes qui ne viennent pas de
// l'application -- le C++ de CascLib et le script du paquet -- restent ainsi dehors : ils ne
// peuvent pas lire notre memoire, et se tuent d'un geste.
//
// CE QUI SORT EST TRAITE COMME CE QUI SORT D'UN ZIP. Le processus fils ecrit dans un dossier de
// travail jetable ; rien n'entre dans `voices\` ou `characters\` sans passer par le meme
// controle qu'une entree de zip. Un script qui rendrait `..\..\Windows\System32\x.wav` ne pose
// rien.
//
// LE JEU N'EST JAMAIS TOUCHE. Le fils l'ouvre en lecture, et n'y ecrit pas.

use anyhow::{Context, Result, anyhow};
use std::io::BufRead;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::{clock, paths};

use super::job::Job;
use super::manifest::Manifest;
use super::store;

/// Ce qu'une fabrication a produit, tel qu'on le raconte dans la fenetre.
pub struct BuildReport {
    pub log: Vec<String>,
    pub produced: Vec<String>,
}

/// Ce que le fils a le droit de poser, ou le chercher, et ou le mettre.
///
/// DEUX NOMS DE SOURCE PAR DESTINATION, parce que le fabricant est un binaire a part : il se
/// rebatit et se redeploie a son propre rythme, et celui qui est deja pose sur une machine ecrit
/// encore dans `voix\` et `pnj\`. Chercher les deux coute une lecture de dossier absent et
/// evite qu'une fabrication rende « rien pose d'utilisable » sur une simple difference de
/// vocabulaire.
const HARVEST: [(&[&str], &str, &str); 2] = [
    (&[paths::VOICES, "voix"], ".wav", paths::VOICES),
    (&[paths::CHARACTERS, "pnj"], ".json", paths::CHARACTERS),
];

/// Un nom de fichier rendu par le fils, accepte seulement s'il est ordinaire : pas de dossier,
/// pas de `..`, rien d'invisible. C'est la meme severite que pour une entree de zip.
fn safe_file_name(raw: &str) -> Option<String> {
    let acceptable = !raw.is_empty()
        && raw.len() <= 80
        && !raw.starts_with('.')
        && !raw.contains("..")
        && raw.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');
    acceptable.then(|| raw.to_string())
}

/// Deplace ce que le fils a produit vers les dossiers de l'application.
///
/// `extension` dit ce qui a le droit d'atterrir dans quel dossier : une recette ne peut pas
/// glisser un `.exe` dans `voices\` en le nommant bien.
fn harvest(work: &Path, root: &Path, from: &str, extension: &str, into: &str) -> Result<Vec<String>> {
    let source = work.join(from);
    let target = root.join(into);
    std::fs::create_dir_all(&target).ok();

    let mut placed = Vec::new();
    let Ok(entries) = std::fs::read_dir(&source) else {
        return Ok(placed);
    };
    for entry in entries.flatten() {
        if !entry.path().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(safe) = safe_file_name(&name) else { continue };
        if !safe.to_lowercase().ends_with(extension) {
            continue;
        }
        std::fs::copy(entry.path(), target.join(&safe))
            .with_context(|| format!("pose de {into}/{safe}"))?;
        placed.push(format!("{into}/{safe}"));
    }
    placed.sort();
    Ok(placed)
}

/// Lance la fabrication et moissonne. Bloquant : compter deux a quatre minutes sur StarCraft II,
/// dont une pour recenser les 780 000 entrees du stockage.
pub fn build(
    root: &Path,
    builder: &Path,
    manifest: &mut Manifest,
    game: &Path,
    job: &Job,
) -> Result<BuildReport> {
    let script = locate_script(root, manifest)?;
    if !builder.is_file() {
        return Err(anyhow!("le fabricant n'est pas depose : {}", builder.display()));
    }
    if !game.is_dir() {
        return Err(anyhow!("ce n'est pas un dossier : {}", game.display()));
    }

    // Un dossier de travail jetable, hors des dossiers de donnees : ce que le fils ecrit n'est
    // pas encore du bien de l'application.
    let work = root.join(paths::WORK).join(manifest.id());
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).context("creation du dossier de fabrication")?;

    let log = run_builder(builder, &script, game, &work, job)?;

    job.set_step("moisson…");
    let mut produced = Vec::new();
    for (sources, extension, into) in HARVEST {
        for from in sources {
            produced.extend(harvest(&work, root, from, extension, into)?);
        }
    }
    produced.sort();
    produced.dedup();
    let _ = std::fs::remove_dir_all(&work);

    if produced.is_empty() {
        return Err(anyhow!("la fabrication n'a rien pose d'utilisable"));
    }

    // Le manifeste garde ce qui a ete pose, pour qu'une desinstallation sache quoi retirer, et
    // la date pour que l'onglet cesse de dire « a fabriquer ».
    for file in &produced {
        if !manifest.files.contains(file) {
            manifest.files.push(file.clone());
        }
    }
    manifest.built_on = clock::today();
    store::record(root, manifest)?;

    Ok(BuildReport { log, produced })
}

fn locate_script(root: &Path, manifest: &Manifest) -> Result<std::path::PathBuf> {
    if manifest.recipe.is_empty() {
        return Err(anyhow!("« {} » ne porte pas de recette", manifest.name));
    }
    let Some(name) = safe_file_name(&manifest.recipe) else {
        return Err(anyhow!("nom de recette refuse : {}", manifest.recipe));
    };
    let script = root.join(paths::RECIPES).join(&name);
    if !script.is_file() {
        return Err(anyhow!("recette introuvable : {}", script.display()));
    }
    Ok(script)
}

/// Lance le fils et rend son journal.
///
/// LE JOURNAL EST LU AU FIL DE L'EAU, pas ramasse a la fin. Une fabrication dure deux a quatre
/// minutes ; attendre la derniere ligne pour montrer la premiere laisserait la fenetre muette
/// tout ce temps, sans moyen de distinguer un travail qui avance d'un processus mort.
fn run_builder(
    builder: &Path,
    script: &Path,
    game: &Path,
    work: &Path,
    job: &Job,
) -> Result<Vec<String>> {
    let mut command = Command::new(builder);
    command
        // LES DRAPEAUX SONT CEUX DU FABRICANT, et il est un binaire a part : le renommer ici
        // n'aurait renomme que l'appel. Ils changeront quand le fabricant changera.
        .arg("--script").arg(script)
        .arg("--jeu").arg(game)
        .arg("--sortie").arg(work)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Sans cela, une console noire s'ouvre a cote de la fenetre.
        command.creation_flags(0x0800_0000);
    }

    let mut child = command.spawn().context("lancement du fabricant")?;
    #[cfg(windows)]
    crate::engine::tie_to_process_lifetime(&child);

    let mut log: Vec<String> = Vec::new();
    if let Some(output) = child.stdout.take() {
        for line in std::io::BufReader::new(output).lines().map_while(Result::ok) {
            let line = line.trim_end().to_string();
            if line.is_empty() {
                continue;
            }
            job.set_step(&line);
            job.log(&line);
            log.push(line);
        }
    }

    // `stdout` a deja ete pris : il ne reste que `stderr` a ramasser, et l'attente du fils.
    let outcome = child.wait_with_output().context("attente du fabricant")?;
    if !outcome.status.success() {
        let complaint = String::from_utf8_lossy(&outcome.stderr).trim().to_string();
        let _ = std::fs::remove_dir_all(work);
        return Err(anyhow!(if complaint.is_empty() {
            "la fabrication a echoue".to_string()
        } else {
            complaint
        }));
    }
    Ok(log)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::TempDir;

    #[test]
    fn les_noms_dangereux_sont_refuses() {
        assert!(safe_file_name("sc2_kerrigan.wav").is_some());
        assert!(safe_file_name("../dehors.wav").is_none());
        assert!(safe_file_name("voices/ailleurs.wav").is_none());
        assert!(safe_file_name(".cache").is_none());
        assert!(safe_file_name("").is_none());
        assert!(safe_file_name(&"x".repeat(81)).is_none());
    }

    // Ce qui n'a pas la bonne extension ne passe pas, meme bien nomme.
    #[test]
    fn la_moisson_trie_par_extension() {
        let base = TempDir::new("harvest");
        let work = base.child("work");
        let root = base.child("root");
        std::fs::create_dir_all(work.join("voices")).unwrap();
        std::fs::write(work.join("voices/bonne.wav"), b"RIFF").unwrap();
        std::fs::write(work.join("voices/mauvaise.exe"), b"MZ").unwrap();

        let placed = harvest(&work, &root, "voices", ".wav", "voices").unwrap();

        assert_eq!(placed, vec!["voices/bonne.wav".to_string()]);
        assert!(root.join("voices/bonne.wav").is_file());
        assert!(!root.join("voices/mauvaise.exe").exists());
    }

    #[test]
    fn un_dossier_de_travail_vide_ne_moissonne_rien() {
        let base = TempDir::new("harvest-vide");
        assert!(
            harvest(&base.child("work"), &base.child("root"), "voices", ".wav", "voices")
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn un_paquet_sans_recette_ne_se_fabrique_pas() {
        let root = TempDir::new("build-sans-recette");
        let e = locate_script(root.path(), &Manifest::default()).unwrap_err().to_string();
        assert!(e.contains("ne porte pas de recette"), "{e}");
    }

    #[test]
    fn une_recette_mal_nommee_ou_absente_est_refusee() {
        let root = TempDir::new("build-recette");
        let hostile = Manifest { recipe: "../dehors.rhai".into(), ..Manifest::default() };
        assert!(locate_script(root.path(), &hostile).unwrap_err().to_string().contains("refuse"));

        let missing = Manifest { recipe: "absente.rhai".into(), ..Manifest::default() };
        assert!(
            locate_script(root.path(), &missing).unwrap_err().to_string().contains("introuvable")
        );
    }
}
