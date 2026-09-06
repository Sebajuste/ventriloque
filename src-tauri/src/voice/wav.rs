// Ecrire une reference sur le disque : mono, 16 bits, a la frequence de la source.
//
// 16 BITS ET PAS 32. C'est ce que le moteur attend d'une reference, et deux fois moins de place
// pour un `.wav` qu'on garde a cote des donnees.

use anyhow::{Context, Result};
use std::path::Path;

use super::buffer::Audio;

fn spec(sample_rate: u32) -> hound::WavSpec {
    hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    }
}

pub fn write_wav(audio: &Audio, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let mut writer = hound::WavWriter::create(path, spec(audio.sample_rate))
        .with_context(|| format!("ecriture de {}", path.display()))?;
    for sample in &audio.samples {
        writer.write_sample((sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
    }
    writer.finalize().context("fermeture du wav")?;
    Ok(())
}

/// Un `.wav` de test : une sinusoide, dont le canal droit a la moitie de l'amplitude du gauche.
///
/// La moyenne des deux n'est donc pas le canal gauche, ce qui rend le repli mono verifiable.
/// Vit ici plutot que dans chaque module de test : trois d'entre eux en ont besoin.
#[cfg(test)]
pub fn write_test_wav(path: &Path, channels: u16, sample_rate: u32, frames: usize) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let interleaved = hound::WavSpec { channels, ..spec(sample_rate) };
    let mut writer = hound::WavWriter::create(path, interleaved).unwrap();
    for i in 0..frames {
        let value = ((i as f32) * 0.05).sin() * 0.5;
        for channel in 0..channels {
            let sample = if channel == 0 { value } else { value * 0.5 };
            writer.write_sample((sample * i16::MAX as f32) as i16).unwrap();
        }
    }
    writer.finalize().unwrap();
}

#[cfg(test)]
mod tests {
    use crate::testing::TempDir;
    use crate::voice::buffer::Audio;
    use crate::voice::decode::decode;

    // Ce qui est ecrit doit se relire : c'est le seul controle qui vaille sur un format.
    #[test]
    fn ce_qui_est_ecrit_se_relit() {
        let dir = TempDir::new("wav");
        let path = dir.child("voix.wav");
        let audio = Audio {
            samples: (0..2_400).map(|i| ((i as f32) * 0.05).sin() * 0.5).collect(),
            sample_rate: 24_000,
        };

        super::write_wav(&audio, &path).unwrap();
        let relu = decode(&path).unwrap();

        assert_eq!(relu.sample_rate, 24_000);
        assert_eq!(relu.samples.len(), 2_400);
        // Le passage en 16 bits perd un peu : on compare a la precision du format.
        for (avant, apres) in audio.samples.iter().zip(&relu.samples) {
            assert!((avant - apres).abs() < 1e-3, "{avant} devenu {apres}");
        }
    }

    #[test]
    fn le_dossier_manquant_est_cree() {
        let dir = TempDir::new("wav-dossier");
        let path = dir.child("pas/encore/la/voix.wav");
        let audio = Audio { samples: vec![0.0; 10], sample_rate: 24_000 };

        super::write_wav(&audio, &path).unwrap();
        assert!(path.is_file());
    }
}
