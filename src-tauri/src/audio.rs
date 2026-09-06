// La sortie son : un fil dedie qui possede le peripherique, et une boite aux lettres.
//
// POURQUOI UN FIL. Le flux cpal n'est pas `Send` sous Windows -- il ne peut pas vivre dans
// l'etat partage de Tauri. Plutot que de le contourner, on lui donne un fil a lui : il ouvre le
// peripherique, garde le lecteur, et lit des ordres sur un canal. Les commandes de l'interface
// deposent un ordre et rendent la main tout de suite.
//
// CHOISIR LE PERIPHERIQUE COMPTE. Une partie se joue parfois en visio : la voix doit pouvoir
// partir dans un cable virtuel plutot que dans les enceintes du salon. C'est la seule raison
// pour laquelle la lecture est ici et pas dans la fenetre web.

use anyhow::{Result, anyhow};
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::stream::DeviceSinkBuilder;
use rodio::Player;
use std::collections::VecDeque;
use std::num::NonZero;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, TryRecvError, channel};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

pub enum Ordre {
    // Une replique qui arrive au fil de l'eau, pendant que le moteur la fabrique.
    Jouer(Flux),
    // Le silence tout de suite : ce qui joue s'arrete et ce qui attendait est jete.
    Taire,
    // Suspendre et reprendre la ou l'on en etait. Le calcul, lui, continue : il remplit le
    // tampon de la replique en cours pendant la pause, et rien ne se perd.
    Pause,
    Reprendre,
    // Rouvrir sur un autre peripherique, par son rang dans `peripheriques()`.
    Peripherique(usize),
}

pub struct Sortie {
    envoi: Sender<Ordre>,
}

// L'enumeration passe par cpal directement : le module `speakers` de rodio, plus joli, est
// derriere sa feature `experimental` et on ne batit pas une seance de jeu sur une API annoncee
// comme mouvante.
fn sorties() -> Vec<rodio::cpal::Device> {
    rodio::cpal::default_host()
        .output_devices()
        .map(|d| d.collect())
        .unwrap_or_default()
}

fn nom(peripherique: &rodio::cpal::Device) -> String {
    peripherique
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "peripherique sans nom".into())
}

pub fn peripheriques() -> Vec<String> {
    sorties().iter().map(nom).collect()
}

pub fn peripherique_defaut() -> usize {
    let defaut = rodio::cpal::default_host().default_output_device().map(|d| d.id());
    sorties().iter().position(|d| Some(d.id()) == defaut).unwrap_or(0)
}

impl Sortie {
    pub fn demarrer() -> Self {
        let (envoi, reception) = channel::<Ordre>();
        std::thread::spawn(move || {
            let mut lecteur = ouvrir(peripherique_defaut()).ok();
            while let Ok(ordre) = reception.recv() {
                match ordre {
                    Ordre::Peripherique(rang) => match ouvrir(rang) {
                        Ok(neuf) => lecteur = Some(neuf),
                        Err(e) => eprintln!("peripherique refuse : {e}"),
                    },
                    Ordre::Taire => {
                        if let Some(l) = &lecteur {
                            l.clear();
                            // `clear` laisse le lecteur en pause : sans ce `play`, la replique
                            // suivante serait mise en file et n'en sortirait jamais.
                            l.play();
                        }
                    }
                    Ordre::Pause => {
                        if let Some(l) = &lecteur {
                            l.pause();
                        }
                    }
                    Ordre::Reprendre => {
                        if let Some(l) = &lecteur {
                            l.play();
                        }
                    }
                    // `append` met en file : une replique envoyee pendant qu'une autre joue
                    // s'enchaine au lieu de la couper, ce qui est le bon comportement pour un
                    // personnage qui parle en plusieurs phrases.
                    Ordre::Jouer(flux) => {
                        if let Some(l) = &lecteur {
                            l.append(flux);
                            l.play();
                        }
                    }
                }
            }
        });
        Self { envoi }
    }

    pub fn ordonner(&self, ordre: Ordre) {
        let _ = self.envoi.send(ordre);
    }

    // De quoi parler depuis un autre fil : la synthese vit dans un fil a elle et doit pouvoir
    // deposer sa replique sans passer par l'etat de l'application.
    pub fn canal(&self) -> Sender<Ordre> {
        self.envoi.clone()
    }
}

fn ouvrir(rang: usize) -> Result<Player> {
    let disponibles = sorties();
    let sortie = disponibles
        .get(rang)
        .cloned()
        .ok_or_else(|| anyhow!("aucun peripherique de sortie au rang {rang}"))?;
    let melangeur = DeviceSinkBuilder::from_device(sortie)?.open_stream()?;
    let lecteur = Player::connect_new(melangeur.mixer());
    // Le melangeur tient le flux cpal ouvert : le laisser tomber couperait le son. Il vit aussi
    // longtemps que le fil, et le fil vit aussi longtemps que l'application.
    std::mem::forget(melangeur);
    Ok(lecteur)
}


// ── La replique qui arrive pendant qu'on l'ecoute ────────────────────────────
//
// Le moteur rend son premier morceau en ~190 ms et la replique entiere en une a deux secondes.
// Attendre la fin avant de jouer faisait attendre la seconde pour entendre la premiere ; ici
// chaque morceau part des qu'il existe.
//
// NE BLOQUE JAMAIS. `next()` est appele depuis le rappel audio du peripherique : s'y endormir
// ferait craquer la carte son. Quand le morceau suivant n'est pas encore la, on rend du silence
// -- inaudible, et le son reprend des qu'il arrive. C'est aussi ce qui couvre proprement le
// premier clonage d'une voix, qui prend plusieurs secondes.
// CE QU'ON SAIT D'UNE REPLIQUE PENDANT QU'ELLE JOUE.
//
// Deux comptes, et ils ne mesurent pas la meme chose. `produits` est ce que la synthese a
// fabrique ; `sortis` est ce qui est REELLEMENT passe dans le haut-parleur. Le moteur fabrique
// environ trois fois plus vite qu'on n'ecoute, donc les deux ne se rejoignent qu'a la fin.
//
// C'est ce qui permet de dire la verite plutot que de l'estimer : `sortis == 0` veut dire que
// rien n'a encore ete entendu -- la voix se prepare -- et non « il s'est ecoule moins de
// 250 ms ». La duree totale, elle, n'est connue qu'une fois `fini` pose : avant, `produits`
// grandit encore, et un pourcentage calcule dessus reculerait a chaque morceau qui arrive.
#[derive(Default)]
pub struct Avancement {
    produits: AtomicUsize,
    sortis: AtomicUsize,
    fini: AtomicBool,
}

impl Avancement {
    pub fn produits(&self, combien: usize) {
        self.produits.fetch_add(combien, Ordering::Relaxed);
    }

    pub fn terminer(&self) {
        self.fini.store(true, Ordering::Relaxed);
    }

    /// Echantillons sortis, echantillons fabriques, et si la fabrication est finie.
    pub fn lire(&self) -> (usize, usize, bool) {
        (
            self.sortis.load(Ordering::Relaxed),
            self.produits.load(Ordering::Relaxed),
            self.fini.load(Ordering::Relaxed),
        )
    }
}

pub struct Flux {
    morceaux: Receiver<Vec<f32>>,
    tampon: VecDeque<f32>,
    abandon: Arc<AtomicBool>,
    sorti: Sorti,
    avancement: Arc<Avancement>,
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
pub struct Sorti(Arc<(Mutex<bool>, Condvar)>);

impl Sorti {
    fn neuf() -> Self {
        Self(Arc::new((Mutex::new(false), Condvar::new())))
    }

    pub fn attendre(&self, limite: Duration) {
        let (verrou, signal) = &*self.0;
        let mut sorti = verrou.lock().unwrap();
        while !*sorti {
            let (garde, fin) = signal.wait_timeout(sorti, limite).unwrap();
            sorti = garde;
            if fin.timed_out() {
                return;
            }
        }
    }
}

impl Drop for Flux {
    fn drop(&mut self) {
        let (verrou, signal) = &*self.sorti.0;
        *verrou.lock().unwrap() = true;
        signal.notify_all();
    }
}

impl Iterator for Flux {
    type Item = rodio::Sample;

    fn next(&mut self) -> Option<Self::Item> {
        // Raccrocher coupe la voix au prochain echantillon, pas a la fin de la phrase.
        if self.abandon.load(Ordering::Relaxed) {
            return None;
        }
        if let Some(e) = self.tampon.pop_front() {
            self.avancement.sortis.fetch_add(1, Ordering::Relaxed);
            return Some(e);
        }
        match self.morceaux.try_recv() {
            Ok(morceau) => {
                self.tampon.extend(morceau);
                match self.tampon.pop_front() {
                    Some(e) => {
                        self.avancement.sortis.fetch_add(1, Ordering::Relaxed);
                        Some(e)
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

impl rodio::Source for Flux {
    // La longueur n'est pas connue d'avance -- c'est le propre d'un flux.
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        NonZero::new(1).unwrap()
    }
    // Celle de Mimi, pas un choix. rodio reechantillonne vers le peripherique.
    fn sample_rate(&self) -> rodio::SampleRate {
        NonZero::new(crate::engine::SORTIE).unwrap()
    }
    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

pub fn flux(abandon: Arc<AtomicBool>) -> (Sender<Vec<f32>>, Flux, Sorti, Arc<Avancement>) {
    let (envoi, reception) = channel();
    let sorti = Sorti::neuf();
    let avancement = Arc::new(Avancement::default());
    let flux = Flux {
        morceaux: reception,
        tampon: VecDeque::new(),
        abandon,
        sorti: sorti.clone(),
        avancement: avancement.clone(),
    };
    (envoi, flux, sorti, avancement)
}
