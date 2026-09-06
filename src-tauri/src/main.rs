// Ventriloque : un atelier qui fabrique des voix de PNJ, un player qui les joue en seance.
//
// L'ATELIER ET LE PLAYER SONT LE MEME PROGRAMME, et c'est la seule decision d'architecture qui
// compte ici : la boucle de l'atelier -- forger, entendre, corriger -- passe par le player. Les
// separer aurait fait ecrire la lecture audio deux fois, et la voix entendue en forgeant n'aurait
// pas ete celle qu'entend la table.
//
// Rien ne parle sur le fil de l'interface. Le moteur vit dans un processus a cote, la sortie son
// dans un fil a elle ; les commandes ci-dessous deposent du travail et rendent la main.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod embarque;
mod engine;
mod fiches;
mod forge;
mod packs;
mod stretch;

use audio::{Ordre, Sortie};
use engine::Moteur;
use fiches::Fiche;
use serde::Serialize;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Manager, State};


struct App {
    racine: PathBuf,
    modeles: PathBuf,
    voix: PathBuf,
    pnj: PathBuf,
    // Partage plutot que possede : la synthese tourne dans un fil a elle, et ce fil doit
    // pouvoir tenir le moteur pendant les secondes que dure une replique.
    moteur: Arc<Mutex<Option<Moteur>>>,
    // Leve quand le joueur coupe. La voie de parole le regarde a chaque echantillon et le fil
    // de synthese a chaque morceau recu : les deux moities s'arretent, pas seulement le son.
    abandon: Arc<AtomicBool>,
    // Ce que le moteur a refuse de faire au demarrage, dit en francais. Vide quand tout va bien.
    panne: Mutex<String>,
    sortie: Sortie,
    peripherique: Mutex<usize>,
    // Choisi au demarrage, une fois : le moteur ecoute dessus tant que l'application vit.
    port: u16,
}

#[derive(Serialize)]
struct Voix {
    nom: String,
    // Ce qu'on envoie au moteur : `judy.wav` pour un clone, `jean` pour une voix de catalogue.
    reference: String,
    palier: &'static str,
}

#[derive(Serialize)]
struct Etat {
    pret: bool,
    fiches: Vec<Fiche>,
    packs: Vec<packs::Manifeste>,
    panne: String,
    racine: String,
    modeles: String,
    voix: Vec<Voix>,
    peripheriques: Vec<String>,
    peripherique: usize,
}

#[tauri::command]
fn etat(app: State<App>) -> Etat {
    let mut voix: Vec<Voix> = engine::clones(&app.voix)
        .into_iter()
        .map(|f| Voix {
            nom: f.trim_end_matches(".wav").to_string(),
            reference: f,
            palier: "clone",
        })
        .collect();
    voix.extend(engine::catalogue(&app.modeles).into_iter().map(|n| Voix {
        nom: n.clone(),
        reference: n,
        palier: "catalogue",
    }));

    Etat {
        pret: app.moteur.lock().unwrap().is_some(),
        fiches: fiches::toutes(&app.pnj),
        packs: packs::installes(&app.racine),
        panne: app.panne.lock().unwrap().clone(),
        racine: app.racine.display().to_string(),
        modeles: app.modeles.display().to_string(),
        voix,
        peripheriques: audio::peripheriques(),
        peripherique: *app.peripherique.lock().unwrap(),
    }
}

// Parler sans figer la fenetre.
//
// La synthese dure une a deux secondes, et plusieurs de plus la premiere fois qu'une voix se
// clone. La faire sur le fil de la fenetre la figerait tout ce temps -- y compris le bouton qui
// sert a couper, qui est precisement celui dont on a besoin a ce moment-la.
//
// `spawn_blocking` la met sur le vivier de fils bloquants et l'attend sans rien bloquer : la
// commande ne rend la main qu'a la fin, donc l'appelant en JavaScript peut simplement l'attendre
// et recuperer l'erreur au passage. Le SON, lui, a commence bien avant -- des le premier morceau.
#[tauri::command]
async fn parler(app: State<'_, App>, reference: String, texte: String) -> Result<(), String> {
    let texte = texte.trim().to_string();
    if texte.is_empty() {
        return Ok(());
    }
    // Une nouvelle replique leve la marque : sans cela, une replique demandee apres un Silence
    // serait coupee avant d'avoir commence.
    app.abandon.store(false, Ordering::Relaxed);

    let moteur = app.moteur.clone();
    let abandon = app.abandon.clone();
    let canal = app.sortie.canal();

    tauri::async_runtime::spawn_blocking(move || {
        let (vers, flux, sorti) = audio::flux(abandon.clone());
        // La source part AVANT la synthese : elle rend du silence en attendant le premier
        // morceau, et le son commence a l'instant ou il arrive.
        let _ = canal.send(Ordre::Jouer(flux));

        // Le verrou du moteur est rendu des la fin du CALCUL, pas de la lecture. C'est ce qui
        // laisse la replique suivante se fabriquer pendant qu'on ecoute celle-ci -- et c'est
        // pour ca que le son ne manque jamais de matiere.
        {
            let garde = moteur.lock().map_err(|_| "la voie de parole est cassee".to_string())?;
            match garde.as_ref() {
                Some(m) => m.dire_en_flux(&reference, &texte, vers, abandon).map_err(|e| e.to_string())?,
                None => return Err("le moteur de parole n'est pas demarre".to_string()),
            }
        }

        // Puis on attend que le son soit REELLEMENT sorti. Rendre la main a la fin du calcul
        // ferait annoncer « plus rien en file » pendant que le personnage parle encore.
        sorti.attendre(Duration::from_secs(600));
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
fn taire(app: State<App>) {
    app.abandon.store(true, Ordering::Relaxed);
    app.sortie.ordonner(Ordre::Taire);
}

#[tauri::command]
fn choisir_peripherique(app: State<App>, rang: usize) {
    *app.peripherique.lock().unwrap() = rang;
    app.sortie.ordonner(Ordre::Peripherique(rang));
}

// L'atelier, en une commande : des fichiers en entree, une voix nommee en sortie.
//
// Le nom sert de nom de fichier. Reforger sous le meme nom est sur : le moteur invalide son
// cache sur la date du .wav, verifie, donc la voix suivante sera bien la nouvelle.
#[tauri::command]
fn forger(
    app: State<App>,
    nom: String,
    fichiers: Vec<String>,
    pitch: f32,
    formants: f32,
) -> Result<String, String> {
    let nom = nom.trim();
    if nom.is_empty() {
        return Err("il faut nommer la voix".into());
    }
    if fichiers.is_empty() {
        return Err("il faut au moins un fichier son".into());
    }
    let chemins: Vec<PathBuf> = fichiers.iter().map(PathBuf::from).collect();
    let brut = forge::assembler(&chemins, 32.0).map_err(|e| e.to_string())?;

    let recette = forge::Recipe {
        source: chemins.iter().map(|p| p.display().to_string()).collect::<Vec<_>>().join(" ; "),
        from: None,
        to: None,
        pitch,
        formants,
    };
    let faite = forge::forge(&brut, &recette).map_err(|e| e.to_string())?;

    // Le nom devient un nom de fichier : on ne garde que ce qui en fait un sans surprise.
    let sur: String = nom.chars().map(|c| if c.is_alphanumeric() { c } else { 0x5f as char }).collect();
    let fichier = format!("{sur}.wav");
    let cible = app.voix.join(&fichier);
    forge::write_wav(&faite, &cible).map_err(|e| e.to_string())?;

    // La recette a cote du resultat. Sans elle, une voix qui plait a quatre-vingt-dix pour cent
    // est intouchable : on ne peut que la refaire de zero.
    let _ = std::fs::write(
        cible.with_extension("json"),
        serde_json::to_string_pretty(&recette).unwrap_or_default(),
    );
    Ok(fichier)
}

// Le selecteur de fichiers vit en Rust et pas dans la page.
//
// L'interface est du HTML servi tel quel, sans empaqueteur : elle ne peut donc pas importer
// l'API JavaScript d'un plugin. Et un `<input type="file">` ne donnerait pas les chemins reels,
// dont l'atelier a besoin pour aller lire les sons.
#[tauri::command]
fn choisir_fichiers(fenetre: tauri::Window) -> Vec<String> {
    use tauri_plugin_dialog::DialogExt;
    fenetre
        .dialog()
        .file()
        .add_filter("sons", &["wav", "mp3", "flac", "ogg", "m4a", "opus"])
        .blocking_pick_files()
        .unwrap_or_default()
        .into_iter()
        .map(|f| f.to_string())
        .collect()
}

#[tauri::command]
fn ecrire_fiche(app: State<App>, fiche: Fiche, ancien: String) -> Result<Fiche, String> {
    fiches::ecrire(&app.pnj, fiche, &ancien).map_err(|e| e.to_string())
}

#[tauri::command]
fn supprimer_fiche(app: State<App>, id: String) -> Result<(), String> {
    fiches::supprimer(&app.pnj, &id).map_err(|e| e.to_string())
}

// Payer d'avance le clonage des voix qu'on va utiliser.
//
// LE SEUL MOMENT OU CA SE PAIE SANS GENER. La premiere replique d'une voix clonee coute ~6 s :
// le moteur encode la reference et conditionne son etat. Ces six secondes, entendues au moment
// ou un PNJ prend la parole, sont un silence que personne ne comprend ; passees pendant qu'on
// installe la table, elles n'existent pas. Ensuite le cache tient, y compris d'une seance a
// l'autre -- il vit a cote des references et s'invalide sur leur date.
//
// On rend le son et on le jette : c'est le chemin complet, et rien d'autre ne prouve qu'une
// voix est prete.
#[tauri::command]
async fn prechauffer(app: State<'_, App>) -> Result<Vec<String>, String> {
    let voulues: Vec<String> = {
        let mut v: Vec<String> = fiches::toutes(&app.pnj)
            .into_iter()
            .map(|f| f.voix)
            .filter(|v| !v.is_empty())
            .collect();
        v.sort();
        v.dedup();
        v
    };
    if voulues.is_empty() {
        return Ok(vec!["aucune fiche ne nomme de voix".into()]);
    }
    let moteur = app.moteur.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let garde = moteur.lock().map_err(|_| "la voie de parole est cassee".to_string())?;
        let m = garde.as_ref().ok_or("le moteur de parole n'est pas demarre")?;
        let mut dit = Vec::new();
        for voix in voulues {
            let debut = std::time::Instant::now();
            match m.dire(&voix, "Bonjour.") {
                Ok(_) => dit.push(format!("{voix} prete en {:.1} s", debut.elapsed().as_secs_f32())),
                Err(e) => dit.push(format!("{voix} : {e}")),
            }
        }
        Ok(dit)
    })
    .await
    .map_err(|e| e.to_string())?
}

// Met le moteur en route, ou retient pourquoi il n'a pas pu.
//
// Appelee au demarrage et apres l'installation d'un paquet : une premiere ouverture sans modeles
// est le cas NORMAL d'un executable portable, et il ne faut pas obliger a relancer l'application
// pour que le paquet qu'on vient de poser serve.
fn allumer(app: &App) {
    let binaire = match embarque::deployer(&app.racine) {
        Ok(chemin) => chemin,
        Err(e) => {
            *app.panne.lock().unwrap() = e.to_string();
            return;
        }
    };
    match Moteur::lancer(&binaire, &app.modeles, &app.voix, app.port) {
        Ok(m) => {
            *app.moteur.lock().unwrap() = Some(m);
            app.panne.lock().unwrap().clear();
        }
        Err(e) => *app.panne.lock().unwrap() = e.to_string(),
    }
}

// Installer un paquet : le zip est choisi ici, comme les sons de l'atelier, parce que la page
// n'a pas de quoi ouvrir un selecteur de fichiers.
#[tauri::command]
fn installer_pack(app: State<App>, fenetre: tauri::Window) -> Result<String, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(zip) = fenetre.dialog().file().add_filter("paquets", &["zip"]).blocking_pick_file() else {
        return Ok(String::new());
    };
    let chemin = zip.into_path().map_err(|e| e.to_string())?;
    let manifeste = packs::installer(&app.racine, &chemin).map_err(|e| e.to_string())?;

    // Un paquet de modeles rend souvent parlant ce qui ne l'etait pas : on retente tout de suite.
    let apporte_des_modeles = manifeste.fichiers.iter().any(|f| f.starts_with("modeles/"));
    if apporte_des_modeles && app.moteur.lock().unwrap().is_none() {
        allumer(&app);
    }
    Ok(format!("{} installe : {} fichier(s)", manifeste.nom, manifeste.fichiers.len()))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let racine = engine::racine();
            let modeles = lire_modeles(&racine);
            let voix = racine.join("voix");
            let pnj = racine.join("pnj");
            std::fs::create_dir_all(&voix).ok();
            std::fs::create_dir_all(&pnj).ok();

            let etat = App {
                racine,
                modeles,
                voix,
                pnj,
                moteur: Arc::new(Mutex::new(None)),
                panne: Mutex::new(String::new()),
                abandon: Arc::new(AtomicBool::new(false)),
                sortie: Sortie::demarrer(),
                peripherique: Mutex::new(audio::peripherique_defaut()),
                port: engine::port_libre(),
            };
            allumer(&etat);
            app.manage(etat);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            etat, parler, taire, choisir_peripherique, forger, choisir_fichiers,
            ecrire_fiche, supprimer_fiche, prechauffer, installer_pack
        ])
        .build(tauri::generate_context!())
        .expect("Ventriloque n'a pas pu demarrer")
        // FERMER LA FENETRE DOIT TUER LE MOTEUR, et rien ne le fait tout seul.
        //
        // `Drop for Moteur` est ecrit pour ca, mais il ne s'execute jamais : Tauri termine le
        // processus sans derouler la pile, donc les destructeurs de l'etat manage ne partent
        // pas. Mesure le 2026-09-05 : l'application sortie avec le code 0, le moteur ecoutait
        // toujours sur son port -- un demi-gigaoctet de modeles en memoire et le port pris pour
        // le lancement suivant.
        //
        // Sortir le moteur de son emplacement suffit : c'est la destruction de la valeur, ici,
        // qui tue le processus fils PAR SON PID.
        .run(|poignee, evenement| {
            if let tauri::RunEvent::Exit = evenement {
                if let Some(etat) = poignee.try_state::<App>() {
                    if let Ok(mut garde) = etat.moteur.lock() {
                        garde.take();
                    }
                }
            }
        });
}

// Ou sont les modeles. Ils pesent un demi-gigaoctet et viennent d'un depot sur liste
// d'autorisation : ils ne sont pas dans le depot, ils sont designes.
fn lire_modeles(racine: &std::path::Path) -> PathBuf {
    #[derive(serde::Deserialize)]
    struct Reglages {
        modeles: String,
    }
    std::fs::read_to_string(racine.join("ventriloque.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Reglages>(&s).ok())
        .map(|r| PathBuf::from(r.modeles))
        .unwrap_or_else(|| racine.join("modeles"))
}

