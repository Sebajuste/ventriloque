//! Le stockage d'un jeu, quel que soit son conteneur.
//!
//! LE LECTEUR EST CE QUI DECIDE, PAS LA RECETTE. Un script nomme des entrees et des personnages ;
//! il ne sait pas, et n'a pas a savoir, si derriere il y a du CASC de Blizzard ou du RDAR de
//! REDengine. Les deux rendent la meme chose : un nom, une taille, des octets.
//!
//! Ce que « nom » veut dire change en revanche d'un conteneur a l'autre, et c'est la seule chose
//! qu'une recette doit connaitre de son jeu :
//!
//! | jeu | reconnu par | `product()` | un nom d'entree |
//! |---|---|---|---|
//! | StarCraft II | `.build.info` | `s2` | un chemin, `…/vo/zbriefing_kerrigan_007.ogg` |
//! | Cyberpunk 2077 | `archive\pc\content\` | `cp77` | un hachage, `019291c8e47b9861` |
//!
//! Cyberpunk ne range pas les chemins de ses fichiers : voir `rdar`.

mod casc;
mod rdar;
pub mod wwise;

use std::path::Path;

pub use rdar::path_hash;

/// Une entree nommee du stockage.
#[derive(Clone, Debug)]
pub struct Entry {
    pub name: String,
    pub size: u64,
}

pub enum Storage {
    Blizzard(casc::CascStorage),
    Redengine(rdar::RedengineStorage),
}

/// Ouvre l'installation posee la, en reconnaissant le jeu a ce qu'il laisse voir de lui-meme.
///
/// Aucune des deux marques n'est un choix de notre part : `.build.info` est le fichier que
/// l'installateur de Blizzard depose a la racine, `archive\pc\content` la disposition de
/// REDengine. On ne demande donc rien au joueur qu'on ne puisse constater.
pub fn open(root: &Path) -> Result<Storage, String> {
    if root.join(".build.info").exists() {
        return casc::CascStorage::open(root).map(Storage::Blizzard);
    }
    if root.join("archive").join("pc").join("content").is_dir() {
        return rdar::RedengineStorage::open(root).map(Storage::Redengine);
    }
    Err(format!(
        "{} ne porte ni `.build.info` (installation Blizzard) ni `archive\\pc\\content` \
         (REDengine) : quel jeu est-ce ?",
        root.display()
    ))
}

impl Storage {
    /// Le nom de code du jeu, pour qu'une recette puisse verifier qu'elle est au bon endroit.
    pub fn product(&self) -> String {
        match self {
            Storage::Blizzard(s) => s.product(),
            Storage::Redengine(s) => s.product(),
        }
    }

    /// L'index de tout ce que le stockage nomme. Etabli au premier appel, garde ensuite : le
    /// parcours coute une minute sur StarCraft II et ses 780 000 entrees.
    pub fn entries(&mut self) -> &[Entry] {
        match self {
            Storage::Blizzard(s) => s.entries(),
            Storage::Redengine(s) => s.entries(),
        }
    }

    /// Lit une entree en entier.
    pub fn read(&self, name: &str) -> Result<Vec<u8>, String> {
        match self {
            Storage::Blizzard(s) => s.read(name),
            Storage::Redengine(s) => s.read(name),
        }
    }
}
