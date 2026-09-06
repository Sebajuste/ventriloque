// Trouver une installation de jeu sans rien demander a l'utilisateur.
//
// POURQUOI PAS UNE LISTE DE CHEMINS HABITUELS. Sur la machine de developpement, StarCraft II est
// dans `D:\Jeux\StarCraft II` et Diablo IV a cote : aucune supposition sur `C:\Program Files`
// n'aurait rien trouve. Ce qui marche, c'est le registre de desinstallation, qui porte le chemin
// exact quel qu'il soit. Les racines usuelles ne servent que de filet.
//
// COMMENT ON RECONNAIT UN JEU. `.build.info`, a la racine d'une installation Blizzard, est un
// fichier texte a colonnes. Sa colonne `CDN Path` vaut `tpr/sc2` pour StarCraft II, `tpr/fenris`
// pour Diablo IV : le dernier morceau est le code du produit.
//
// ATTENTION, DEUX NOMENCLATURES. Ce code n'est pas celui que rend CascLib : `.build.info` dit
// `sc2` la ou `CascGetStorageInfo` dit `s2`. Le manifeste d'un paquet declare celui de
// `.build.info` -- c'est le seul lisible sans ouvrir le stockage. Le script, lui, verifie celui
// de CascLib une fois l'archive ouverte. Les deux controles se completent.

use std::path::{Path, PathBuf};

/// Le code produit d'une installation, lu dans son `.build.info`. `None` si le dossier n'en
/// porte pas, ou si le fichier ne dit rien d'exploitable.
pub fn produit_de(dossier: &Path) -> Option<String> {
    let texte = std::fs::read_to_string(dossier.join(".build.info")).ok()?;
    let mut lignes = texte.lines();
    let entetes: Vec<&str> = lignes.next()?.split('|').collect();
    let valeurs: Vec<&str> = lignes.next()?.split('|').collect();

    // Le nom de colonne porte son type derriere un `!` : « CDN Path!STRING:0 ».
    let colonne = |voulu: &str| {
        entetes
            .iter()
            .position(|e| e.split('!').next() == Some(voulu))
            .and_then(|i| valeurs.get(i))
            .map(|v| v.trim())
            .filter(|v| !v.is_empty())
    };

    // `Product` existe dans l'entete mais reste vide sur les deux installations mesurees : c'est
    // `CDN Path` qui porte l'information, sous la forme `tpr/<code>`.
    colonne("Product")
        .or_else(|| colonne("CDN Path").and_then(|c| c.rsplit('/').next()))
        .map(|c| c.to_lowercase())
}

/// Les dossiers ou un jeu peut vivre, du plus fiable au plus devine.
fn candidats() -> Vec<PathBuf> {
    let mut vus: Vec<PathBuf> = Vec::new();
    let mut ajouter = |chemin: PathBuf| {
        if chemin.is_dir() && !vus.contains(&chemin) {
            vus.push(chemin);
        }
    };

    #[cfg(windows)]
    for chemin in registre::emplacements_installes() {
        ajouter(chemin);
    }

    // Le filet : les racines ou l'on range des jeux, sur chaque disque. Un seul niveau de
    // profondeur -- au-dela, on parcourrait le disque pour rien.
    for lettre in 'A'..='Z' {
        for nid in ["Jeux", "Games", "Program Files (x86)", "Program Files"] {
            let racine = PathBuf::from(format!("{lettre}:\\")).join(nid);
            let Ok(entrees) = std::fs::read_dir(&racine) else { continue };
            for entree in entrees.flatten() {
                ajouter(entree.path());
            }
        }
    }
    vus
}

/// Le dossier de l'installation dont le code produit est `produit`, s'il y en a une.
pub fn trouver(produit: &str) -> Option<PathBuf> {
    if produit.trim().is_empty() {
        return None;
    }
    let voulu = produit.trim().to_lowercase();
    candidats()
        .into_iter()
        .find(|c| produit_de(c).is_some_and(|p| p == voulu))
}

// Le registre de desinstallation : chaque programme installe y depose son `InstallLocation`.
// Les deux vues, 32 et 64 bits, sont lues -- Battle.net s'inscrit dans la premiere.
#[cfg(windows)]
mod registre {
    use std::path::PathBuf;
    use windows_sys::Win32::Foundation::ERROR_SUCCESS;
    use windows_sys::Win32::System::Registry::{
        HKEY, HKEY_LOCAL_MACHINE, KEY_READ, RegCloseKey, RegEnumKeyExW, RegOpenKeyExW,
        RegQueryValueExW,
    };

    const DESINSTALLATION: [&str; 2] = [
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ];

    fn large(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(Some(0)).collect()
    }

    unsafe fn ouvrir(parent: HKEY, sous: &str) -> Option<HKEY> {
        let mut cle: HKEY = std::ptr::null_mut();
        let code =
            unsafe { RegOpenKeyExW(parent, large(sous).as_ptr(), 0, KEY_READ, &mut cle) };
        (code == ERROR_SUCCESS).then_some(cle)
    }

    unsafe fn chaine(cle: HKEY, nom: &str) -> Option<String> {
        let nom = large(nom);
        let mut taille: u32 = 0;
        let code = unsafe {
            RegQueryValueExW(cle, nom.as_ptr(), std::ptr::null(), std::ptr::null_mut(), std::ptr::null_mut(), &mut taille)
        };
        if code != ERROR_SUCCESS || taille == 0 {
            return None;
        }
        let mut tampon = vec![0u16; (taille as usize).div_ceil(2)];
        let code = unsafe {
            RegQueryValueExW(
                cle,
                nom.as_ptr(),
                std::ptr::null(),
                std::ptr::null_mut(),
                tampon.as_mut_ptr() as *mut u8,
                &mut taille,
            )
        };
        if code != ERROR_SUCCESS {
            return None;
        }
        let fin = tampon.iter().position(|&c| c == 0).unwrap_or(tampon.len());
        Some(String::from_utf16_lossy(&tampon[..fin]))
    }

    pub fn emplacements_installes() -> Vec<PathBuf> {
        let mut trouves = Vec::new();
        for racine in DESINSTALLATION {
            unsafe {
                let Some(parent) = ouvrir(HKEY_LOCAL_MACHINE, racine) else { continue };
                let mut rang = 0u32;
                loop {
                    // 255 est la longueur maximale d'un nom de cle, plus le zero final.
                    let mut nom = [0u16; 256];
                    let mut longueur = nom.len() as u32;
                    let code = RegEnumKeyExW(
                        parent,
                        rang,
                        nom.as_mut_ptr(),
                        &mut longueur,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    );
                    if code != ERROR_SUCCESS {
                        break;
                    }
                    rang += 1;

                    let nom = String::from_utf16_lossy(&nom[..longueur as usize]);
                    if let Some(enfant) = ouvrir(parent, &nom) {
                        if let Some(ou) = chaine(enfant, "InstallLocation") {
                            let ou = ou.trim().trim_end_matches('\\');
                            if !ou.is_empty() {
                                trouves.push(PathBuf::from(ou));
                            }
                        }
                        RegCloseKey(enfant);
                    }
                }
                RegCloseKey(parent);
            }
        }
        trouves
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dossier_jetable(contenu: Option<&str>) -> PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        let p = std::env::temp_dir().join(format!("ventriloque-jeu-{n}"));
        std::fs::create_dir_all(&p).unwrap();
        if let Some(c) = contenu {
            std::fs::write(p.join(".build.info"), c).unwrap();
        }
        p
    }

    // La forme exacte relevee sur l'installation du projet, colonnes vides comprises.
    const SC2: &str = "Branch!STRING:0|Active!DEC:1|CDN Path!STRING:0|Version!STRING:0|Product!STRING:0\n\
                       eu|1|tpr/sc2|5.0.16.97563|\n";

    #[test]
    fn le_code_produit_sort_du_chemin_de_cdn() {
        let d = dossier_jetable(Some(SC2));
        assert_eq!(produit_de(&d).as_deref(), Some("sc2"));
        let _ = std::fs::remove_dir_all(&d);
    }

    // Quand la colonne `Product` est remplie, c'est elle qui prime.
    #[test]
    fn la_colonne_produit_prime_si_elle_dit_quelque_chose() {
        let d = dossier_jetable(Some(
            "CDN Path!STRING:0|Product!STRING:0\ntpr/autre|fenris\n",
        ));
        assert_eq!(produit_de(&d).as_deref(), Some("fenris"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn un_dossier_sans_build_info_ne_dit_rien() {
        let d = dossier_jetable(None);
        assert_eq!(produit_de(&d), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn un_build_info_tronque_ne_fait_pas_paniquer() {
        let d = dossier_jetable(Some("Branch!STRING:0|CDN Path!STRING:0\n"));
        assert_eq!(produit_de(&d), None);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn sans_produit_demande_on_ne_cherche_rien() {
        assert_eq!(trouver(""), None);
        assert_eq!(trouver("   "), None);
    }

    // HORS SUITE : ce que la detection voit sur la machine qui l'execute. Ce n'est pas une
    // verification -- personne ne garantit qu'un jeu Blizzard y est installe -- mais c'est le
    // seul moyen de constater que le registre et `.build.info` s'accordent pour de vrai.
    //
    //   cargo test --manifest-path src-tauri\Cargo.toml jeux -- --ignored --nocapture
    #[test]
    #[ignore = "depend des jeux installes sur la machine"]
    fn ce_que_la_detection_voit_ici() {
        let mut trouves = 0;
        for candidat in candidats() {
            if let Some(produit) = produit_de(&candidat) {
                println!("  {produit:<10} {}", candidat.display());
                trouves += 1;
            }
        }
        println!("{trouves} installation(s) Blizzard");
        println!("sc2 -> {:?}", trouver("sc2"));
    }
}
