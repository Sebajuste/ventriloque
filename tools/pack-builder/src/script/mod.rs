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
//! product()                     le nom de code du jeu ouvert : "s2", "cp77"
//! list(motif)                   les entrees dont le nom concorde, en minuscules
//! size(nom)                     la taille d'une entree, en octets
//! text(nom)                     le contenu texte d'une entree (tables de sous-titres)
//! log(ligne)                    une ligne de journal, que la fenetre affiche
//! voice(#{ name, takes, target, min, max })          monte et ecrit une reference
//! character(#{ id, name, universe, voice, lines })   ecrit une fiche de personnage
//! ```
//!
//! LES VERBES FRANCAIS REPONDENT ENCORE -- `produit`, `lister`, `taille`, `texte`, `dire`,
//! `voix`, `fiche` --, et les cles de leurs tables aussi. Une recette est un fichier de donnees
//! qui part chez les gens : la renommer chez nous ne la renomme pas chez eux.
//!
//! Ce qu'un script ne peut pas faire : ouvrir un fichier, lancer un processus, joindre le
//! reseau, sortir du dossier de travail, ni tourner sans fin -- les compteurs de Rhai l'arretent.

mod character;
mod safety;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use rhai::packages::Package;
use rhai::{Array, Dynamic, Engine, EvalAltResult, Map, Scope};
use voice_assembly::{Settings, assemble};

use crate::storage::Storage;
use character::Character;
use safety::{matches, normalise, safe_name};

/// Les dossiers ou la moisson ira chercher, et que l'application connait.
const VOICES_DIR: &str = "voices";
const CHARACTERS_DIR: &str = "characters";

/// Ce que la fabrication a produit, pour le compte rendu final.
#[derive(Default)]
pub struct Harvest {
    pub voices: Vec<String>,
    pub characters: Vec<String>,
}

/// L'etat que les verbes se partagent. Un seul fil le touche : Rhai tourne ici, et le stockage
/// n'est pas partageable entre fils.
struct Session {
    storage: Storage,
    /// Le nom en minuscules de chaque entree et sa taille, pour repondre a `list` et `size` sans
    /// reparcourir le stockage a chaque appel.
    sizes: HashMap<String, u64>,
    out_dir: PathBuf,
    harvest: Harvest,
}

/// Lit un champ, sous son nom d'aujourd'hui ou celui d'avant.
fn field<'a>(map: &'a Map, name: &str, legacy: &str) -> Option<&'a Dynamic> {
    map.get(name).or_else(|| map.get(legacy))
}

fn string_field(map: &Map, name: &str, legacy: &str) -> String {
    field(map, name, legacy)
        .and_then(|v| v.clone().into_string().ok())
        .unwrap_or_default()
}

fn number_field(map: &Map, name: &str, legacy: &str, fallback: f32) -> f32 {
    field(map, name, legacy)
        .and_then(|v| v.as_float().ok().or_else(|| v.as_int().ok().map(|i| i as f64)))
        .map(|f| f as f32)
        .unwrap_or(fallback)
}

fn array_field(map: &Map, name: &str, legacy: &str) -> Vec<String> {
    field(map, name, legacy)
        .and_then(|v| v.clone().try_cast::<Array>())
        .map(|a| a.into_iter().filter_map(|v| v.into_string().ok()).collect())
        .unwrap_or_default()
}

/// Prepare le moteur, y branche les verbes, et execute le script.
pub fn run(script: &str, storage: Storage, out_dir: &Path) -> Result<Harvest, String> {
    let mut storage = storage;
    let sizes: HashMap<String, u64> = storage
        .entries()
        .iter()
        .map(|e| (e.name.to_lowercase().replace('\\', "/"), e.size))
        .collect();

    let session = Rc::new(RefCell::new(Session {
        storage,
        sizes,
        out_dir: out_dir.to_path_buf(),
        harvest: Harvest::default(),
    }));

    let mut engine = Engine::new_raw();
    engine.register_global_module(rhai::packages::StandardPackage::new().as_shared_module());
    guard(&mut engine);
    register_verbs(&mut engine, &session);

    let mut scope = Scope::new();
    engine
        .run_with_scope(&mut scope, script)
        .map_err(|e| format!("le script a echoue : {e}"))?;

    Ok(std::mem::take(&mut session.borrow_mut().harvest))
}

/// Les garde-fous. Rhai n'a aucune entree-sortie a retirer ; ce qui reste a borner, c'est le
/// temps et la memoire, pour qu'un script en faute s'arrete au lieu de manger la machine.
fn guard(engine: &mut Engine) {
    engine.set_max_operations(200_000_000);
    engine.set_max_call_levels(64);
    engine.set_max_expr_depths(128, 64);
    engine.set_max_string_size(16 * 1024 * 1024);
    engine.set_max_array_size(2_000_000);
    engine.set_max_map_size(2_000_000);
    // `eval` permettrait a un script de se reecrire en cours de route : illisible a l'audit.
    engine.disable_symbol("eval");
}

fn register_verbs(engine: &mut Engine, session: &Rc<RefCell<Session>>) {
    // Chaque verbe est inscrit deux fois : sous son nom d'aujourd'hui et sous celui d'avant.
    // Une recette deja distribuee continue de tourner sans qu'on ait a la reecrire.
    let s = session.clone();
    let product = move || s.borrow().storage.product();
    engine.register_fn("product", product.clone());
    engine.register_fn("produit", product);

    let log = |line: &str| println!("  {line}");
    engine.register_fn("log", log);
    engine.register_fn("dire", log);

    let s = session.clone();
    let list = move |pattern: &str| -> Array {
        let pattern = normalise(pattern);
        let session = s.borrow();
        session
            .sizes
            .keys()
            .filter(|name| matches(pattern.as_bytes(), name.as_bytes()))
            .map(|name| Dynamic::from(name.clone()))
            .collect()
    };
    engine.register_fn("list", list.clone());
    engine.register_fn("lister", list);

    let s = session.clone();
    let size = move |name: &str| -> i64 {
        let key = name.to_lowercase().replace('\\', "/");
        s.borrow().sizes.get(&key).copied().unwrap_or(0) as i64
    };
    engine.register_fn("size", size.clone());
    engine.register_fn("taille", size);

    let s = session.clone();
    let text = move |name: &str| -> Result<String, Box<EvalAltResult>> {
        let data = s.borrow().storage.read(name).map_err(|e| -> Box<EvalAltResult> { e.into() })?;
        Ok(String::from_utf8_lossy(&data).into_owned())
    };
    engine.register_fn("text", text.clone());
    engine.register_fn("texte", text);

    let s = session.clone();
    let voice = move |request: Map| -> Result<Map, Box<EvalAltResult>> { make_voice(&s, request) };
    engine.register_fn("voice", voice.clone());
    engine.register_fn("voix", voice);

    let s = session.clone();
    let character =
        move |fields: Map| -> Result<(), Box<EvalAltResult>> { write_character(&s, fields) };
    engine.register_fn("character", character.clone());
    engine.register_fn("fiche", character);
}

/// Monte une reference a partir des prises que le script nomme.
fn make_voice(session: &Rc<RefCell<Session>>, request: Map) -> Result<Map, Box<EvalAltResult>> {
    let name = safe_name(&string_field(&request, "name", "nom"))?;
    let takes = array_field(&request, "takes", "prises");
    if takes.is_empty() {
        return Err(format!("voix « {name} » : aucune prise").into());
    }

    let settings = Settings {
        target_secs: number_field(&request, "target", "duree", 32.0),
        min_secs: number_field(&request, "min", "min", 1.0),
        max_secs: number_field(&request, "max", "max", 4.0),
    };

    let (sources, target) = {
        let session = session.borrow();
        let mut sources = Vec::new();
        for take in &takes {
            match session.storage.read(take) {
                Ok(data) => sources.push((take.clone(), data)),
                // Une prise illisible ne condamne pas la voix : il en reste des dizaines.
                Err(_) => continue,
            }
        }
        (sources, session.out_dir.join(VOICES_DIR).join(format!("{name}.wav")))
    };

    let made = assemble(sources, &target, &settings)
        .map_err(|e| -> Box<EvalAltResult> { format!("voix « {name} » : {e}").into() })?;

    session.borrow_mut().harvest.voices.push(format!("{name}.wav"));
    println!(
        "  {name}.wav — {:.1}s, {} repliques, {} Hz mono ({} ecartees)",
        made.seconds, made.takes, made.sample_rate, made.rejected
    );

    let mut report = Map::new();
    report.insert("seconds".into(), Dynamic::from_float(made.seconds as f64));
    report.insert("takes".into(), Dynamic::from_int(made.takes as i64));
    report.insert("sample_rate".into(), Dynamic::from_int(made.sample_rate as i64));
    // Les cles d'avant, pour une recette qui lirait le compte rendu.
    report.insert("secondes".into(), Dynamic::from_float(made.seconds as f64));
    report.insert("prises".into(), Dynamic::from_int(made.takes as i64));
    report.insert("frequence".into(), Dynamic::from_int(made.sample_rate as i64));
    Ok(report)
}

/// Ecrit la fiche d'un personnage.
fn write_character(
    session: &Rc<RefCell<Session>>,
    fields: Map,
) -> Result<(), Box<EvalAltResult>> {
    let id = safe_name(&string_field(&fields, "id", "id"))?;
    if id.contains('.') {
        return Err(format!("l'identifiant « {id} » ne peut pas porter de point").into());
    }
    let voice = safe_name(&string_field(&fields, "voice", "voix"))?;

    let json = character::to_json(&Character {
        id: id.clone(),
        name: string_field(&fields, "name", "nom"),
        universe: string_field(&fields, "universe", "univers"),
        voice,
        lines: array_field(&fields, "lines", "repliques"),
    });

    let folder = session.borrow().out_dir.join(CHARACTERS_DIR);
    std::fs::create_dir_all(&folder).ok();
    std::fs::write(folder.join(format!("{id}.json")), json)
        .map_err(|e| -> Box<EvalAltResult> { format!("fiche « {id} » : {e}").into() })?;

    session.borrow_mut().harvest.characters.push(format!("{id}.json"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(&str, Dynamic)]) -> Map {
        pairs.iter().map(|(k, v)| ((*k).into(), v.clone())).collect()
    }

    #[test]
    fn un_champ_se_lit_sous_son_nom_d_aujourd_hui() {
        let m = map(&[("name", Dynamic::from("Judy".to_string()))]);
        assert_eq!(string_field(&m, "name", "nom"), "Judy");
    }

    // Une recette deja distribuee porte les cles francaises.
    #[test]
    fn un_champ_se_lit_encore_sous_son_ancien_nom() {
        let m = map(&[("nom", Dynamic::from("Judy".to_string()))]);
        assert_eq!(string_field(&m, "name", "nom"), "Judy");
    }

    #[test]
    fn le_nom_d_aujourd_hui_l_emporte_sur_l_ancien() {
        let m = map(&[
            ("name", Dynamic::from("neuf".to_string())),
            ("nom", Dynamic::from("ancien".to_string())),
        ]);
        assert_eq!(string_field(&m, "name", "nom"), "neuf");
    }

    #[test]
    fn un_nombre_se_lit_entier_ou_flottant() {
        assert_eq!(number_field(&map(&[("target", Dynamic::from_int(20))]), "target", "duree", 1.0), 20.0);
        assert_eq!(
            number_field(&map(&[("duree", Dynamic::from_float(2.5))]), "target", "duree", 1.0),
            2.5
        );
        // Absent : la valeur de repli, et pas zero.
        assert_eq!(number_field(&Map::new(), "target", "duree", 32.0), 32.0);
    }

    #[test]
    fn un_tableau_absent_rend_une_liste_vide() {
        assert!(array_field(&Map::new(), "lines", "repliques").is_empty());
    }

    #[test]
    fn un_tableau_se_lit_sous_les_deux_noms() {
        let lines = Dynamic::from(vec![Dynamic::from("a".to_string())]);
        assert_eq!(array_field(&map(&[("repliques", lines)]), "lines", "repliques"), vec!["a"]);
    }
}
