// CE QU'ON SAIT D'UNE REPLIQUE PENDANT QU'ELLE JOUE.
//
// Deux comptes, et ils ne mesurent pas la meme chose. `produced` est ce que la synthese a
// fabrique ; `played` est ce qui est REELLEMENT passe dans le haut-parleur. Le moteur fabrique
// environ trois fois plus vite qu'on n'ecoute, donc les deux ne se rejoignent qu'a la fin.
//
// C'est ce qui permet de dire la verite plutot que de l'estimer : `played` a zero veut dire que
// rien n'a encore ete entendu -- la voix se prepare -- et non « il s'est ecoule moins de
// 250 ms ». La duree totale, elle, n'est connue qu'une fois `finished` pose : avant, `produced`
// grandit encore, et un pourcentage calcule dessus reculerait a chaque morceau qui arrive.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Default)]
pub struct PlaybackProgress {
    produced: AtomicUsize,
    played: AtomicUsize,
    finished: AtomicBool,
}

/// Echantillons sortis, echantillons fabriques, et si la fabrication est finie.
pub struct Counts {
    pub played: usize,
    pub produced: usize,
    pub finished: bool,
}

impl PlaybackProgress {
    /// Compte des echantillons rendus par la synthese, avant qu'ils partent vers la sortie.
    pub fn add_produced(&self, count: usize) {
        self.produced.fetch_add(count, Ordering::Relaxed);
    }

    /// Compte un echantillon REELLEMENT sorti. Appele depuis le rappel audio du peripherique :
    /// rien de plus couteux qu'un `fetch_add` n'a sa place ici.
    pub fn add_played(&self, count: usize) {
        self.played.fetch_add(count, Ordering::Relaxed);
    }

    /// La synthese a fini : la duree totale est desormais connue.
    pub fn finish(&self) {
        self.finished.store(true, Ordering::Relaxed);
    }

    pub fn read(&self) -> Counts {
        Counts {
            played: self.played.load(Ordering::Relaxed),
            produced: self.produced.load(Ordering::Relaxed),
            finished: self.finished.load(Ordering::Relaxed),
        }
    }
}
