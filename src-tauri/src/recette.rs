// La fabrication : faire tourner le script d'un paquet sur une copie installee d'un jeu.
//
// LE SCRIPT NE TOURNE PAS ICI. Il tourne dans `fabriquer.exe`, un processus separe, lie a notre
// vie par un job object comme le moteur de parole. Deux codes qui ne viennent pas de
// l'application -- le C++ de CascLib et le script du paquet -- restent ainsi dehors : ils ne
// peuvent pas lire notre memoire, et se tuent d'un geste.
//
// CE QUI SORT EST TRAITE COMME CE QUI SORT D'UN ZIP. Le processus fils ecrit dans un dossier de
// travail jetable ; rien n'entre dans `voix\` ou `pnj\` sans passer par le meme controle que
// `packs::accueillir`. Un script qui rendrait `..\..\Windows\System32\x.wav` ne pose rien.
//
// LE JEU N'EST JAMAIS TOUCHE. Le fils l'ouvre en lecture, et n'y ecrit pas.

use anyhow::{Context, Result, anyhow};
use std::path::Path;
use std::process::{Command, Stdio};

use crate::packs::{Chantier, Manifeste};

/// Ce qu'une fabrication a produit, tel qu'on le raconte dans la fenetre.
pub struct Rapport {
    pub journal: Vec<String>,
    pub poses: Vec<String>,
}

/// Un nom de fichier rendu par le fils, accepte seulement s'il est ordinaire : pas de dossier,
/// pas de `..`, rien d'invisible. C'est la meme severite que pour une entree de zip.
fn nom_sur(brut: &str) -> Option<String> {
    let acceptable = !brut.is_empty()
        && brut.len() <= 80
        && !brut.starts_with('.')
        && !brut.contains("..")
        && brut
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');
    acceptable.then(|| brut.to_string())
}

/// Deplace ce que le fils a produit vers les dossiers de l'application.
///
/// `extension` dit ce qui a le droit d'atterrir dans quel dossier : une recette ne peut pas
/// glisser un `.exe` dans `voix\` en le nommant bien.
fn moissonner(travail: &Path, racine: &Path, sous: &str, extension: &str) -> Result<Vec<String>> {
    let source = travail.join(sous);
    let cible = racine.join(sous);
    std::fs::create_dir_all(&cible).ok();

    let mut poses = Vec::new();
    let Ok(entrees) = std::fs::read_dir(&source) else {
        return Ok(poses);
    };
    for entree in entrees.flatten() {
        if !entree.path().is_file() {
            continue;
        }
        let nom = entree.file_name().to_string_lossy().into_owned();
        let Some(sur) = nom_sur(&nom) else { continue };
        if !sur.to_lowercase().ends_with(extension) {
            continue;
        }
        std::fs::copy(entree.path(), cible.join(&sur))
            .with_context(|| format!("pose de {sous}/{sur}"))?;
        poses.push(format!("{sous}/{sur}"));
    }
    poses.sort();
    Ok(poses)
}

/// Lance la fabrication et moissonne. Bloquant : compter deux a quatre minutes sur StarCraft II,
/// dont une pour recenser les 780 000 entrees du stockage.
pub fn fabriquer(
    racine: &Path,
    fabricant: &Path,
    manifeste: &mut Manifeste,
    jeu: &Path,
    chantier: &Chantier,
) -> Result<Rapport> {
    if manifeste.recette.is_empty() {
        return Err(anyhow!("« {} » ne porte pas de recette", manifeste.nom));
    }
    let Some(recette) = nom_sur(&manifeste.recette) else {
        return Err(anyhow!("nom de recette refuse : {}", manifeste.recette));
    };
    let script = racine.join("recettes").join(&recette);
    if !script.is_file() {
        return Err(anyhow!("recette introuvable : {}", script.display()));
    }
    if !fabricant.is_file() {
        return Err(anyhow!(
            "le fabricant n'est pas depose : {}",
            fabricant.display()
        ));
    }
    if !jeu.is_dir() {
        return Err(anyhow!("ce n'est pas un dossier : {}", jeu.display()));
    }

    // Un dossier de travail jetable, hors des dossiers de donnees : ce que le fils ecrit n'est
    // pas encore du bien de l'application.
    let travail = racine.join(".fabrique").join(manifeste.identifiant());
    let _ = std::fs::remove_dir_all(&travail);
    std::fs::create_dir_all(&travail).context("creation du dossier de fabrication")?;

    let mut commande = Command::new(fabricant);
    commande
        .arg("--script").arg(&script)
        .arg("--jeu").arg(jeu)
        .arg("--sortie").arg(&travail)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // Sans cela, une console noire s'ouvre a cote de la fenetre.
        commande.creation_flags(0x0800_0000);
    }

    let mut enfant = commande.spawn().context("lancement du fabricant")?;
    #[cfg(windows)]
    crate::engine::lier_a_notre_vie(&enfant);

    // LE JOURNAL EST LU AU FIL DE L'EAU, pas ramasse a la fin. Une fabrication dure deux a quatre
    // minutes ; attendre la derniere ligne pour montrer la premiere laisserait la fenetre muette
    // tout ce temps, sans moyen de distinguer un travail qui avance d'un processus mort.
    let mut journal: Vec<String> = Vec::new();
    if let Some(sortie) = enfant.stdout.take() {
        use std::io::BufRead;
        for ligne in std::io::BufReader::new(sortie).lines().map_while(Result::ok) {
            let ligne = ligne.trim_end().to_string();
            if ligne.is_empty() {
                continue;
            }
            chantier.dire(&ligne);
            chantier.poser(&ligne);
            journal.push(ligne);
        }
    }

    // `stdout` a deja ete pris : il ne reste que `stderr` a ramasser, et l'attente du fils.
    let issue = enfant.wait_with_output().context("attente du fabricant")?;

    if !issue.status.success() {
        let plainte = String::from_utf8_lossy(&issue.stderr).trim().to_string();
        let _ = std::fs::remove_dir_all(&travail);
        return Err(anyhow!(if plainte.is_empty() {
            "la fabrication a echoue".to_string()
        } else {
            plainte
        }));
    }

    chantier.dire("moisson…");
    let mut poses = moissonner(&travail, racine, "voix", ".wav")?;
    poses.extend(moissonner(&travail, racine, "pnj", ".json")?);
    let _ = std::fs::remove_dir_all(&travail);

    if poses.is_empty() {
        return Err(anyhow!("la fabrication n'a rien pose d'utilisable"));
    }

    // Le manifeste garde ce qui a ete pose, pour qu'une desinstallation sache quoi retirer, et
    // la date pour que l'onglet cesse de dire « a fabriquer ».
    for pose in &poses {
        if !manifeste.fichiers.contains(pose) {
            manifeste.fichiers.push(pose.clone());
        }
    }
    manifeste.fabrique_le = crate::packs::horodatage();
    crate::packs::inscrire(racine, manifeste)?;

    Ok(Rapport { journal, poses })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn les_noms_dangereux_sont_refuses() {
        assert!(nom_sur("sc2_kerrigan.wav").is_some());
        assert!(nom_sur("../dehors.wav").is_none());
        assert!(nom_sur("voix/ailleurs.wav").is_none());
        assert!(nom_sur(".cache").is_none());
        assert!(nom_sur("").is_none());
    }

    // Ce qui n'a pas la bonne extension ne passe pas, meme bien nomme.
    #[test]
    fn la_moisson_trie_par_extension() {
        let base = std::env::temp_dir().join(format!(
            "ventriloque-moisson-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().subsec_nanos()
        ));
        let travail = base.join("travail");
        let racine = base.join("racine");
        std::fs::create_dir_all(travail.join("voix")).unwrap();
        std::fs::write(travail.join("voix/bonne.wav"), b"RIFF").unwrap();
        std::fs::write(travail.join("voix/mauvaise.exe"), b"MZ").unwrap();

        let poses = moissonner(&travail, &racine, "voix", ".wav").unwrap();
        assert_eq!(poses, vec!["voix/bonne.wav".to_string()]);
        assert!(racine.join("voix/bonne.wav").is_file());
        assert!(!racine.join("voix/mauvaise.exe").exists());

        let _ = std::fs::remove_dir_all(&base);
    }
}
