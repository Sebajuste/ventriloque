//! Les archives `.archive` de REDengine, en lecture seule.
//!
//! UNE ARCHIVE CYBERPUNK NE PORTE PAS DE NOMS. Elle ne range que le FNV1a64 du path depot en
//! minuscules ; le path lui-meme n'est nulle part dans le jeu. WolvenKit distribue un
//! dictionnaire de 1,7 million de chemins connus pour combler ce trou, et il pese 135 Mo une
//! fois decompresse.
//!
//! CE LECTEUR NE LE PORTE PAS, et c'est un choix, pas un manque. Une recette n'a pas besoin de
//! parcourir les chemins : elle nomme les repliques qu'elle veut, et un hash de 64 bits est
//! un name parfaitement utilisable. Les entries sont donc nommees par leurs seize chiffres
//! hexadecimaux, `lister("*")` les rend telles quelles, et le dictionnaire reste dehors. C'est
//! ce que fait deja le mode « recette » de l'extracteur d'`ai_npc-holo`, dont les hachages ont
//! ete verifies contre cette implementation.
//!
//! CE QUI A ETE MESURE, le 2026-09-06, sur `D:\Jeux\Cyberpunk 2077` (2.3 + Phantom Liberty) :
//! `lang_fr_voice.archive` porte 90 755 entries, celle d'`ep1` 28 041. Les 68 repliques
//! francaises de la recette s'y retrouvent toutes, chacune en **un seul segment non compresse**.
//! Aucune decompression Oodle n'est donc necessaire pour du doublage ; les segments compresses
//! existent ailleurs dans le jeu et ce lecteur les refuse plutot que de faire semblant.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use super::Entry;


struct Archive {
    file: File,
    /// hash -> (offset dans le file, size compressed, size real)
    entries: HashMap<u64, (u64, u32, u32)>,
}

pub struct RedengineStorage {
    archives: Vec<Archive>,
    entries: Vec<Entry>,
}

fn u32_at(d: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(d[p..p + 4].try_into().unwrap())
}
fn u64_at(d: &[u8], p: usize) -> u64 {
    u64::from_le_bytes(d[p..p + 8].try_into().unwrap())
}

/// L'en-tete d'une archive fait 40 octets, tasses : `IndexPosition` n'est pas alignee et
/// `DebugPosition` encore moins. Le read champ par champ plutot que de plaquer une structure.
const HEADER: usize = 0x28;
/// Une entry de la table des fichiers : hash, horodatage, comptes, et un SHA1.
const FILE_ENTRY: usize = 56;
/// Un segment : offset sur 64 bits, size compressed et size real sur 32.
const SEGMENT: usize = 16;

impl Archive {
    fn open(path: &Path) -> Result<Self, String> {
        // `FileShare.ReadWrite` du cote Windows : le jeu garde ses archives ouvertes, et poser
        // un verrou exclusif ici empecherait une fabrication pendant qu'il tourne. L'ouverture
        // en lecture seule de Rust ne pose deja aucun verrou.
        let mut file =
            File::open(path).map_err(|e| format!("{} : {e}", path.display()))?;

        let mut header = [0u8; HEADER];
        file
            .read_exact(&mut header)
            .map_err(|e| format!("{} : en-tete illisible ({e})", path.display()))?;
        if &header[0..4] != b"RDAR" {
            return Err(format!("{} : ce n'est pas une archive RDAR", path.display()));
        }
        let version = u32_at(&header, 4);
        if version != 12 {
            return Err(format!(
                "{} : archive de version {version}, seule la 12 est traitee",
                path.display()
            ));
        }

        let offset = u64_at(&header, 8);
        let size = u32_at(&header, 16) as usize;
        file
            .seek(SeekFrom::Start(offset))
            .map_err(|e| format!("{} : index introuvable ({e})", path.display()))?;
        let mut index = vec![0u8; size];
        file
            .read_exact(&mut index)
            .map_err(|e| format!("{} : index tronque ({e})", path.display()))?;

        // LES COMPTES SE LISENT A L'OCTET PRES. `fileCount` est a l'offset 16 et `segmentCount`
        // a 20 ; se tromper d'un champ n'echoue pas a l'ouverture — l'archive se lit, et l'index
        // de segment deborde quelques milliers de fichiers plus loin.
        if index.len() < 28 {
            return Err(format!("{} : index trop court", path.display()));
        }
        let file_count = u32_at(&index, 16) as usize;
        let segment_count = u32_at(&index, 20) as usize;
        let link_count = u32_at(&index, 24) as usize;

        let segments_start = 28 + file_count * FILE_ENTRY;
        let end = segments_start + segment_count * SEGMENT + link_count * 8;
        if end != index.len() {
            return Err(format!(
                "{} : index de {} octets, {end} attendus d'apres ses comptes",
                path.display(),
                index.len()
            ));
        }

        let segment = |index_of: usize| -> (u64, u32, u32) {
            let p = segments_start + index_of * SEGMENT;
            (u64_at(&index, p), u32_at(&index, p + 8), u32_at(&index, p + 12))
        };

        let mut entries = HashMap::with_capacity(file_count);
        for i in 0..file_count {
            let p = 28 + i * FILE_ENTRY;
            let hash = u64_at(&index, p);
            let first = u32_at(&index, p + 20) as usize;
            let last = u32_at(&index, p + 24) as usize;
            // Le doublage tient dans un segment. Un file decoupe est une ressource du moteur,
            // dont ce lecteur n'a que faire ; l'ignorer vaut mieux que de rendre un morceau.
            if last != first + 1 || last > segment_count {
                continue;
            }
            entries.insert(hash, segment(first));
        }

        Ok(Archive { file, entries })
    }
}

/// Le FNV1a64 d'un path depot en minuscules : la seule chose qu'une archive connaisse d'un
/// file. Sert aux tests et a qui voudrait verifier un hash a la main.
pub fn path_hash(path: &str) -> u64 {
    let mut value: u64 = 0xCBF2_9CE4_8422_2325;
    for byte in path.to_lowercase().bytes() {
        value ^= byte as u64;
        value = value.wrapping_mul(0x0000_0100_0000_01B3);
    }
    value
}

impl RedengineStorage {
    /// Ouvre les archives de doublage d'une installation Cyberpunk 2077.
    ///
    /// Seules celles-la : le reste du jeu — trente giga-octets de maillages et de textures — n'a
    /// rien a faire ici, et les open couterait le recensement de plusieurs millions d'entries
    /// pour n'en read aucune.
    pub fn open(root: &Path) -> Result<Self, String> {
        let mut archives = Vec::new();
        let mut complaints = Vec::new();
        for sub in ["content", "ep1"] {
            let folder = root.join("archive").join("pc").join(sub);
            let Ok(entries) = std::fs::read_dir(&folder) else {
                continue;
            };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if !name.starts_with("lang_") || !name.ends_with("_voice.archive") {
                    continue;
                }
                match Archive::open(&entry.path()) {
                    Ok(a) => archives.push(a),
                    Err(e) => complaints.push(e),
                }
            }
        }

        if archives.is_empty() {
            let detail = if complaints.is_empty() {
                "aucune `lang_*_voice.archive` sub `archive\\pc\\`".to_string()
            } else {
                complaints.join(" ; ")
            };
            return Err(format!("{} : {detail}", root.display()));
        }

        Ok(RedengineStorage { archives, entries: Vec::new() })
    }

    /// Le name de code du jeu, pour qu'une recette puisse verifier qu'elle est au bon endroit.
    pub fn product(&self) -> String {
        "cp77".to_string()
    }

    /// L'index de tout ce que le stockage nomme. Les archives de doublage ne portant pas de
    /// chemins, un name est un hash en seize chiffres hexadecimaux.
    pub fn entries(&mut self) -> &[Entry] {
        if self.entries.is_empty() {
            for archive in &self.archives {
                for (hash, (_, _, size)) in &archive.entries {
                    self.entries.push(Entry {
                        name: format!("{hash:016x}"),
                        size: *size as u64,
                    });
                }
            }
            self.entries.sort_by(|a, b| a.name.cmp(&b.name));
        }
        &self.entries
    }

    /// Lit une entry en entier, par son hash ecrit en hexadecimal.
    pub fn read(&self, name: &str) -> Result<Vec<u8>, String> {
        let clean = name.trim().trim_start_matches("0x");
        let hash = u64::from_str_radix(clean, 16)
            .map_err(|_| format!("« {name} » n'est pas un hash hexadecimal de 64 bits"))?;

        for archive in &self.archives {
            let Some((offset, compressed, real)) = archive.entries.get(&hash) else {
                continue;
            };
            if compressed != real {
                return Err(format!(
                    "{hash:016x} : segment compresse ({compressed} -> {real} octets), \
                     ce lecteur ne traite pas Oodle"
                ));
            }
            let mut data = vec![0u8; *real as usize];
            let mut file = &archive.file;
            file
                .seek(SeekFrom::Start(*offset))
                .map_err(|e| format!("{hash:016x} : {e}"))?;
            file
                .read_exact(&mut data)
                .map_err(|e| format!("{hash:016x} : {e}"))?;
            return Ok(data);
        }
        Err(format!("{hash:016x} : absent des archives de doublage"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Le hash releve dans l'archive francaise le 2026-09-06, pour une replique de Judy. Si
    // cette value change, c'est la regle de hash qui a bouge, pas le jeu.
    #[test]
    fn le_hachage_est_le_fnv1a64_du_chemin_en_minuscules() {
        assert_eq!(
            path_hash("base\\localization\\fr-fr\\vo\\judy_q105_f_17a687de8d2b6000.wem"),
            0x0192_91c8_e47b_9861
        );
        // La casse ne compte pas : le jeu range tout en minuscules.
        assert_eq!(
            path_hash("BASE\\Localization\\FR-FR\\VO\\Judy_Q105_F_17a687de8d2b6000.WEM"),
            0x0192_91c8_e47b_9861
        );
    }

    #[test]
    fn le_hachage_vide_est_la_valeur_de_depart() {
        assert_eq!(path_hash(""), 0xCBF2_9CE4_8422_2325);
    }
}
