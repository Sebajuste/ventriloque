// L'etat de l'application : ce que toutes les commandes se partagent.
//
// L'ATELIER ET LE PLAYER SONT LE MEME PROGRAMME, et c'est la seule decision d'architecture qui
// compte ici : la boucle de l'atelier -- forger, entendre, corriger -- passe par le player. Les
// separer aurait fait ecrire la lecture audio deux fois, et la voix entendue en forgeant n'aurait
// pas ete celle qu'entend la table.
//
// RIEN NE PARLE SUR LE FIL DE L'INTERFACE. Le moteur vit dans un processus a cote, la sortie son
// dans un fil a elle ; les commandes deposent du travail et rendent la main.

use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use crate::audio::{Output, PlaybackProgress, Takes};
use crate::engine::Engine;
use crate::{audio, embedded, engine, packs, paths, settings};

pub struct AppState {
    pub root: PathBuf,
    pub models_dir: PathBuf,
    pub voices_dir: PathBuf,
    pub characters_dir: PathBuf,
    /// Partage plutot que possede : la synthese tourne dans un fil a elle, et ce fil doit
    /// pouvoir tenir le moteur pendant les secondes que dure une replique.
    pub engine: Arc<Mutex<Option<Engine>>>,
    /// Leve quand le joueur coupe. La voie de parole le regarde a chaque echantillon et le fil
    /// de synthese a chaque morceau recu : les deux moities s'arretent, pas seulement le son.
    pub stop: Arc<AtomicBool>,
    /// Ce que le moteur a refuse de faire au demarrage, dit en francais. Vide quand tout va bien.
    pub failure: Mutex<String>,
    pub output: Output,
    /// L'avancement de la replique EN COURS, et il n'y en a qu'une : la fenetre ne confie au
    /// lecteur qu'une replique a la fois, donc un seul emplacement dit toujours de qui l'on parle.
    pub speech: Arc<Mutex<Option<Arc<PlaybackProgress>>>>,
    /// Les dernieres repliques synthetisees, pour les redire sans refaire le calcul.
    pub takes: Arc<Mutex<Takes>>,
    /// L'avancement de l'operation en cours sur les paquets. Un seul emplacement : les boutons
    /// se ferment pendant qu'elle tourne, donc il n'y en a jamais deux.
    pub job: Arc<packs::Job>,
    pub device: Mutex<usize>,
}

impl AppState {
    /// Prepare les dossiers et l'etat, sans encore allumer le moteur.
    pub fn new() -> Self {
        let root = paths::data_root();
        // AVANT TOUT LE RESTE : une installation faite sous les anciens noms doit etre retrouvee
        // ici, pas recreee a cote.
        crate::migration::run(&root);

        let models_dir = settings::models_dir(&root);
        let voices_dir = root.join(paths::VOICES);
        let characters_dir = root.join(paths::CHARACTERS);
        std::fs::create_dir_all(&voices_dir).ok();
        std::fs::create_dir_all(&characters_dir).ok();

        Self {
            root,
            models_dir,
            voices_dir,
            characters_dir,
            engine: Arc::new(Mutex::new(None)),
            stop: Arc::new(AtomicBool::new(false)),
            failure: Mutex::new(String::new()),
            output: Output::start(),
            speech: Arc::new(Mutex::new(None)),
            takes: Arc::new(Mutex::new(Takes::default())),
            job: Arc::new(packs::Job::default()),
            device: Mutex::new(audio::default_device()),
        }
    }

    /// Met le moteur en route -- ou le relance s'il tournait deja --, ou retient pourquoi il n'a
    /// pas pu.
    ///
    /// Appelee au demarrage, apres l'installation d'un paquet, et quand les reglages du moteur
    /// changent : une premiere ouverture sans modeles est le cas NORMAL d'un executable portable,
    /// et le moteur ne lit ses reglages qu'a son lancement.
    ///
    /// LE VERROU EST TENU DE BOUT EN BOUT. Une replique demandee pendant la relance attend le
    /// nouveau moteur au lieu d'echouer sur un emplacement vide ; une replique en cours de calcul
    /// finit avec l'ancien avant qu'on le tue.
    ///
    /// L'ANCIEN EST TUE AVANT TOUT LE RESTE, deploiement compris. Windows refuse d'ecraser un
    /// executable qui tourne, et en developpement `deploy` reecrit le moteur a chaque appel : le
    /// deployer d'abord faisait echouer chaque relance sur « acces refuse ». Le port est retire a
    /// chaque fois, sinon l'attente d'ecoute pourrait croire pret un moteur en train de mourir.
    pub fn start_engine(&self) {
        let mut slot = self.engine.lock().unwrap();
        slot.take();
        let binary = match embedded::deploy(&self.root) {
            Ok(path) => path,
            Err(e) => return self.fail(e.to_string()),
        };
        let tuning = settings::engine(&self.root);
        let started = Engine::start(
            &binary,
            &self.models_dir,
            &self.voices_dir,
            engine::free_port(),
            &tuning,
        );
        match started {
            Ok(engine) => {
                *slot = Some(engine);
                self.failure.lock().unwrap().clear();
            }
            Err(e) => self.fail(e.to_string()),
        }
    }

    fn fail(&self, message: String) {
        *self.failure.lock().unwrap() = message;
    }

    pub fn is_ready(&self) -> bool {
        self.engine.lock().is_ok_and(|guard| guard.is_some())
    }

    pub fn failure_message(&self) -> String {
        self.failure.lock().map(|f| f.clone()).unwrap_or_default()
    }

    /// FERMER LA FENETRE DOIT TUER LE MOTEUR, et rien ne le fait tout seul.
    ///
    /// `Drop for Engine` est ecrit pour ca, mais il ne s'execute jamais : Tauri termine le
    /// processus sans derouler la pile, donc les destructeurs de l'etat manage ne partent pas.
    /// Mesure le 2026-09-05 : l'application sortie avec le code 0, le moteur ecoutait toujours
    /// sur son port -- un demi-gigaoctet de modeles en memoire et le port pris pour le lancement
    /// suivant.
    ///
    /// Sortir le moteur de son emplacement suffit : c'est la destruction de la valeur, ici, qui
    /// tue le processus fils PAR SON PID.
    pub fn shutdown(&self) {
        if let Ok(mut guard) = self.engine.lock() {
            guard.take();
        }
    }
}
