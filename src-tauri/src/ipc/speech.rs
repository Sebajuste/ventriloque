// Parler, se taire, et dire ou l'on en est.

use serde::Serialize;
use std::sync::atomic::Ordering;
use std::time::Duration;
use tauri::State;

use crate::app::AppState;
use crate::audio::{OutputCommand, SpeechChannel};
use crate::{audio, characters, engine};

/// Le son d'une replique tient largement dans dix minutes : au-dela, on cesse d'attendre pour
/// ne pas garder un fil bloquant a vie.
const DRAIN_LIMIT: Duration = Duration::from_secs(600);

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
#[specta::specta]
pub async fn speak(
    app: State<'_, AppState>,
    reference: String,
    text: String,
) -> Result<(), String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Ok(());
    }
    // Une nouvelle replique leve la marque : sans cela, une replique demandee apres un Silence
    // serait coupee avant d'avoir commence.
    app.stop.store(false, Ordering::Relaxed);
    // VIDER L'EMPLACEMENT TOUT DE SUITE, et pas dans le fil qui suit. Entre cet appel et le
    // demarrage du fil, la fenetre interroge l'avancement : elle lirait celui de la replique
    // PRECEDENTE, terminee, et la barre afficherait un instant « fini » pour une replique qui
    // n'a pas commence.
    *app.speech.lock().unwrap() = None;

    let engine = app.engine.clone();
    let stop = app.stop.clone();
    let output = app.output.sender();
    let slot = app.speech.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let SpeechChannel { sink, stream, drained, progress } = audio::speech_channel(stop.clone());
        *slot.lock().unwrap() = Some(progress.clone());
        // La source part AVANT la synthese : elle rend du silence en attendant le premier
        // morceau, et le son commence a l'instant ou il arrive.
        let _ = output.send(OutputCommand::Play(stream));

        // Le verrou du moteur est rendu des la fin du CALCUL, pas de la lecture. C'est ce qui
        // laisse la replique suivante se fabriquer pendant qu'on ecoute celle-ci -- et c'est
        // pour ca que le son ne manque jamais de matiere.
        {
            let guard = engine.lock().map_err(|_| "la voie de parole est cassee".to_string())?;
            match guard.as_ref() {
                Some(e) => e
                    .speak_streaming(&reference, &text, sink, stop, &progress)
                    .map_err(|e| e.to_string())?,
                None => return Err("le moteur de parole n'est pas demarre".to_string()),
            }
        }
        // La duree totale n'est connue qu'ici : avant, le compte des echantillons produits
        // grandissait encore.
        progress.finish();

        // Puis on attend que le son soit REELLEMENT sorti. Rendre la main a la fin du calcul
        // ferait annoncer « plus rien en file » pendant que le personnage parle encore.
        drained.wait(DRAIN_LIMIT);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}

// CE QUI EST SORTI DU HAUT-PARLEUR, EN MILLISECONDES.
//
// Interroge par la fenetre pendant qu'une replique joue. `duration` vaut zero tant que la
// synthese n'a pas fini : donner un pourcentage avant serait mentir, puisque le denominateur
// grandit encore et que la barre reculerait a chaque morceau qui arrive.
#[derive(Serialize, specta::Type, Default)]
pub struct SpeechProgress {
    pub position: u32,
    pub duration: u32,
    /// La duree est connue : la barre peut devenir une vraie proportion.
    pub complete: bool,
    /// Au moins un echantillon de la replique est passe. Avant, la voix se prepare.
    pub audible: bool,
}

#[tauri::command]
#[specta::specta]
pub fn speech_progress(app: State<AppState>) -> SpeechProgress {
    let guard = app.speech.lock().unwrap();
    let Some(progress) = guard.as_ref() else {
        return SpeechProgress::default();
    };
    let counts = progress.read();
    let ms = |samples: usize| (samples as u64 * 1000 / engine::SAMPLE_RATE as u64) as u32;
    SpeechProgress {
        position: ms(counts.played),
        duration: if counts.finished { ms(counts.produced) } else { 0 },
        complete: counts.finished,
        audible: counts.played > 0,
    }
}

// SUSPENDRE N'EST PAS COUPER. `silence` jette la replique ; `pause` la garde ou elle en est, et
// le calcul continue de remplir son tampon pendant ce temps. C'est le geste d'une table qui
// s'interrompt -- quelqu'un pose une question, on reprend la phrase la ou elle etait.
//
// Il n'y a pas de commande « suivant » ici, et ce n'est pas un oubli : la fenetre ne confie
// qu'UNE replique a la fois au lecteur, donc passer a la suivante, c'est couper celle-ci et
// laisser la file de la fenetre engager la prochaine. Un `skip_one` ferait exactement pareil
// pour un ordre de plus.
#[tauri::command]
#[specta::specta]
pub fn pause(app: State<AppState>) {
    app.output.send(OutputCommand::Pause);
}

#[tauri::command]
#[specta::specta]
pub fn resume(app: State<AppState>) {
    app.output.send(OutputCommand::Resume);
}

#[tauri::command]
#[specta::specta]
pub fn silence(app: State<AppState>) {
    app.stop.store(true, Ordering::Relaxed);
    *app.speech.lock().unwrap() = None;
    app.output.send(OutputCommand::Stop);
}

// `u32` ET PAS `usize` : c'est le type de la FRONTIERE, pas celui du calcul. Specta refuse
// `usize` parce qu'au-dela de 2^53 un entier perdrait des chiffres en silence cote JavaScript,
// et l'IPC de Tauri passe par du JSON, qui ne transporte pas de `bigint`. Un rang dans la liste
// des sorties audio de la machine tient largement dans 32 bits.
#[tauri::command]
#[specta::specta]
pub fn select_device(app: State<AppState>, index: u32) {
    let index = index as usize;
    *app.device.lock().unwrap() = index;
    app.output.send(OutputCommand::SelectDevice(index));
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
#[specta::specta]
pub async fn warm_up(app: State<'_, AppState>) -> Result<Vec<String>, String> {
    let wanted: Vec<String> = {
        let mut voices: Vec<String> = characters::all(&app.characters_dir)
            .into_iter()
            .map(|c| c.voice)
            .filter(|v| !v.is_empty())
            .collect();
        voices.sort();
        voices.dedup();
        voices
    };
    if wanted.is_empty() {
        return Ok(vec!["aucune fiche ne nomme de voix".into()]);
    }
    let engine = app.engine.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let guard = engine.lock().map_err(|_| "la voie de parole est cassee".to_string())?;
        let engine = guard.as_ref().ok_or("le moteur de parole n'est pas demarre")?;
        let mut said = Vec::new();
        for voice in wanted {
            let start = std::time::Instant::now();
            match engine.speak(&voice, "Bonjour.") {
                Ok(_) => {
                    said.push(format!("{voice} prete en {:.1} s", start.elapsed().as_secs_f32()))
                }
                Err(e) => said.push(format!("{voice} : {e}")),
            }
        }
        Ok(said)
    })
    .await
    .map_err(|e| e.to_string())?
}
