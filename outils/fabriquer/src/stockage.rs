//! Le stockage d'un jeu, quel que soit son conteneur.
//!
//! LE LECTEUR EST CE QUI DECIDE, PAS LA RECETTE. Un script nomme des entrees et des personnages ;
//! il ne sait pas, et n'a pas a savoir, si derriere il y a du CASC de Blizzard ou du RDAR de
//! REDengine. Les deux rendent la meme chose : un nom, une taille, des octets.
//!
//! Ce que « nom » veut dire change en revanche d'un conteneur a l'autre, et c'est la seule chose
//! qu'une recette doit connaitre de son jeu :
//!
//! | jeu | reconnu par | `produit()` | un nom d'entree |
//! |---|---|---|---|
//! | StarCraft II | `.build.info` | `s2` | un chemin, `…/vo/zbriefing_kerrigan_007.ogg` |
//! | Cyberpunk 2077 | `archive\pc\content\` | `cp77` | un hachage, `019291c8e47b9861` |
//!
//! Cyberpunk ne range pas les chemins de ses fichiers : voir `rdar.rs`.

use std::path::Path;

/// Une entree nommee du stockage.
#[derive(Clone, Debug)]
pub struct Entree {
    pub nom: String,
    pub taille: u64,
}

pub enum Stockage {
    Blizzard(crate::casc::Stockage),
    Redengine(crate::rdar::Stockage),
}

/// Ouvre l'installation posee la, en reconnaissant le jeu a ce qu'il laisse voir de lui-meme.
///
/// Aucune des deux marques n'est un choix de notre part : `.build.info` est le fichier que
/// l'installateur de Blizzard depose a la racine, `archive\pc\content` la disposition de
/// REDengine. On ne demande donc rien au joueur qu'on ne puisse constater.
pub fn ouvrir(racine: &Path) -> Result<Stockage, String> {
    if racine.join(".build.info").exists() {
        return crate::casc::Stockage::ouvrir(racine).map(Stockage::Blizzard);
    }
    if racine.join("archive").join("pc").join("content").is_dir() {
        return crate::rdar::Stockage::ouvrir(racine).map(Stockage::Redengine);
    }
    Err(format!(
        "{} ne porte ni `.build.info` (installation Blizzard) ni `archive\\pc\\content` \
         (REDengine) : quel jeu est-ce ?",
        racine.display()
    ))
}

impl Stockage {
    pub fn produit(&self) -> String {
        match self {
            Stockage::Blizzard(s) => s.produit(),
            Stockage::Redengine(s) => s.produit(),
        }
    }

    pub fn entrees(&mut self) -> &[Entree] {
        match self {
            Stockage::Blizzard(s) => s.entrees(),
            Stockage::Redengine(s) => s.entrees(),
        }
    }

    pub fn lire(&self, nom: &str) -> Result<Vec<u8>, String> {
        match self {
            Stockage::Blizzard(s) => s.lire(nom),
            Stockage::Redengine(s) => s.lire(nom),
        }
    }
}
