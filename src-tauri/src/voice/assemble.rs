// Assembler une reference a partir de plusieurs repliques.
//
// C'EST LA FORME NORMALE DE L'ATELIER. Un personnage de jeu ne parle pas trente secondes d'une
// traite : sa voix se recolte en dix ou vingt lignes courtes, et c'est leur mise bout a bout qui
// fait une reference. Trente secondes suffisent -- les references du mod en font 30 a 35 -- et
// au-dela on paie du temps de clonage sans rien gagner.
//
// Un souffle de silence entre les repliques, parce que deux lignes collees donnent une elocution
// qui ne respire pas, et le clone en herite.

use anyhow::{Result, anyhow};
use std::path::PathBuf;

use super::buffer::Audio;
use super::decode::decode;
use super::resample::resample;

/// Le silence entre deux repliques, en secondes.
const BREATH: f32 = 0.18;

pub fn assemble(files: &[PathBuf], max_seconds: f32) -> Result<Audio> {
    let mut pieces = Vec::new();
    let mut failures = Vec::new();
    for file in files {
        match decode(file) {
            Ok(audio) => pieces.push(audio),
            // Un fichier illisible ne condamne pas la recolte : on le nomme a la fin.
            Err(e) => failures.push(format!("{} ({e})", file.display())),
        }
    }
    let Some(first) = pieces.first() else {
        return Err(anyhow!("aucun fichier lisible. {}", failures.join(" ; ")));
    };

    // La frequence du premier fichier fait loi : c'est presque toujours celle de tous.
    let sample_rate = first.sample_rate;
    let breath = vec![0.0f32; (sample_rate as f32 * BREATH) as usize];
    let ceiling = (sample_rate as f32 * max_seconds) as usize;

    let mut samples = Vec::new();
    for piece in &pieces {
        if !samples.is_empty() {
            samples.extend_from_slice(&breath);
        }
        samples.extend(resample(&piece.samples, piece.sample_rate, sample_rate));
        if samples.len() >= ceiling {
            break;
        }
    }
    samples.truncate(ceiling);
    Ok(Audio { samples, sample_rate })
}

#[cfg(test)]
mod tests {
    use crate::testing::TempDir;
    use crate::voice::wav;

    #[test]
    fn assemble_avec_un_souffle_entre_les_repliques() {
        let dir = TempDir::new("assemble");
        let a = dir.child("a.wav");
        let b = dir.child("b.wav");
        wav::write_test_wav(&a, 1, 24_000, 12_000);
        wav::write_test_wav(&b, 1, 24_000, 11_000);

        let made = super::assemble(&[a, b], 32.0).unwrap();

        assert_eq!(made.samples.len(), 12_000 + 4_320 + 11_000);
        assert_eq!(made.sample_rate, 24_000);
    }

    #[test]
    fn le_plafond_borne_la_reference() {
        let dir = TempDir::new("assemble-plafond");
        let long = dir.child("long.wav");
        wav::write_test_wav(&long, 1, 24_000, 24_000 * 60);

        assert_eq!(super::assemble(&[long], 32.0).unwrap().samples.len(), 24_000 * 32);
    }

    #[test]
    fn un_fichier_illisible_ne_condamne_pas_la_recolte() {
        let dir = TempDir::new("assemble-mauvais");
        let good = dir.child("bon.wav");
        let bad = dir.child("mauvais.wav");
        wav::write_test_wav(&good, 1, 24_000, 8_000);
        std::fs::write(&bad, b"ceci n'est pas un wav").unwrap();

        let made = super::assemble(&[bad.clone(), good], 32.0).unwrap();
        assert_eq!(made.samples.len(), 8_000);

        // Mais si RIEN n'est lisible, le message nomme les fautifs.
        let e = super::assemble(&[bad], 32.0).unwrap_err().to_string();
        assert!(e.contains("aucun fichier lisible"), "{e}");
        assert!(e.contains("mauvais.wav"), "{e}");
    }
}
