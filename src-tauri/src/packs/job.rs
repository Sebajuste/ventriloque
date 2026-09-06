// L'avancement de l'operation en cours sur les paquets, lu par la fenetre a intervalle.
//
// UN SEUL EMPLACEMENT SUFFIT, comme pour l'avancement d'une replique : les boutons se ferment
// pendant qu'une operation tourne, donc il n'y en a jamais deux a la fois.
//
// C'est un etat partage plutot qu'un evenement parce que c'est ce que le projet fait deja pour
// la parole (`audio::PlaybackProgress`) : la fenetre bat toutes les 200 ms et lit. Un evenement
// de plus aurait demande un second mecanisme pour la meme chose.
//
// LES PAS N'ONT PAS D'UNITE. Une installation les compte en kilooctets, un retrait en fichiers :
// la fenetre n'en tire qu'une proportion, et c'est `step` qui dit ce qui se passe en francais.
// Compter les entrees d'un zip aurait donne une barre inutile -- le paquet moteur a dix-sept
// entrees dont une de 204 Mo, donc dix-sept sauts dont un qui dure cinq secondes.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Default)]
pub struct Job {
    active: AtomicBool,
    done: AtomicUsize,
    total: AtomicUsize,
    step: Mutex<String>,
    log: Mutex<Vec<String>>,
}

/// Ce que la fenetre lit d'un coup, pour ne pas voir un compte et une etape qui se contredisent.
pub struct JobState {
    pub active: bool,
    pub done: usize,
    pub total: usize,
    pub step: String,
}

impl Job {
    /// Ouvre le chantier. Un total de zero veut dire « en cours, sans compte » : c'est le cas
    /// d'une fabrication, dont on ne connait pas le nombre d'etapes a l'avance.
    pub fn start(&self, step: &str, total: usize) {
        self.done.store(0, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
        if let Ok(mut guard) = self.log.lock() {
            guard.clear();
        }
        self.set_step(step);
        self.active.store(true, Ordering::Relaxed);
    }

    /// Ce qui se passe maintenant, en une ligne que la fenetre affiche telle quelle.
    pub fn set_step(&self, step: &str) {
        if let Ok(mut guard) = self.step.lock() {
            step.clone_into(&mut guard);
        }
    }

    /// Une ligne de plus au journal, que la fenetre montre au fur et a mesure.
    pub fn log(&self, line: &str) {
        if let Ok(mut guard) = self.log.lock() {
            guard.push(line.to_string());
        }
    }

    pub fn lines(&self) -> Vec<String> {
        self.log.lock().map(|log| log.clone()).unwrap_or_default()
    }

    pub fn advance(&self, done: usize) {
        self.done.store(done, Ordering::Relaxed);
    }

    /// Le journal SURVIT a la fin : la fenetre le montre encore le temps d'afficher le resultat.
    pub fn finish(&self) {
        self.active.store(false, Ordering::Relaxed);
        self.set_step("");
    }

    pub fn read(&self) -> JobState {
        JobState {
            active: self.active.load(Ordering::Relaxed),
            done: self.done.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
            step: self.step.lock().map(|s| s.clone()).unwrap_or_default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_chantier_neuf_est_au_repos() {
        let state = Job::default().read();
        assert!(!state.active);
        assert_eq!((state.done, state.total), (0, 0));
        assert_eq!(state.step, "");
    }

    #[test]
    fn ouvrir_un_chantier_efface_le_journal_du_precedent() {
        let job = Job::default();
        job.start("premier", 10);
        job.log("une ligne");
        assert_eq!(job.lines(), vec!["une ligne"]);

        job.start("second", 5);
        assert!(job.lines().is_empty());
        assert_eq!(job.read().total, 5);
    }

    // Le journal survit a la fin : la fenetre affiche encore le compte rendu.
    #[test]
    fn finir_desarme_sans_effacer_le_journal() {
        let job = Job::default();
        job.start("travail", 2);
        job.log("fait");
        job.advance(2);
        job.finish();

        let state = job.read();
        assert!(!state.active);
        assert_eq!(state.done, 2);
        assert_eq!(state.step, "");
        assert_eq!(job.lines(), vec!["fait"]);
    }
}
