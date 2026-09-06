// Le moteur de parole : PocketTTS, lance a cote, pilote en JSON sur un port local.
//
// LA LIGNE DE COMMANDE N'EST PAS NEGOCIABLE. Chaque drapeau ci-dessous reproduit la
// configuration deja validee en jeu par le mod ai_npc (`ptt_create(..., "int8", 0.7f, 1, 0)`
// suivi de `ptt_set_eos_extra`). En changer un, c'est quitter la seule combinaison qu'on ait
// entendue tourner.
//
// `--threads 0` demande la moitie des coeurs. UN SEUL FIL EST LE PIRE REGLAGE POSSIBLE -- 1,02x
// le temps reel contre 4,8x -- et c'est le defaut de la bibliotheque sous-jacente, pas un choix.
//
// EN JSON, JAMAIS EN ARGUMENT. Le texte accentue ne passe pas par la ligne de commande sous
// Windows : `argv` arrive en ANSI et SentencePiece attend de l'UTF-8. « Ça se voit » deviendrait
// « a se voit » sans que rien ne le signale.
//
// UN SEUL DOSSIER DE MODELES SERT LES DEUX PALIERS. Un nom de voix qui finit par `.wav` est une
// reference clonee, cherchee dans le dossier des voix ; tout autre nom est une voix de catalogue,
// lue dans `<modeles>\catalogue\<nom>.kv`. Le pack libre ne convient pas ici : il n'a pas
// d'encodeur Mimi du tout et le binaire refuse de demarrer sans.

use anyhow::{Context, Result, anyhow};
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::{Duration, Instant};

// La frequence a laquelle le moteur rend son son. Celle de Mimi, pas un choix.
//
// Elle vit ici et plus dans la forge : la forge ne touche plus a la frequence des references,
// c'est le moteur qui les ramene lui-meme. Seule la SORTIE est a 24 kHz.
pub const SORTIE: u32 = 24_000;

pub struct Moteur {
    processus: Child,
    port: u16,
}

impl Drop for Moteur {
    // PAR PID, TOUJOURS. L'utilisateur fait tourner d'autres choses ; tuer par nom d'image
    // frapperait le moteur d'une autre session en meme temps que le notre.
    fn drop(&mut self) {
        let _ = self.processus.kill();
        let _ = self.processus.wait();
    }
}

// Un port libre, plutot qu'un port fixe.
//
// Ventriloque se copie sur une cle et se lance ou l'on veut : deux exemplaires ouverts en meme
// temps sont un cas ordinaire, et avec un port fixe le second trouverait le moteur du premier --
// il parlerait avec ses voix a lui, ce qui est pire qu'une erreur franche.
//
// La fenetre entre la liberation et la reprise du port est une course theorique. Elle est
// acceptee : le seul concurrent plausible est un autre Ventriloque, et il tirera un autre numero.
pub fn port_libre() -> u16 {
    for essai in 8231..8331 {
        if std::net::TcpListener::bind(("127.0.0.1", essai)).is_ok() {
            return essai;
        }
    }
    8231
}

fn frames_apres_eos(modeles: &Path) -> i32 {
    std::fs::read_to_string(modeles.join("frames_after_eos.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(-1)
}

// LIER LE MOTEUR A LA VIE DE L'APPLICATION, par le systeme et pas par notre bonne volonte.
//
// `Drop for Moteur` ne suffit pas, et c'est mesure : le 2026-09-05, la fenetre fermee, le
// processus sorti avec le code 0, le moteur ecoutait toujours sur son port -- un demi-gigaoctet
// de modeles en memoire et le port pris pour le lancement suivant. Tauri termine le processus
// sans derouler la pile ; aucun destructeur de l'etat manage ne part.
//
// Un « job object » deplace la question hors de notre code. L'enfant y est inscrit, le drapeau
// KILL_ON_JOB_CLOSE dit au systeme de le tuer quand le dernier descripteur du job se ferme, et
// ce descripteur se ferme quand NOTRE processus disparait -- proprement, en plantant, ou tue de
// force. Il n'y a plus de chemin de sortie qui laisse un orphelin.
//
// Le descripteur est volontairement abandonne : sa fermeture par le systeme EST le mecanisme.
#[cfg(windows)]
pub fn lier_a_notre_vie(enfant: &Child) {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };

    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return;
        }
        let mut reglages: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        reglages.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &reglages as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        AssignProcessToJobObject(job, enfant.as_raw_handle() as _);
    }
}

impl Moteur {
    pub fn lancer(binaire: &Path, modeles: &Path, voix: &Path, port: u16) -> Result<Self> {
        if !binaire.exists() {
            return Err(anyhow!("moteur introuvable : {}", binaire.display()));
        }
        if !modeles.join("flow_lm_main_int8.onnx").exists() {
            return Err(anyhow!("pas de modeles dans {}", modeles.display()));
        }
        std::fs::create_dir_all(voix).ok();

        let mut commande = Command::new(binaire);
        commande
            .arg("--server").arg("--port").arg(port.to_string())
            .arg("--models-dir").arg(modeles)
            .arg("--voices-dir").arg(voix)
            .arg("--tokenizer").arg(modeles.join("tokenizer.model"))
            .arg("--precision").arg("int8")
            .arg("--temperature").arg("0.7")
            .arg("--lsd-steps").arg("1")
            .arg("--threads").arg("0")
            .arg("--eos-extra").arg(frames_apres_eos(modeles).to_string());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Sans cela, une console noire s'ouvre a cote de la fenetre a chaque lancement.
            commande.creation_flags(0x0800_0000);
        }

        let processus = commande.spawn().context("lancement du moteur de parole")?;
        #[cfg(windows)]
        lier_a_notre_vie(&processus);
        let moteur = Moteur { processus, port };
        moteur.attendre(Duration::from_secs(60))?;
        Ok(moteur)
    }

    // Le modele met ~2,5 s a se charger. On attend qu'il ecoute plutot que de dormir a
    // l'aveugle : sur une machine plus lente, une attente fixe serait soit fausse soit longue.
    fn attendre(&self, limite: Duration) -> Result<()> {
        let debut = Instant::now();
        while debut.elapsed() < limite {
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        Err(anyhow!("le moteur n'a pas repondu en {} s", limite.as_secs()))
    }

    // Une replique, rendue entiere. Bloquant : ~1 s pour cinq secondes de son une fois la voix
    // en cache, une poignee de secondes la premiere fois, parce que c'est la que la reference
    // se clone.
    pub fn dire(&self, voix: &str, texte: &str) -> Result<Vec<u8>> {
        let corps = serde_json::json!({ "input": texte, "voice": voix });
        let reponse = reqwest::blocking::Client::new()
            .post(format!("http://127.0.0.1:{}/v1/audio/speech", self.port))
            .json(&corps)
            .timeout(Duration::from_secs(300))
            .send()
            .context("le moteur n'a pas repondu")?;
        if !reponse.status().is_success() {
            let code = reponse.status();
            let dit = reponse.text().unwrap_or_default();
            return Err(anyhow!("le moteur a refuse ({code}) : {dit}"));
        }
        let mut octets = Vec::new();
        reponse.take(64 * 1024 * 1024).read_to_end(&mut octets)?;
        Ok(octets)
    }
    // La meme replique, mais poussee vers la sortie AU FIL DE L'EAU.
    //
    // `/tts` rend du `audio/pcm;rate=24000;encoding=float;bits=32` en chunked : des flottants
    // bruts, sans en-tete, des que le moteur les a. C'est la difference entre entendre le
    // premier mot au bout de 190 ms et l'entendre au bout d'une seconde.
    //
    // Bloquant jusqu'a la fin de la synthese -- le son, lui, a commence bien avant. L'appelant
    // est un fil dedie, jamais celui de la fenetre.
    pub fn dire_en_flux(
        &self,
        voix: &str,
        texte: &str,
        vers: Sender<Vec<f32>>,
        abandon: Arc<AtomicBool>,
        avancement: &crate::audio::Avancement,
    ) -> Result<()> {
        let corps = serde_json::json!({ "text": texte, "voice": voix });
        let mut reponse = reqwest::blocking::Client::new()
            .post(format!("http://127.0.0.1:{}/tts", self.port))
            .json(&corps)
            .timeout(Duration::from_secs(300))
            .send()
            .context("le moteur n'a pas repondu")?;
        if !reponse.status().is_success() {
            let code = reponse.status();
            let dit = reponse.text().unwrap_or_default();
            return Err(anyhow!("le moteur a refuse ({code}) : {dit}"));
        }

        let mut brut = [0u8; 8192];
        // Un morceau du reseau ne tombe pas sur une frontiere de flottant : ce qui depasse
        // attend le morceau suivant plutot que d'etre jete ou mal lu.
        let mut reste: Vec<u8> = Vec::new();
        loop {
            if abandon.load(Ordering::Relaxed) {
                return Ok(());
            }
            let lus = reponse.read(&mut brut).context("lecture du flux audio")?;
            if lus == 0 {
                return Ok(());
            }
            reste.extend_from_slice(&brut[..lus]);
            let entiers = reste.len() - reste.len() % 4;
            if entiers == 0 {
                continue;
            }
            let morceau: Vec<f32> = reste[..entiers]
                .chunks_exact(4)
                .map(|o| f32::from_le_bytes([o[0], o[1], o[2], o[3]]))
                .collect();
            reste.drain(..entiers);
            // Compte AVANT l'envoi : une fois parti, le morceau ne nous appartient plus.
            avancement.produits(morceau.len());
            // Le destinataire est parti : la replique n'interesse plus personne.
            if vers.send(morceau).is_err() {
                return Ok(());
            }
        }
    }
}

// Les voix de catalogue livrees avec les modeles. Elles ne coutent aucun clonage -- une voix de
// catalogue EST un etat deja calcule, elle se charge au lieu de se conditionner -- et c'est le
// palier qui parle quand on n'a encore rien fabrique.
pub fn catalogue(modeles: &Path) -> Vec<String> {
    let mut noms: Vec<String> = std::fs::read_dir(modeles.join("catalogue"))
        .map(|entrees| {
            entrees
                .flatten()
                .filter_map(|e| {
                    let p = e.path();
                    (p.extension()? == "kv").then(|| p.file_stem()?.to_str().map(str::to_owned))?
                })
                .collect()
        })
        .unwrap_or_default();
    noms.sort();
    noms
}

// Les references fabriquees, c'est-a-dire les voix clonees disponibles.
pub fn clones(voix: &Path) -> Vec<String> {
    let mut noms: Vec<String> = std::fs::read_dir(voix)
        .map(|entrees| {
            entrees
                .flatten()
                .filter_map(|e| {
                    let p = e.path();
                    (p.extension()? == "wav").then(|| p.file_name()?.to_str().map(str::to_owned))?
                })
                .collect()
        })
        .unwrap_or_default();
    noms.sort();
    noms
}

// Ou vivent les fichiers, en partant de l'executable.
//
// LE REPERE EST `ventriloque.json`, pas le dossier de l'executable. En developpement le binaire
// est dans `src-tauri\target\release\` et les reglages sont quatre crans plus haut ; une fois
// empaquete les deux sont cote a cote. Un seul repere couvre les deux cas, la ou une regle par
// cas se serait trompee sur l'un des deux.
pub fn racine() -> PathBuf {
    if let Ok(force) = std::env::var("VENTRILOQUE_RACINE") {
        return PathBuf::from(force);
    }
    let exe = std::env::current_exe().unwrap_or_default();
    let depart = exe.parent().map(Path::to_path_buf).unwrap_or_default();
    let mut dossier = depart.clone();
    for _ in 0..6 {
        if dossier.join("ventriloque.json").is_file() {
            return dossier;
        }
        match dossier.parent() {
            Some(p) => dossier = p.to_path_buf(),
            None => break,
        }
    }
    depart
}

