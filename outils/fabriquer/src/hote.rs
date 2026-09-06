//! Les verbes exposes au script d'un paquet, et le bac a sable qui les entoure.
//!
//! LE SCRIPT N'A AUCUNE ENTREE-SORTIE. Il ne recoit jamais un chemin, il ne peut pas en
//! fabriquer un, et aucun verbe n'en accepte : il nomme des entrees d'archive et des
//! personnages, l'hote fait le reste. Le bac a sable est donc presque vide par construction --
//! il n'y a rien a retirer, parce qu'il n'y a rien eu a ajouter.
//!
//! Ce qu'un script peut faire :
//!
//! ```text
//! produit()                     le nom de code du jeu ouvert : "s2", "fenris"
//! lister(motif)                 les entrees dont le nom concorde, en minuscules
//! taille(nom)                   la taille d'une entree, en octets
//! texte(nom)                    le contenu texte d'une entree (tables de sous-titres)
//! dire(ligne)                   une ligne de journal, que la fenetre affiche
//! voix(#{ nom, prises, duree, min, max })      monte et ecrit une reference
//! fiche(#{ id, nom, univers, voix, repliques }) ecrit une fiche de personnage
//! ```
//!
//! Ce qu'il ne peut pas faire : ouvrir un fichier, lancer un processus, joindre le reseau,
//! sortir du dossier de travail, ni tourner sans fin -- les compteurs de Rhai l'arretent.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use rhai::packages::Package;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Scope};

use crate::casc::Stockage;
use crate::montage::{self, Reglages};

/// Ce que la fabrication a produit, pour le compte rendu final.
#[derive(Default)]
pub struct Recolte {
    pub voix: Vec<String>,
    pub fiches: Vec<String>,
}

struct Atelier {
    stockage: Stockage,
    tailles: HashMap<String, u64>,
    sortie: PathBuf,
    recolte: Recolte,
}

/// Joker simple et insensible a la casse : `*` couvre n'importe quoi, `?` un caractere.
/// Sans joker, le motif est traite comme un fragment.
fn concorde(motif: &[u8], texte: &[u8]) -> bool {
    match motif.first() {
        None => texte.is_empty(),
        Some(b'*') => {
            if motif.len() == 1 {
                return true;
            }
            (0..=texte.len()).any(|i| concorde(&motif[1..], &texte[i..]))
        }
        Some(&c) => match texte.first() {
            Some(&t) if c == b'?' || c == t => concorde(&motif[1..], &texte[1..]),
            _ => false,
        },
    }
}

fn normaliser(brut: &str) -> String {
    let bas = brut.to_lowercase().replace('\\', "/");
    if bas.contains('*') || bas.contains('?') { bas } else { format!("*{bas}*") }
}

/// Un nom de fichier que le script propose, rendu sur : pas de dossier, pas de `..`, rien
/// d'invisible. Ce qui sort du script est traite comme ce qui sort d'un zip.
fn nom_sur(brut: &str) -> Result<String, Box<EvalAltResult>> {
    let propre = brut.trim();
    let acceptable = !propre.is_empty()
        && propre.len() <= 80
        && propre
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        && !propre.starts_with('.')
        && !propre.contains("..");
    if acceptable {
        Ok(propre.to_string())
    } else {
        Err(format!("nom refuse : « {brut} » (lettres, chiffres, _ - . seulement)").into())
    }
}

fn chaine(carte: &Map, cle: &str) -> String {
    carte.get(cle).and_then(|v| v.clone().into_string().ok()).unwrap_or_default()
}

fn nombre(carte: &Map, cle: &str, defaut: f32) -> f32 {
    carte
        .get(cle)
        .and_then(|v| v.as_float().ok().or_else(|| v.as_int().ok().map(|i| i as f64)))
        .map(|f| f as f32)
        .unwrap_or(defaut)
}

fn tableau(carte: &Map, cle: &str) -> Vec<String> {
    carte
        .get(cle)
        .and_then(|v| v.clone().try_cast::<Array>())
        .map(|a| a.into_iter().filter_map(|v| v.into_string().ok()).collect())
        .unwrap_or_default()
}

/// Prepare le moteur, y branche les verbes, et execute le script.
pub fn executer(script: &str, stockage: Stockage, sortie: &Path) -> Result<Recolte, String> {
    let mut stockage = stockage;
    let tailles: HashMap<String, u64> = stockage
        .entrees()
        .iter()
        .map(|e| (e.nom.to_lowercase().replace('\\', "/"), e.taille))
        .collect();

    let atelier = Rc::new(RefCell::new(Atelier {
        stockage,
        tailles,
        sortie: sortie.to_path_buf(),
        recolte: Recolte::default(),
    }));

    let mut moteur = Engine::new_raw();
    moteur.register_global_module(rhai::packages::StandardPackage::new().as_shared_module());

    // Les garde-fous. Rhai n'a aucune entree-sortie a retirer ; ce qui reste a borner, c'est le
    // temps et la memoire, pour qu'un script en faute s'arrete au lieu de manger la machine.
    moteur.set_max_operations(200_000_000);
    moteur.set_max_call_levels(64);
    moteur.set_max_expr_depths(128, 64);
    moteur.set_max_string_size(16 * 1024 * 1024);
    moteur.set_max_array_size(2_000_000);
    moteur.set_max_map_size(2_000_000);
    // `eval` permettrait a un script de se reecrire en cours de route : illisible a l'audit.
    moteur.disable_symbol("eval");

    let a = atelier.clone();
    moteur.register_fn("produit", move || a.borrow().stockage.produit());

    let a = atelier.clone();
    moteur.register_fn("dire", move |ligne: &str| {
        println!("  {ligne}");
        let _ = &a;
    });

    let a = atelier.clone();
    moteur.register_fn("lister", move |motif: &str| -> Array {
        let motif = normaliser(motif);
        let atelier = a.borrow();
        atelier
            .tailles
            .keys()
            .filter(|nom| concorde(motif.as_bytes(), nom.as_bytes()))
            .map(|nom| Dynamic::from(nom.clone()))
            .collect()
    });

    let a = atelier.clone();
    moteur.register_fn("taille", move |nom: &str| -> i64 {
        let cle = nom.to_lowercase().replace('\\', "/");
        a.borrow().tailles.get(&cle).copied().unwrap_or(0) as i64
    });

    let a = atelier.clone();
    moteur.register_fn("texte", move |nom: &str| -> Result<String, Box<EvalAltResult>> {
        let donnees = a.borrow().stockage.lire(nom).map_err(|e| -> Box<EvalAltResult> { e.into() })?;
        Ok(String::from_utf8_lossy(&donnees).into_owned())
    });

    let a = atelier.clone();
    moteur.register_fn("voix", move |reglage: Map| -> Result<Map, Box<EvalAltResult>> {
        let nom = nom_sur(&chaine(&reglage, "nom"))?;
        let prises = tableau(&reglage, "prises");
        if prises.is_empty() {
            return Err(format!("voix « {nom} » : aucune prise").into());
        }

        let reglages = Reglages {
            duree: nombre(&reglage, "duree", 32.0),
            mini: nombre(&reglage, "min", 1.0),
            maxi: nombre(&reglage, "max", 4.0),
        };

        let (sources, cible) = {
            let atelier = a.borrow();
            let mut sources = Vec::new();
            for prise in &prises {
                match atelier.stockage.lire(prise) {
                    Ok(donnees) => sources.push((prise.clone(), donnees)),
                    // Une prise illisible ne condamne pas la voix : il en reste des dizaines.
                    Err(_) => continue,
                }
            }
            (sources, atelier.sortie.join("voix").join(format!("{nom}.wav")))
        };

        let monte = montage::monter(sources, &cible, &reglages)
            .map_err(|e| -> Box<EvalAltResult> { format!("voix « {nom} » : {e}").into() })?;

        a.borrow_mut().recolte.voix.push(format!("{nom}.wav"));
        println!(
            "  {nom}.wav — {:.1}s, {} repliques, {} Hz mono ({} ecartees)",
            monte.secondes, monte.prises, monte.frequence, monte.ecartes
        );

        let mut compte = Map::new();
        compte.insert("secondes".into(), Dynamic::from_float(monte.secondes as f64));
        compte.insert("prises".into(), Dynamic::from_int(monte.prises as i64));
        compte.insert("frequence".into(), Dynamic::from_int(monte.frequence as i64));
        Ok(compte)
    });

    let a = atelier.clone();
    moteur.register_fn("fiche", move |champs: Map| -> Result<(), Box<EvalAltResult>> {
        let id = nom_sur(&chaine(&champs, "id"))?;
        if id.contains('.') {
            return Err(format!("l'identifiant « {id} » ne peut pas porter de point").into());
        }
        let voix = nom_sur(&chaine(&champs, "voix"))?;

        let fiche = serde_fiche(&id, &chaine(&champs, "nom"), &chaine(&champs, "univers"), &voix, &tableau(&champs, "repliques"));
        let atelier = a.borrow();
        let dossier = atelier.sortie.join("pnj");
        std::fs::create_dir_all(&dossier).ok();
        std::fs::write(dossier.join(format!("{id}.json")), fiche)
            .map_err(|e| -> Box<EvalAltResult> { format!("fiche « {id} » : {e}").into() })?;
        drop(atelier);

        a.borrow_mut().recolte.fiches.push(format!("{id}.json"));
        Ok(())
    });

    let mut portee = Scope::new();
    moteur
        .run_with_scope(&mut portee, script)
        .map_err(|e| format!("le script a echoue : {e}"))?;

    let recolte = std::mem::take(&mut atelier.borrow_mut().recolte);
    Ok(recolte)
}

/// La fiche, ecrite a la main : `serde_json` n'est pas dans ce binaire, et le format tient en
/// cinq champs dont un tableau de chaines.
fn serde_fiche(id: &str, nom: &str, univers: &str, voix: &str, repliques: &[String]) -> String {
    let echapper = |s: &str| {
        let mut sortie = String::with_capacity(s.len() + 8);
        for c in s.chars() {
            match c {
                '"' => sortie.push_str("\\\""),
                '\\' => sortie.push_str("\\\\"),
                '\n' => sortie.push_str("\\n"),
                '\r' => sortie.push_str("\\r"),
                '\t' => sortie.push_str("\\t"),
                c if (c as u32) < 0x20 => sortie.push_str(&format!("\\u{:04x}", c as u32)),
                c => sortie.push(c),
            }
        }
        sortie
    };

    let lignes: Vec<String> =
        repliques.iter().map(|r| format!("    \"{}\"", echapper(r))).collect();
    format!(
        "{{\n  \"id\": \"{}\",\n  \"nom\": \"{}\",\n  \"univers\": \"{}\",\n  \"voix\": \"{}\",\n  \"repliques\": [\n{}\n  ]\n}}\n",
        echapper(id),
        echapper(nom),
        echapper(univers),
        echapper(voix),
        lignes.join(",\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_joker_couvre_les_cas_usuels() {
        assert!(concorde(b"*kerrigan*", b"vo/zbriefing_kerrigan_007.ogg"));
        assert!(concorde(b"*.ogg", b"a.ogg"));
        assert!(!concorde(b"*.ogg", b"a.wav"));
        assert!(concorde(b"a?c", b"abc"));
        assert!(!concorde(b"a?c", b"ac"));
        assert!(concorde(b"*", b""));
    }

    #[test]
    fn les_noms_dangereux_sont_refuses() {
        assert!(nom_sur("sc2_kerrigan").is_ok());
        assert!(nom_sur("../../windows/system32").is_err());
        assert!(nom_sur("voix/ailleurs").is_err());
        assert!(nom_sur("..").is_err());
        assert!(nom_sur(".cache").is_err());
        assert!(nom_sur("").is_err());
    }

    #[test]
    fn la_fiche_echappe_ce_qui_casserait_le_json() {
        let json = serde_fiche("x", "Jean \"le Gros\"", "U", "x.wav", &["a\\b".into()]);
        assert!(json.contains("\\\"le Gros\\\""));
        assert!(json.contains("a\\\\b"));
    }
}
