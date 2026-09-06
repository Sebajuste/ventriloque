//! Les archives `.archive` de REDengine, en lecture seule.
//!
//! UNE ARCHIVE CYBERPUNK NE PORTE PAS DE NOMS. Elle ne range que le FNV1a64 du chemin depot en
//! minuscules ; le chemin lui-meme n'est nulle part dans le jeu. WolvenKit distribue un
//! dictionnaire de 1,7 million de chemins connus pour combler ce trou, et il pese 135 Mo une
//! fois decompresse.
//!
//! CE LECTEUR NE LE PORTE PAS, et c'est un choix, pas un manque. Une recette n'a pas besoin de
//! parcourir les chemins : elle nomme les repliques qu'elle veut, et un hachage de 64 bits est
//! un nom parfaitement utilisable. Les entrees sont donc nommees par leurs seize chiffres
//! hexadecimaux, `lister("*")` les rend telles quelles, et le dictionnaire reste dehors. C'est
//! ce que fait deja le mode « recette » de l'extracteur d'`ai_npc-holo`, dont les hachages ont
//! ete verifies contre cette implementation.
//!
//! CE QUI A ETE MESURE, le 2026-09-06, sur `D:\Jeux\Cyberpunk 2077` (2.3 + Phantom Liberty) :
//! `lang_fr_voice.archive` porte 90 755 entrees, celle d'`ep1` 28 041. Les 68 repliques
//! francaises de la recette s'y retrouvent toutes, chacune en **un seul segment non compresse**.
//! Aucune decompression Oodle n'est donc necessaire pour du doublage ; les segments compresses
//! existent ailleurs dans le jeu et ce lecteur les refuse plutot que de faire semblant.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::stockage::Entree;


struct Archive {
    fichier: File,
    /// hachage -> (position dans le fichier, taille compressee, taille reelle)
    entrees: HashMap<u64, (u64, u32, u32)>,
}

pub struct Stockage {
    archives: Vec<Archive>,
    entrees: Vec<Entree>,
}

fn u32_de(d: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(d[p..p + 4].try_into().unwrap())
}
fn u64_de(d: &[u8], p: usize) -> u64 {
    u64::from_le_bytes(d[p..p + 8].try_into().unwrap())
}

/// L'en-tete d'une archive fait 40 octets, tasses : `IndexPosition` n'est pas alignee et
/// `DebugPosition` encore moins. Le lire champ par champ plutot que de plaquer une structure.
const ENTETE: usize = 0x28;
/// Une entree de la table des fichiers : hachage, horodatage, comptes, et un SHA1.
const FICHIER: usize = 56;
/// Un segment : position sur 64 bits, taille compressee et taille reelle sur 32.
const SEGMENT: usize = 16;

impl Archive {
    fn ouvrir(chemin: &Path) -> Result<Self, String> {
        // `FileShare.ReadWrite` du cote Windows : le jeu garde ses archives ouvertes, et poser
        // un verrou exclusif ici empecherait une fabrication pendant qu'il tourne. L'ouverture
        // en lecture seule de Rust ne pose deja aucun verrou.
        let mut fichier =
            File::open(chemin).map_err(|e| format!("{} : {e}", chemin.display()))?;

        let mut entete = [0u8; ENTETE];
        fichier
            .read_exact(&mut entete)
            .map_err(|e| format!("{} : en-tete illisible ({e})", chemin.display()))?;
        if &entete[0..4] != b"RDAR" {
            return Err(format!("{} : ce n'est pas une archive RDAR", chemin.display()));
        }
        let version = u32_de(&entete, 4);
        if version != 12 {
            return Err(format!(
                "{} : archive de version {version}, seule la 12 est traitee",
                chemin.display()
            ));
        }

        let position = u64_de(&entete, 8);
        let taille = u32_de(&entete, 16) as usize;
        fichier
            .seek(SeekFrom::Start(position))
            .map_err(|e| format!("{} : index introuvable ({e})", chemin.display()))?;
        let mut index = vec![0u8; taille];
        fichier
            .read_exact(&mut index)
            .map_err(|e| format!("{} : index tronque ({e})", chemin.display()))?;

        // LES COMPTES SE LISENT A L'OCTET PRES. `fileCount` est a l'offset 16 et `segmentCount`
        // a 20 ; se tromper d'un champ n'echoue pas a l'ouverture — l'archive se lit, et l'index
        // de segment deborde quelques milliers de fichiers plus loin.
        if index.len() < 28 {
            return Err(format!("{} : index trop court", chemin.display()));
        }
        let compte_fichiers = u32_de(&index, 16) as usize;
        let compte_segments = u32_de(&index, 20) as usize;
        let compte_liens = u32_de(&index, 24) as usize;

        let debut_segments = 28 + compte_fichiers * FICHIER;
        let fin = debut_segments + compte_segments * SEGMENT + compte_liens * 8;
        if fin != index.len() {
            return Err(format!(
                "{} : index de {} octets, {fin} attendus d'apres ses comptes",
                chemin.display(),
                index.len()
            ));
        }

        let segment = |rang: usize| -> (u64, u32, u32) {
            let p = debut_segments + rang * SEGMENT;
            (u64_de(&index, p), u32_de(&index, p + 8), u32_de(&index, p + 12))
        };

        let mut entrees = HashMap::with_capacity(compte_fichiers);
        for i in 0..compte_fichiers {
            let p = 28 + i * FICHIER;
            let hachage = u64_de(&index, p);
            let premier = u32_de(&index, p + 20) as usize;
            let dernier = u32_de(&index, p + 24) as usize;
            // Le doublage tient dans un segment. Un fichier decoupe est une ressource du moteur,
            // dont ce lecteur n'a que faire ; l'ignorer vaut mieux que de rendre un morceau.
            if dernier != premier + 1 || dernier > compte_segments {
                continue;
            }
            entrees.insert(hachage, segment(premier));
        }

        Ok(Archive { fichier, entrees })
    }
}

/// Le FNV1a64 d'un chemin depot en minuscules : la seule chose qu'une archive connaisse d'un
/// fichier. Sert aux tests et a qui voudrait verifier un hachage a la main.
pub fn hachage(chemin: &str) -> u64 {
    let mut valeur: u64 = 0xCBF2_9CE4_8422_2325;
    for octet in chemin.to_lowercase().bytes() {
        valeur ^= octet as u64;
        valeur = valeur.wrapping_mul(0x0000_0100_0000_01B3);
    }
    valeur
}

impl Stockage {
    /// Ouvre les archives de doublage d'une installation Cyberpunk 2077.
    ///
    /// Seules celles-la : le reste du jeu — trente giga-octets de maillages et de textures — n'a
    /// rien a faire ici, et les ouvrir couterait le recensement de plusieurs millions d'entrees
    /// pour n'en lire aucune.
    pub fn ouvrir(racine: &Path) -> Result<Self, String> {
        let mut archives = Vec::new();
        let mut plaintes = Vec::new();
        for sous in ["content", "ep1"] {
            let dossier = racine.join("archive").join("pc").join(sous);
            let Ok(entrees) = std::fs::read_dir(&dossier) else {
                continue;
            };
            for entree in entrees.flatten() {
                let nom = entree.file_name().to_string_lossy().to_lowercase();
                if !nom.starts_with("lang_") || !nom.ends_with("_voice.archive") {
                    continue;
                }
                match Archive::ouvrir(&entree.path()) {
                    Ok(a) => archives.push(a),
                    Err(e) => plaintes.push(e),
                }
            }
        }

        if archives.is_empty() {
            let detail = if plaintes.is_empty() {
                "aucune `lang_*_voice.archive` sous `archive\\pc\\`".to_string()
            } else {
                plaintes.join(" ; ")
            };
            return Err(format!("{} : {detail}", racine.display()));
        }

        Ok(Stockage { archives, entrees: Vec::new() })
    }

    /// Le nom de code du jeu, pour qu'une recette puisse verifier qu'elle est au bon endroit.
    pub fn produit(&self) -> String {
        "cp77".to_string()
    }

    /// L'index de tout ce que le stockage nomme. Les archives de doublage ne portant pas de
    /// chemins, un nom est un hachage en seize chiffres hexadecimaux.
    pub fn entrees(&mut self) -> &[Entree] {
        if self.entrees.is_empty() {
            for archive in &self.archives {
                for (hachage, (_, _, taille)) in &archive.entrees {
                    self.entrees.push(Entree {
                        nom: format!("{hachage:016x}"),
                        taille: *taille as u64,
                    });
                }
            }
            self.entrees.sort_by(|a, b| a.nom.cmp(&b.nom));
        }
        &self.entrees
    }

    /// Lit une entree en entier, par son hachage ecrit en hexadecimal.
    pub fn lire(&self, nom: &str) -> Result<Vec<u8>, String> {
        let propre = nom.trim().trim_start_matches("0x");
        let hachage = u64::from_str_radix(propre, 16)
            .map_err(|_| format!("« {nom} » n'est pas un hachage hexadecimal de 64 bits"))?;

        for archive in &self.archives {
            let Some((position, compressee, reelle)) = archive.entrees.get(&hachage) else {
                continue;
            };
            if compressee != reelle {
                return Err(format!(
                    "{hachage:016x} : segment compresse ({compressee} -> {reelle} octets), \
                     ce lecteur ne traite pas Oodle"
                ));
            }
            let mut donnees = vec![0u8; *reelle as usize];
            let mut fichier = &archive.fichier;
            fichier
                .seek(SeekFrom::Start(*position))
                .map_err(|e| format!("{hachage:016x} : {e}"))?;
            fichier
                .read_exact(&mut donnees)
                .map_err(|e| format!("{hachage:016x} : {e}"))?;
            return Ok(donnees);
        }
        Err(format!("{hachage:016x} : absent des archives de doublage"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Le hachage releve dans l'archive francaise le 2026-09-06, pour une replique de Judy. Si
    // cette valeur change, c'est la regle de hachage qui a bouge, pas le jeu.
    #[test]
    fn le_hachage_est_le_fnv1a64_du_chemin_en_minuscules() {
        assert_eq!(
            hachage("base\\localization\\fr-fr\\vo\\judy_q105_f_17a687de8d2b6000.wem"),
            0x0192_91c8_e47b_9861
        );
        // La casse ne compte pas : le jeu range tout en minuscules.
        assert_eq!(
            hachage("BASE\\Localization\\FR-FR\\VO\\Judy_Q105_F_17a687de8d2b6000.WEM"),
            0x0192_91c8_e47b_9861
        );
    }

    #[test]
    fn le_hachage_vide_est_la_valeur_de_depart() {
        assert_eq!(hachage(""), 0xCBF2_9CE4_8422_2325);
    }
}
