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

#[cfg(windows)]
mod battle_net;
mod build_info;
#[cfg(windows)]
mod windows_registry;

use std::path::PathBuf;

pub use build_info::product_at;

/// Les racines ou l'on range des jeux, en dernier recours.
const NESTS: [&str; 4] = ["Jeux", "Games", "Program Files (x86)", "Program Files"];

/// Les dossiers ou un jeu peut vivre, du plus fiable au plus devine.
fn candidates() -> Vec<PathBuf> {
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut add = |path: PathBuf| {
        if path.is_dir() && !seen.contains(&path) {
            seen.push(path);
        }
    };

    #[cfg(windows)]
    for path in windows_registry::install_locations() {
        add(path);
    }

    // Le filet : les racines usuelles, plus celle que Battle.net s'est choisie -- c'est elle
    // qui rattrape une installation posee hors des chemins habituels. Un seul niveau de
    // profondeur : au-dela, on parcourrait le disque pour rien.
    let mut nests: Vec<PathBuf> = Vec::new();
    #[cfg(windows)]
    if let Some(chosen) = battle_net::default_install_path() {
        nests.push(chosen);
    }
    for letter in 'A'..='Z' {
        for nest in NESTS {
            nests.push(PathBuf::from(format!("{letter}:\\")).join(nest));
        }
    }

    for nest in nests {
        let Ok(entries) = std::fs::read_dir(&nest) else { continue };
        for entry in entries.flatten() {
            add(entry.path());
        }
    }
    seen
}

/// Un marqueur venu d'un manifeste n'a le droit de designer qu'un fichier SOUS le dossier du
/// jeu. Sans ce controle, `C:/Windows/notepad.exe` ferait concorder le premier candidat venu --
/// `join` sur un chemin absolu remplace la base -- et l'on fabriquerait depuis le mauvais
/// dossier.
fn is_safe_relative(marker: &str) -> bool {
    let m = marker.trim();
    !m.is_empty()
        && !m.contains("..")
        && !m.starts_with('/')
        && !m.starts_with('\\')
        && !m.contains(':')
}

/// Le dossier de l'installation que decrit la recette, s'il y en a une.
///
/// DEUX FACONS DE RECONNAITRE UN JEU, parce qu'ils ne se ressemblent pas tous. Les jeux Blizzard
/// portent un `.build.info` qui les nomme ; Cyberpunk n'a rien de tel, et se reconnait a la
/// presence de son executable. Le `marker` prime quand le paquet en donne un.
pub fn find_install(product: &str, marker: &str) -> Option<PathBuf> {
    if is_safe_relative(marker) {
        let marker = marker.trim();
        return candidates().into_iter().find(|c| c.join(marker).exists());
    }
    if product.trim().is_empty() {
        return None;
    }
    let wanted = product.trim().to_lowercase();
    candidates().into_iter().find(|c| product_at(c).is_some_and(|p| p == wanted))
}

#[cfg(test)]
mod tests {
    use super::*;

    // Un marqueur qui sort du dossier du jeu ferait concorder n'importe quel candidat.
    #[test]
    fn un_marqueur_doit_rester_sous_le_dossier_du_jeu() {
        assert!(is_safe_relative("bin/x64/Cyberpunk2077.exe"));
        assert!(is_safe_relative(".build.info"));
        assert!(!is_safe_relative("C:/Windows/notepad.exe"));
        assert!(!is_safe_relative("/etc/passwd"));
        assert!(!is_safe_relative("\\\\serveur\\part"));
        assert!(!is_safe_relative("../../ailleurs.exe"));
        assert!(!is_safe_relative(""));
        assert!(!is_safe_relative("   "));
    }

    #[test]
    fn sans_produit_demande_on_ne_cherche_rien() {
        assert_eq!(find_install("", ""), None);
        assert_eq!(find_install("   ", ""), None);
    }

    // HORS SUITE : ce que la detection voit sur la machine qui l'execute. Ce n'est pas une
    // verification -- personne ne garantit qu'un jeu Blizzard y est installe -- mais c'est le
    // seul moyen de constater que le registre et `.build.info` s'accordent pour de vrai.
    //
    //   cargo test --manifest-path src-tauri\Cargo.toml games -- --ignored --nocapture
    #[test]
    #[ignore = "depend des jeux installes sur la machine"]
    fn ce_que_la_detection_voit_ici() {
        let mut found = 0;
        for candidate in candidates() {
            if let Some(product) = product_at(&candidate) {
                println!("  {product:<10} {}", candidate.display());
                found += 1;
            }
        }
        println!("{found} installation(s) Blizzard");
        println!("sc2 (par .build.info)  -> {:?}", find_install("sc2", ""));
        println!(
            "cp77 (par marqueur)     -> {:?}",
            find_install("cp77", "bin/x64/Cyberpunk2077.exe")
        );
    }
}
