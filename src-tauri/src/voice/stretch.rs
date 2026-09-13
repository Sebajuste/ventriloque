// Decaler la hauteur et les formants d'un enregistrement, separement.
//
// C'EST LE LEVIER. PocketTTS n'a aucun parametre de timbre : sa seule entree est un son de
// reference. Inventer une voix, c'est donc fabriquer une reference qui n'existe pas, et le
// moyen le plus rentable est de prendre une vraie voix et d'en deplacer les formants sans
// toucher a la hauteur -- ce qui change l'identite percue en laissant l'accent et la diction
// intacts.
//
// Les deux reglages sont independants et c'est tout l'interet :
//
//   hauteur seule    la meme personne qui parle plus grave -- on la reconnait encore
//   formants seuls   un autre gabarit de gorge, meme melodie -- on ne la reconnait plus
//   les deux         une autre personne
//
// `compensate_pitch` a false : sinon la bibliotheque annule le decalage de formants induit par
// le decalage de hauteur, et les deux curseurs cessent d'etre independants.

use std::os::raw::{c_float, c_int};

enum Handle {}

unsafe extern "C" {
    fn signalsmith_stretch_create_preset_default(
        channel_count: c_int,
        sample_rate: c_float,
    ) -> *mut Handle;
    fn signalsmith_stretch_destroy(handle: *mut Handle);
    fn signalsmith_stretch_reset(handle: *mut Handle);
    fn signalsmith_stretch_output_latency(handle: *mut Handle) -> usize;
    fn signalsmith_stretch_input_latency(handle: *mut Handle) -> usize;
    fn signalsmith_stretch_process(
        handle: *mut Handle,
        input: *mut c_float,
        input_length: usize,
        output: *mut c_float,
        output_length: usize,
    );
    fn signalsmith_stretch_set_transpose_factor_semitones(
        handle: *mut Handle,
        semitones: c_float,
        tonality_limit: c_float,
    );
    fn signalsmith_stretch_set_formant_factor_semitones(
        handle: *mut Handle,
        semitones: c_float,
        compensate_pitch: c_int,
    );
    fn signalsmith_stretch_set_formant_base(handle: *mut Handle, frequency: c_float);
    fn signalsmith_stretch_exact(
        handle: *mut Handle,
        input: *mut c_float,
        input_length: usize,
        output: *mut c_float,
        output_length: usize,
    ) -> bool;
    fn signalsmith_stretch_flush(handle: *mut Handle, output: *mut c_float, output_length: usize);
}

pub struct Stretch {
    handle: *mut Handle,
}

impl Drop for Stretch {
    fn drop(&mut self) {
        unsafe { signalsmith_stretch_destroy(self.handle) }
    }
}

impl Stretch {
    pub fn new(sample_rate: u32) -> Self {
        let handle =
            unsafe { signalsmith_stretch_create_preset_default(1, sample_rate as c_float) };
        assert!(!handle.is_null(), "signalsmith-stretch n'a pas pu s'initialiser");
        Self { handle }
    }

    /// La queue que `flush` rendra : le decaleur regarde en avant, donc les derniers echantillons
    /// sortent apres que l'entree est epuisee. Les jeter couperait la fin du mot.
    fn output_latency(&self) -> usize {
        unsafe { signalsmith_stretch_output_latency(self.handle) }
    }

    /// Zero demi-ton ne veut pas dire « ne rien faire » : le decaleur reste dans le chemin et
    /// colore legerement. Le rendu doit donc traverser la meme chaine que le reglage soit neutre
    /// ou non, sinon deux references jugees cote a cote ne sont pas comparables.
    pub fn set_shift(&mut self, pitch_semitones: f32, formant_semitones: f32) {
        unsafe {
            // La base AVANT le decalage, et jamais l'inverse : ces deux reglages passent par le
            // meme champ et le dernier ecrit gagne.
            signalsmith_stretch_set_formant_base(self.handle, 0.0);
            signalsmith_stretch_set_transpose_factor_semitones(self.handle, pitch_semitones, 0.0);
            signalsmith_stretch_set_formant_factor_semitones(self.handle, formant_semitones, 0);
            signalsmith_stretch_reset(self.handle);
        }
    }

    /// Mono, tout d'un bloc. Une reference fait trente secondes : il n'y a rien a streamer ici,
    /// et l'appelant est deja hors du fil de l'interface.
    pub fn run(&mut self, input: &[f32]) -> Vec<f32> {
        let tail = self.output_latency();
        let mut body = vec![0.0f32; input.len()];
        let mut source = input.to_vec();
        unsafe {
            signalsmith_stretch_exact(
                self.handle,
                source.as_mut_ptr(),
                source.len(),
                body.as_mut_ptr(),
                body.len(),
            );
        }
        let mut rest = vec![0.0f32; tail];
        unsafe {
            signalsmith_stretch_flush(self.handle, rest.as_mut_ptr(), rest.len());
        }
        body.extend_from_slice(&rest);
        body
    }
}

// CHANGER LE DEBIT D'UN SON QUI ARRIVE PAR MORCEAUX, sans toucher a sa hauteur.
//
// PocketTTS n'a pas de reglage de vitesse -- son serveur accepte un champ `speed` et l'ignore.
// Le debit se regle donc a la sortie : chaque morceau recu est etire ou tasse au fil de l'eau,
// et le premier mot sort toujours des le premier morceau.
//
// La longueur de sortie se calcule sur le CUMUL, pas morceau par morceau : arrondir chaque
// morceau ferait deriver la duree de quelques echantillons a chaque fois.
pub struct Pacer {
    stretch: Stretch,
    speed: f64,
    owed: f64,
}

impl Pacer {
    /// `pace` en pourcent : 85 parle plus lentement, 120 plus vite.
    pub fn new(sample_rate: u32, pace: u32) -> Self {
        Self { stretch: Stretch::new(sample_rate), speed: pace.max(1) as f64 / 100.0, owed: 0.0 }
    }

    pub fn push(&mut self, input: &[f32]) -> Vec<f32> {
        self.owed += input.len() as f64 / self.speed;
        let length = self.owed.floor() as usize;
        self.owed -= length as f64;
        let mut source = input.to_vec();
        let mut output = vec![0.0f32; length];
        unsafe {
            signalsmith_stretch_process(
                self.stretch.handle,
                source.as_mut_ptr(),
                source.len(),
                output.as_mut_ptr(),
                output.len(),
            );
        }
        output
    }

    /// La fin de la replique. L'etireur retient une fenetre d'entree : sans ce silence pousse
    /// derriere et la queue videe, la derniere syllabe resterait dedans.
    pub fn finish(&mut self) -> Vec<f32> {
        let held = unsafe { signalsmith_stretch_input_latency(self.stretch.handle) };
        let mut output = self.push(&vec![0.0f32; held]);
        let mut rest = vec![0.0f32; self.stretch.output_latency()];
        unsafe {
            signalsmith_stretch_flush(self.stretch.handle, rest.as_mut_ptr(), rest.len());
        }
        output.extend_from_slice(&rest);
        output
    }
}

#[cfg(test)]
mod tests {
    use super::Pacer;

    const RATE: u32 = 24_000;

    fn tone(seconds: f32) -> Vec<f32> {
        (0..(RATE as f32 * seconds) as usize)
            .map(|i| (i as f32 * 220.0 * std::f32::consts::TAU / RATE as f32).sin() * 0.5)
            .collect()
    }

    /// Le son passe par morceaux de la taille de ceux du moteur, comme en seance.
    fn paced(pace: u32, input: &[f32]) -> Vec<f32> {
        let mut pacer = Pacer::new(RATE, pace);
        let mut out: Vec<f32> = input.chunks(2048).flat_map(|c| pacer.push(c)).collect();
        out.extend(pacer.finish());
        out
    }

    #[test]
    fn ralentir_allonge_la_replique_d_autant() {
        let input = tone(1.0);
        let out = paced(80, &input);

        // Une seconde a 80 % en dure 1,25 ; la queue de l'etireur s'y ajoute, pas plus.
        assert!(out.len() >= 30_000, "{}", out.len());
        assert!(out.len() < 30_000 + 12_000, "{}", out.len());
        assert!(out.iter().any(|s| s.abs() > 0.1), "le son est reste dans l'etireur");
    }

    #[test]
    fn accelerer_raccourcit_la_replique() {
        let input = tone(1.0);
        let out = paced(125, &input);

        assert!(out.len() >= 19_200, "{}", out.len());
        assert!(out.len() < 19_200 + 12_000, "{}", out.len());
    }
}
