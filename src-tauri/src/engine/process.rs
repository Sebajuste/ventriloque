// Le processus du moteur : le lancer, l'attendre, et le tuer quoi qu'il arrive.
//
// LES REGLAGES DU SON VIENNENT DE `ventriloque.json`, et leurs valeurs d'origine reproduisent la
// configuration validee en jeu par le mod ai_npc (`ptt_create(..., "int8", 0.7f, 1, 0)` suivi de
// `ptt_set_eos_extra`) -- voir `settings::EngineSettings`. La precision, elle, reste fixe : le
// paquet de modeles ne livre que les poids `int8`.
//
// `--threads 0` demande la moitie des coeurs. UN SEUL FIL EST LE PIRE REGLAGE POSSIBLE -- 1,02x
// le temps reel contre 4,8x -- et c'est le defaut de la bibliotheque sous-jacente, pas un choix.

use anyhow::{Context, Result, anyhow};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use crate::settings::EngineSettings;

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
    pub fn start(
        binary: &Path,
        models: &Path,
        voices: &Path,
        port: u16,
        tuning: &EngineSettings,
    ) -> Result<Self> {
        if !binary.exists() {
            return Err(anyhow!("moteur introuvable : {}", binary.display()));
        }
        if !models.join("flow_lm_main_int8.onnx").exists() {
            return Err(anyhow!("pas de modeles dans {}", models.display()));
        }
        std::fs::create_dir_all(voices).ok();

        let mut command = Command::new(binary);
        command
            .arg("--server")
            .arg("--port")
            .arg(port.to_string())
            .arg("--models-dir")
            .arg(models)
            .arg("--voices-dir")
            .arg(voices)
            .arg("--tokenizer")
            .arg(models.join("tokenizer.model"))
            .arg("--precision")
            .arg("int8")
            .args(tuning_arguments(tuning, models));

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

/// Les drapeaux qui reglent le son. Tous passes, meme a leur valeur d'origine : ce qui tourne ne
/// depend pas des defauts d'un binaire qu'on peut remplacer.
fn tuning_arguments(tuning: &EngineSettings, models: &Path) -> Vec<String> {
    let eos_extra = tuning.eos_extra.unwrap_or_else(|| frames_after_eos(models));
    [
        ("--temperature", tuning.temperature.to_string()),
        ("--lsd-steps", tuning.lsd_steps.to_string()),
        ("--noise-clamp", tuning.noise_clamp.to_string()),
        ("--eos-threshold", tuning.eos_threshold.to_string()),
        ("--threads", tuning.threads.to_string()),
        ("--eos-extra", eos_extra.to_string()),
    ]
    .into_iter()
    .flat_map(|(flag, value)| [flag.to_string(), value])
    .collect()
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

#[cfg(test)]
mod tests {
    use super::tuning_arguments;
    use crate::settings::EngineSettings;
    use crate::testing::TempDir;

    fn value_of(arguments: &[String], flag: &str) -> String {
        let at = arguments.iter().position(|a| a == flag).expect(flag);
        arguments[at + 1].clone()
    }

    // Les valeurs d'origine doivent donner exactement la ligne de commande validee en jeu.
    #[test]
    fn a_l_origine_la_ligne_est_celle_du_jeu() {
        let models = TempDir::new("process-origine");
        std::fs::write(models.path().join("frames_after_eos.txt"), "8\n").unwrap();

        let arguments = tuning_arguments(&EngineSettings::default(), models.path());

        assert_eq!(value_of(&arguments, "--temperature"), "0.7");
        assert_eq!(value_of(&arguments, "--lsd-steps"), "1");
        assert_eq!(value_of(&arguments, "--noise-clamp"), "0");
        assert_eq!(value_of(&arguments, "--eos-threshold"), "-4");
        assert_eq!(value_of(&arguments, "--threads"), "0");
        assert_eq!(value_of(&arguments, "--eos-extra"), "8");
    }

    #[test]
    fn un_nombre_de_trames_choisi_l_emporte_sur_le_paquet() {
        let models = TempDir::new("process-trames");
        std::fs::write(models.path().join("frames_after_eos.txt"), "8").unwrap();
        let tuning = EngineSettings { eos_extra: Some(4), temperature: 0.55, ..Default::default() };

        let arguments = tuning_arguments(&tuning, models.path());

        assert_eq!(value_of(&arguments, "--eos-extra"), "4");
        assert_eq!(value_of(&arguments, "--temperature"), "0.55");
    }
}
