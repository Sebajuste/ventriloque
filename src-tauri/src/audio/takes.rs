// Les prises deja entendues, gardees pour etre redites telles quelles.
//
// REDIRE N'EST PAS REFAIRE. Le moteur tire au sort a chaque synthese : redemander la meme
// replique donne une autre intonation, et fait attendre le calcul avant le premier mot. En
// seance, « redis-le » veut dire LE MEME son, tout de suite.
//
// UNE PRISE SAIT CE QU'ELLE DIT. Le numero seul ne suffit pas : un numero mal rapporte par la
// fenetre ferait sortir la voix d'une autre replique. On ne rend donc une prise qu'a qui la
// demande avec la meme voix, le meme texte et le meme debit -- sinon rien, et la replique se
// refait. Se tromper de son devient impossible, au pire on recalcule.
//
// LE STOCK EST BORNE PAR LA DUREE, PAS PAR LE NOMBRE. Une replique fait de une seconde a une
// minute ; compter les prises laisserait la memoire au hasard de leur longueur. Les plus
// anciennes partent en premier : c'est ce qui vient de passer qu'on redit.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::engine::SAMPLE_RATE;

/// Dix minutes de son a 24 kHz en flottants : ~58 Mo, de quoi couvrir une scene entiere.
const BUDGET: usize = SAMPLE_RATE as usize * 600;

/// Ce qu'une prise dit, et avec quelle voix. C'est la cle qui garde chaque son a sa replique.
#[derive(Clone, PartialEq, Debug)]
pub struct Said {
    pub reference: String,
    pub text: String,
    pub pace: u32,
}

struct Take {
    id: u32,
    said: Said,
    samples: Arc<[f32]>,
}

pub struct Takes {
    kept: VecDeque<Take>,
    held: usize,
    budget: usize,
    next: u32,
}

impl Default for Takes {
    fn default() -> Self {
        Self::with_budget(BUDGET)
    }
}

impl Takes {
    fn with_budget(budget: usize) -> Self {
        Self { kept: VecDeque::new(), held: 0, budget, next: 0 }
    }

    /// Range une prise complete et rend de quoi la retrouver.
    pub fn keep(&mut self, said: Said, samples: Vec<f32>) -> u32 {
        let id = self.next;
        self.next = self.next.wrapping_add(1);
        self.held += samples.len();
        self.kept.push_back(Take { id, said, samples: samples.into() });
        // La plus recente reste toujours, meme seule au-dela du budget : c'est celle qu'on
        // s'apprete a redire.
        while self.held > self.budget && self.kept.len() > 1 {
            if let Some(old) = self.kept.pop_front() {
                self.held -= old.samples.len();
            }
        }
        id
    }

    /// La prise, si elle est encore la ET qu'elle dit bien ce qu'on attend d'elle.
    pub fn get(&self, id: u32, said: &Said) -> Option<Arc<[f32]>> {
        self.kept.iter().find(|t| t.id == id && t.said == *said).map(|t| t.samples.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::{Said, Takes};

    fn said(text: &str) -> Said {
        Said { reference: "judy.wav".into(), text: text.into(), pace: 100 }
    }

    #[test]
    fn rend_la_prise_rangee() {
        let mut takes = Takes::default();
        let id = takes.keep(said("Un."), vec![0.1, 0.2]);
        assert_eq!(&*takes.get(id, &said("Un.")).unwrap(), &[0.1, 0.2]);
    }

    #[test]
    fn ne_rend_jamais_le_son_d_une_autre_replique() {
        let mut takes = Takes::default();
        let un = takes.keep(said("Un."), vec![0.1]);
        takes.keep(said("Deux."), vec![0.2]);
        assert!(takes.get(un, &said("Deux.")).is_none());
        let autre_voix = Said { reference: "nova.wav".into(), ..said("Un.") };
        assert!(takes.get(un, &autre_voix).is_none());
        let autre_debit = Said { pace: 80, ..said("Un.") };
        assert!(takes.get(un, &autre_debit).is_none());
    }

    #[test]
    fn oublie_les_plus_anciennes_au_dela_du_budget() {
        let mut takes = Takes::with_budget(5);
        let first = takes.keep(said("Un."), vec![0.0; 3]);
        let second = takes.keep(said("Deux."), vec![0.0; 3]);
        assert!(takes.get(first, &said("Un.")).is_none());
        assert!(takes.get(second, &said("Deux.")).is_some());
    }

    #[test]
    fn garde_la_derniere_meme_trop_longue() {
        let mut takes = Takes::with_budget(2);
        let id = takes.keep(said("Un."), vec![0.0; 10]);
        assert!(takes.get(id, &said("Un.")).is_some());
    }

    #[test]
    fn une_prise_inconnue_ne_rend_rien() {
        assert!(Takes::default().get(42, &said("Un.")).is_none());
    }
}
