// La replique qui arrive pendant qu'on l'ecoute.
//
// Le moteur rend son premier morceau en ~190 ms et la replique entiere en une a deux secondes.
// Attendre la fin avant de jouer faisait attendre la seconde pour entendre la premiere ; ici
// chaque morceau part des qu'il existe.
//
// NE BLOQUE JAMAIS. `next()` est appele depuis le rappel audio du peripherique : s'y endormir
// ferait craquer la carte son. Quand le morceau suivant n'est pas encore la, on rend du silence
// -- inaudible, et le son reprend des qu'il arrive. C'est aussi ce qui couvre proprement le
// premier clonage d'une voix, qui prend plusieurs secondes.

use std::collections::VecDeque;
use std::num::NonZero;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use super::progress::PlaybackProgress;

pub struct SpeechStream {
    chunks: Receiver<Vec<f32>>,
    buffer: VecDeque<f32>,
    stop: Arc<AtomicBool>,
    drained: Drained,
    progress: Arc<PlaybackProgress>,
}

// SAVOIR QUAND LE SON EST SORTI, et pas seulement quand le calcul est fini.
//
// Les deux ne coincident pas : le moteur fabrique environ trois fois plus vite qu'on n'ecoute,
// donc trois repliques mises en file sont calculees bien avant que la premiere ait fini de se
// faire entendre. Une interface qui compte les calculs annonce zero pendant que le personnage
// parle encore.
//
// Le signal est pose a la DESTRUCTION de la source, ce qui couvre les deux fins d'un seul geste :
// le flux epuise -- rodio lache la source des qu'elle rend None -- et le flux coupe, ou c'est le
// lecteur qui la jette.
#[derive(Clone)]
pub struct Drained(Arc<(Mutex<bool>, Condvar)>);

impl Drained {
    fn new() -> Self {
        Self(Arc::new((Mutex::new(false), Condvar::new())))
    }

    /// Attend que le dernier echantillon soit sorti, ou que `limit` s'ecoule.
    pub fn wait(&self, limit: Duration) {
        let (lock, signal) = &*self.0;
        let mut drained = lock.lock().unwrap();
        while !*drained {
            let (guard, outcome) = signal.wait_timeout(drained, limit).unwrap();
            drained = guard;
            if outcome.timed_out() {
                return;
            }
        }
    }
}

impl Drop for SpeechStream {
    fn drop(&mut self) {
        let (lock, signal) = &*self.drained.0;
        *lock.lock().unwrap() = true;
        signal.notify_all();
    }
}

impl Iterator for SpeechStream {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        // Raccrocher coupe la voix au prochain echantillon, pas a la fin de la phrase.
        if self.stop.load(Ordering::Relaxed) {
            return None;
        }
        if let Some(sample) = self.buffer.pop_front() {
            self.progress.add_played(1);
            return Some(sample);
        }
        match self.chunks.try_recv() {
            Ok(chunk) => {
                self.buffer.extend(chunk);
                match self.buffer.pop_front() {
                    Some(sample) => {
                        self.progress.add_played(1);
                        Some(sample)
                    }
                    None => Some(0.0),
                }
            }
            // LE SILENCE D'ATTENTE NE COMPTE PAS. Il est joue, mais ce n'est pas de la replique :
            // le compter avancerait la barre pendant que la voix se clone, sans qu'on entende rien.
            Err(TryRecvError::Empty) => Some(0.0),
            // Le producteur a lache le canal : la replique est finie.
            Err(TryRecvError::Disconnected) => None,
        }
    }
}

impl rodio::Source for SpeechStream {
    // La longueur n'est pas connue d'avance -- c'est le propre d'un flux.
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> rodio::ChannelCount {
        NonZero::new(1).unwrap()
    }

    // Celle de Mimi, pas un choix. rodio reechantillonne vers le peripherique.
    fn sample_rate(&self) -> rodio::SampleRate {
        NonZero::new(crate::engine::SAMPLE_RATE).unwrap()
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

/// Les quatre bouts d'une replique en vol : ou la synthese depose ses morceaux, la source a
/// donner au lecteur, le signal de fin de son, et les compteurs que la fenetre interroge.
pub struct SpeechChannel {
    pub sink: Sender<Vec<f32>>,
    pub stream: SpeechStream,
    pub drained: Drained,
    pub progress: Arc<PlaybackProgress>,
}

pub fn speech_channel(stop: Arc<AtomicBool>) -> SpeechChannel {
    let (sink, chunks) = channel();
    let drained = Drained::new();
    let progress = Arc::new(PlaybackProgress::default());
    let stream = SpeechStream {
        chunks,
        buffer: VecDeque::new(),
        stop,
        drained: drained.clone(),
        progress: progress.clone(),
    };
    SpeechChannel { sink, stream, drained, progress }
}
