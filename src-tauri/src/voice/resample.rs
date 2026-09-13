// Le meme Lanczos que le moteur, a seize lobes.
//
// Ne sert QUE si l'on assemble des fichiers de frequences differentes -- deux extraits du meme
// jeu n'en ont jamais besoin. Refuser le melange aurait ete plus simple et plus penible ; copier
// un rechantillonneur deja eprouve evite d'en inventer un moins bon.

/// Le nombre de lobes. Celui du moteur : en changer donnerait deux references qui ne se
/// comparent plus.
const LOBES: i64 = 16;

fn sinc(x: f32) -> f32 {
    if x.abs() < 1e-6 { 1.0 } else { (std::f32::consts::PI * x).sin() / (std::f32::consts::PI * x) }
}

fn lanczos(x: f32) -> f32 {
    if x.abs() >= LOBES as f32 { 0.0 } else { sinc(x) * sinc(x / LOBES as f32) }
}

pub fn resample(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }

    let ratio = to as f64 / from as f64;
    let count = (input.len() as f64 * ratio) as usize;
    let mut output = Vec::with_capacity(count);
    for i in 0..count {
        let position = i as f64 / ratio;
        let center = position as i64;
        let fraction = (position - center as f64) as f32;
        let (mut sum, mut weight) = (0.0f32, 0.0f32);
        for k in (-LOBES + 1)..=LOBES {
            let index = center + k;
            if index >= 0 && (index as usize) < input.len() {
                let w = lanczos(k as f32 - fraction);
                sum += input[index as usize] * w;
                weight += w;
            }
        }
        output.push(if weight > 0.0 { sum / weight } else { 0.0 });
    }
    output
}

#[cfg(test)]
mod tests {
    use super::resample;

    #[test]
    fn le_rapport_des_frequences_est_suivi() {
        let input: Vec<f32> = (0..44_100).map(|i| ((i as f32) * 0.01).sin()).collect();
        let output = resample(&input, 44_100, 24_000);
        assert_eq!(output.len(), 24_000);
        assert!(output.iter().any(|s| s.abs() > 0.01));
    }

    // Meme frequence : rien a faire, et surtout pas un passage inutile par l'interpolation.
    #[test]
    fn la_meme_frequence_rend_la_meme_chose() {
        let input: Vec<f32> = (0..1_000).map(|i| ((i as f32) * 0.01).sin()).collect();
        assert_eq!(resample(&input, 24_000, 24_000), input);
    }

    #[test]
    fn un_signal_vide_ne_fait_pas_paniquer() {
        assert!(resample(&[], 44_100, 24_000).is_empty());
    }
}
