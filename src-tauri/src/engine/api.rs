// Ce qu'on demande au moteur, et comment il repond.
//
// EN JSON, JAMAIS EN ARGUMENT DE LIGNE DE COMMANDE. Le texte accentue ne passe pas par `argv`
// sous Windows : il arrive en ANSI et SentencePiece attend de l'UTF-8. « Ça se voit »
// deviendrait « a se voit » sans que rien ne le signale.

use anyhow::{Context, Result, anyhow};
use std::io::Read;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::time::Duration;

use crate::audio::PlaybackProgress;

use super::process::Engine;

/// Une replique tient largement dans cinq minutes de calcul : au-dela, quelque chose est casse
/// et attendre plus longtemps n'y changerait rien.
const DEADLINE: Duration = Duration::from_secs(300);

/// Un morceau de reponse HTTP, avant decoupage en flottants.
const CHUNK: usize = 8192;

impl Engine {
    fn url(&self, route: &str) -> String {
        format!("http://127.0.0.1:{}{route}", self.port)
    }

    fn post(&self, route: &str, body: &serde_json::Value) -> Result<reqwest::blocking::Response> {
        let response = reqwest::blocking::Client::new()
            .post(self.url(route))
            .json(body)
            .timeout(DEADLINE)
            .send()
            .context("le moteur n'a pas repondu")?;
        if !response.status().is_success() {
            let code = response.status();
            let said = response.text().unwrap_or_default();
            return Err(anyhow!("le moteur a refuse ({code}) : {said}"));
        }
        Ok(response)
    }

    /// Une replique, rendue entiere. Bloquant : ~1 s pour cinq secondes de son une fois la voix
    /// en cache, une poignee de secondes la premiere fois, parce que c'est la que la reference
    /// se clone.
    pub fn speak(&self, voice: &str, text: &str) -> Result<Vec<u8>> {
        let body = serde_json::json!({ "input": text, "voice": voice });
        let response = self.post("/v1/audio/speech", &body)?;
        let mut bytes = Vec::new();
        // Une borne franche plutot qu'une confiance : la reponse vient d'un processus a cote,
        // et rien ne garantit qu'il s'arrete.
        response.take(64 * 1024 * 1024).read_to_end(&mut bytes)?;
        Ok(bytes)
    }

    /// La meme replique, mais poussee vers la sortie AU FIL DE L'EAU.
    ///
    /// `/tts` rend du `audio/pcm;rate=24000;encoding=float;bits=32` en chunked : des flottants
    /// bruts, sans en-tete, des que le moteur les a. C'est la difference entre entendre le
    /// premier mot au bout de 190 ms et l'entendre au bout d'une seconde.
    ///
    /// Bloquant jusqu'a la fin de la synthese -- le son, lui, a commence bien avant. L'appelant
    /// est un fil dedie, jamais celui de la fenetre.
    pub fn speak_streaming(
        &self,
        voice: &str,
        text: &str,
        sink: Sender<Vec<f32>>,
        stop: Arc<AtomicBool>,
        progress: &PlaybackProgress,
    ) -> Result<()> {
        let body = serde_json::json!({ "text": text, "voice": voice });
        let mut response = self.post("/tts", &body)?;

        let mut raw = [0u8; CHUNK];
        // Un morceau du reseau ne tombe pas sur une frontiere de flottant : ce qui depasse
        // attend le morceau suivant plutot que d'etre jete ou mal lu.
        let mut spare: Vec<u8> = Vec::new();
        loop {
            if stop.load(Ordering::Relaxed) {
                return Ok(());
            }
            let read = response.read(&mut raw).context("lecture du flux audio")?;
            if read == 0 {
                return Ok(());
            }
            spare.extend_from_slice(&raw[..read]);
            let whole = spare.len() - spare.len() % 4;
            if whole == 0 {
                continue;
            }
            let chunk: Vec<f32> = spare[..whole]
                .chunks_exact(4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();
            spare.drain(..whole);
            // Compte AVANT l'envoi : une fois parti, le morceau ne nous appartient plus.
            progress.add_produced(chunk.len());
            // Le destinataire est parti : la replique n'interesse plus personne.
            if sink.send(chunk).is_err() {
                return Ok(());
            }
        }
    }
}
