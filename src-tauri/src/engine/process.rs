// Le processus du moteur : le lancer, l'attendre, et le tuer quoi qu'il arrive.
//
// LA LIGNE DE COMMANDE N'EST PAS NEGOCIABLE. Chaque drapeau ci-dessous reproduit la
// configuration deja validee en jeu par le mod ai_npc (`ptt_create(..., "int8", 0.7f, 1, 0)`
// suivi de `ptt_set_eos_extra`). En changer un, c'est quitter la seule combinaison qu'on ait
// entendue tourner.
//
// `--threads 0` demande la moitie des coeurs. UN SEUL FIL EST LE PIRE REGLAGE POSSIBLE -- 1,02x
// le temps reel contre 4,8x -- et c'est le defaut de la bibliotheque sous-jacente, pas un choix.

use anyhow::{Context, Result, anyhow};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

/// Combien de temps le modele a pour se charger. Il met ~2,5 s sur une machine ordinaire ; la
/// marge couvre un disque lent ou un antivirus curieux.
const STARTUP: Duration = Duration::from_secs(60);

pub struct Engine {
    process: Child,
    pub(super) port: u16,
}

impl Drop for Engine {
    // PAR PID, TOUJOURS. L'utilisateur fait tourner d'autres choses ; tuer par nom d'image
    // frapperait le moteur d'une autre session en meme temps que le notre.
    fn drop(&mut self) {
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}

impl Engine {
    pub fn start(binary: &Path, models: &Path, voices: &Path, port: u16) -> Result<Self> {
        if !binary.exists() {
            return Err(anyhow!("moteur introuvable : {}", binary.display()));
        }
        if !models.join("flow_lm_main_int8.onnx").exists() {
            return Err(anyhow!("pas de modeles dans {}", models.display()));
        }
        std::fs::create_dir_all(voices).ok();

        let mut command = Command::new(binary);
        command
            .arg("--server").arg("--port").arg(port.to_string())
            .arg("--models-dir").arg(models)
            .arg("--voices-dir").arg(voices)
            .arg("--tokenizer").arg(models.join("tokenizer.model"))
            .arg("--precision").arg("int8")
            .arg("--temperature").arg("0.7")
            .arg("--lsd-steps").arg("1")
            .arg("--threads").arg("0")
            .arg("--eos-extra").arg(frames_after_eos(models).to_string());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            // Sans cela, une console noire s'ouvre a cote de la fenetre a chaque lancement.
            command.creation_flags(0x0800_0000);
        }

        let process = command.spawn().context("lancement du moteur de parole")?;
        #[cfg(windows)]
        tie_to_process_lifetime(&process);
        let engine = Engine { process, port };
        engine.wait_until_listening(STARTUP)?;
        Ok(engine)
    }

    // On attend qu'il ecoute plutot que de dormir a l'aveugle : sur une machine plus lente, une
    // attente fixe serait soit fausse soit longue.
    fn wait_until_listening(&self, limit: Duration) -> Result<()> {
        let start = Instant::now();
        while start.elapsed() < limit {
            if TcpStream::connect(("127.0.0.1", self.port)).is_ok() {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(150));
        }
        Err(anyhow!("le moteur n'a pas repondu en {} s", limit.as_secs()))
    }
}

/// Combien de trames laisser courir apres la fin de phrase, tel que le paquet de modeles le dit.
fn frames_after_eos(models: &Path) -> i32 {
    std::fs::read_to_string(models.join("frames_after_eos.txt"))
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(-1)
}

// LIER LE MOTEUR A LA VIE DE L'APPLICATION, par le systeme et pas par notre bonne volonte.
//
// `Drop for Engine` ne suffit pas, et c'est mesure : le 2026-09-05, la fenetre fermee, le
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
pub fn tie_to_process_lifetime(child: &Child) {
    use std::os::windows::io::AsRawHandle;
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
        SetInformationJobObject,
    };

    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return;
        }
        let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &limits as *const _ as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        AssignProcessToJobObject(job, child.as_raw_handle() as _);
    }
}
