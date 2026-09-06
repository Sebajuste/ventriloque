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
// d'autorisation et s'installent en paquet. Voir `packs`.

use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

use crate::paths;

const ENGINE_EXE: &[u8] = include_bytes!("../binaries/pocket-tts-x86_64-pc-windows-msvc.exe");
const ONNX_RUNTIME: &[u8] = include_bytes!("../binaries/onnxruntime.dll");
// Le fabricant : il porte CascLib et le moteur de script, et fait tourner les recettes des
// paquets hors de ce processus. Voir `packs::build`.
const BUILDER_EXE: &[u8] = include_bytes!("../binaries/fabriquer.exe");

const ENGINE_NAME: &str = "pocket-tts.exe";
const BUILDER_NAME: &str = "fabriquer.exe";
const RUNTIME_NAME: &str = "onnxruntime.dll";

// Ecrit seulement si le fichier manque ou n'a pas la bonne taille. La taille suffit comme
// controle : ces fichiers ne changent qu'avec une version de Ventriloque, et une version
// differente les ecrit toutes.
fn write_if_stale(target: &Path, content: &[u8]) -> Result<()> {
    if target.metadata().is_ok_and(|m| m.len() == content.len() as u64) {
        return Ok(());
    }
    std::fs::write(target, content).with_context(|| format!("ecriture de {}", target.display()))
}

/// Le fabricant, pose a cote du moteur. Nomme ici pour que `packs::build` n'ait pas a redire ou
/// il vit.
pub fn builder_path(root: &Path) -> PathBuf {
    root.join(paths::ENGINE).join(BUILDER_NAME)
}

/// Rend le chemin du moteur, pose a cote de l'executable.
pub fn deploy(root: &Path) -> Result<PathBuf> {
    let folder = root.join(paths::ENGINE);
    std::fs::create_dir_all(&folder).context("creation du dossier du moteur")?;

    let exe = folder.join(ENGINE_NAME);
    write_if_stale(&exe, ENGINE_EXE)?;
    write_if_stale(&folder.join(BUILDER_NAME), BUILDER_EXE)?;
    // La bibliotheque ONNX doit etre A COTE de l'executable qui la charge, pas ailleurs : c'est
    // le dossier de l'image qui est cherche en premier.
    write_if_stale(&folder.join(RUNTIME_NAME), ONNX_RUNTIME)?;
    Ok(exe)
}
