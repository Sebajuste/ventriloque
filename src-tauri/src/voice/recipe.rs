// La recette d'une voix : ce qu'on garde de la source, et de combien on la deplace.
//
// C'est CETTE structure qu'on garde a cote du `.wav` produit, et pas seulement le resultat. Sans
// elle, une voix qui plait a quatre-vingt-dix pour cent est intouchable -- on ne peut que la
// refaire de zero.

use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

use super::buffer::Audio;
use super::stretch::Stretch;

/// La crete visee. Les rendus du moteur du mod vont de 0,27 a 0,59 : une reference calee a 0,89
/// laisse de la marge et evite qu'une source enregistree faiblement donne un clone timide.
const PEAK: f32 = 0.89;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Recipe {
    /// D'ou vient la matiere, telle qu'on saura la retrouver.
    pub source: String,
    /// Bornes en secondes ; `None` prend tout.
    pub from: Option<f32>,
    pub to: Option<f32>,
    /// En demi-tons. Zero laisse la voix ou elle est -- mais la fait quand meme passer par le
    /// decaleur, voir `stretch.rs`.
    pub pitch: f32,
    pub formants: f32,
}

/// Applique la recette : extrait, decale, normalise.
pub fn render(audio: &Audio, recipe: &Recipe) -> Result<Audio> {
    let rate = audio.sample_rate as f32;
    let start = (recipe.from.unwrap_or(0.0).max(0.0) * rate) as usize;
    let end = recipe
        .to
        .map(|t| (t.max(0.0) * rate) as usize)
        .unwrap_or(audio.samples.len())
        .min(audio.samples.len());
    if start >= end {
        return Err(anyhow!("l'extrait choisi est vide"));
    }

    let mut stretch = Stretch::new(audio.sample_rate);
    stretch.set_shift(recipe.pitch, recipe.formants);
    let mut samples = stretch.run(&audio.samples[start..end]);

    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 1e-6 {
        let gain = PEAK / peak;
        for sample in &mut samples {
            *sample *= gain;
        }
    }

    Ok(Audio { samples, sample_rate: audio.sample_rate })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(seconds: f32, rate: u32) -> Audio {
        let count = (seconds * rate as f32) as usize;
        Audio {
            samples: (0..count).map(|i| ((i as f32) * 0.05).sin() * 0.2).collect(),
            sample_rate: rate,
        }
    }

    // Le point de la normalisation : une source enregistree faiblement ne doit pas donner un
    // clone timide.
    #[test]
    fn la_sortie_est_calee_sur_la_crete_visee() {
        let made = render(&sine(1.0, 24_000), &Recipe::default()).unwrap();
        let peak = made.samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
        assert!((peak - PEAK).abs() < 0.02, "crete a {peak}");
    }

    #[test]
    fn les_bornes_taillent_l_extrait() {
        let recipe = Recipe { from: Some(1.0), to: Some(2.0), ..Recipe::default() };
        let made = render(&sine(4.0, 24_000), &recipe).unwrap();
        // La queue du decaleur s'ajoute a la seconde demandee : elle porte la fin du mot.
        assert!(made.samples.len() >= 24_000, "{}", made.samples.len());
        assert!(made.samples.len() < 24_000 * 2, "{}", made.samples.len());
    }

    #[test]
    fn un_extrait_vide_est_refuse_plutot_que_rendu_muet() {
        let recipe = Recipe { from: Some(3.0), to: Some(1.0), ..Recipe::default() };
        assert!(render(&sine(4.0, 24_000), &recipe).is_err());
    }
}
