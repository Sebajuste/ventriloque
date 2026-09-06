// Le moteur, embarque DANS l'executable.
//
// POURQUOI. Ventriloque doit tenir dans un fichier qu'on copie sur une cle et qu'on lance : pas
// d'installateur, pas de dossier `binaries` a garder a cote, rien a reinstaller sur la machine
// ou se joue la partie. Les deux pieces necessaires pesent quinze megaoctets a elles deux, ce
// qui est un prix honnete pour cette propriete.
//
// Elles sont posees a cote de l'executable au premier lancement, parce qu'un processus fils ne
// se lance pas depuis de la memoire : il lui faut un chemin. Ecrites une fois, puis reconnues a
// leur taille -- comparer quinze megaoctets a chaque demarrage couterait plus cher que le gain.
//
// CE QUI N'EST PAS ICI. Les modeles, un demi-gigaoctet, qui viennent d'un depot sur liste
// d'autorisation et s'installent en paquet. Voir `packs.rs`.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

const MOTEUR: &[u8] = include_bytes!("../binaries/pocket-tts-x86_64-pc-windows-msvc.exe");
const RUNTIME: &[u8] = include_bytes!("../binaries/onnxruntime.dll");

// Ecrit seulement si le fichier manque ou n'a pas la bonne taille. La taille suffit comme
// controle : ces deux fichiers ne changent qu'avec une version de Ventriloque, et une version
// differente les ecrit toutes les deux.
fn poser(cible: &Path, contenu: &[u8]) -> Result<()> {
    if cible.metadata().is_ok_and(|m| m.len() == contenu.len() as u64) {
        return Ok(());
    }
    std::fs::write(cible, contenu).with_context(|| format!("ecriture de {}", cible.display()))
}

// Rend le chemin du moteur, pose a cote de l'executable.
pub fn deployer(racine: &Path) -> Result<PathBuf> {
    let dossier = racine.join("moteur");
    std::fs::create_dir_all(&dossier).context("creation du dossier du moteur")?;

    let exe = dossier.join("pocket-tts.exe");
    poser(&exe, MOTEUR)?;
    // La bibliotheque ONNX doit etre A COTE de l'executable qui la charge, pas ailleurs : c'est
    // le dossier de l'image qui est cherche en premier.
    poser(&dossier.join("onnxruntime.dll"), RUNTIME)?;
    Ok(exe)
}
