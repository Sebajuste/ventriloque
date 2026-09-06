// Lire un fichier son quelconque et le ramener en mono.
//
// LA FREQUENCE N'EST PAS TOUCHEE, et c'est le moteur qui a raison.
//
// PocketTTS ramene lui-meme toute reference en mono 24 kHz, avec un Lanczos a seize lobes, avant
// de l'encoder -- puis la normalise. Rechantillonner ici ferait passer le son par deux
// interpolations au lieu d'une, pour arriver au meme endroit en moins bon.
//
// Le repli en mono, lui, reste de notre ressort : le decaleur de hauteur et de formants ne
// travaille que sur un canal.
//
// CE QUI S'EST CASSE ICI, et pourquoi le test plus bas existe. Une premiere version confiait les
// deux a l'`UniformSourceIterator` de rodio. Il rend zero echantillon sur une source dont la
// longueur de tranche vaut zero tant que le premier paquet n'est pas decode -- ce qui est le cas
// de tous les decodeurs symphonia. La forge refusait donc TOUS les fichiers, avec le message
// « le fichier ne contient aucun son », qui accusait la source.

use anyhow::{Context, Result, anyhow};
use rodio::{Decoder, Source};
use std::io::BufReader;
use std::path::Path;

use super::buffer::Audio;

pub fn decode(path: &Path) -> Result<Audio> {
    let file =
        std::fs::File::open(path).with_context(|| format!("ouverture de {}", path.display()))?;
    let decoder = Decoder::new(BufReader::new(file)).context("format audio non reconnu")?;

    let sample_rate = decoder.sample_rate().get();
    let channels = decoder.channels().get() as usize;

    // Le repli se fait au fil du decodage, pour ne jamais tenir l'entrelace entier en memoire --
    // une source peut faire une heure.
    let mut samples = Vec::new();
    let mut frame = Vec::with_capacity(channels);
    for sample in decoder {
        frame.push(sample);
        if frame.len() == channels {
            samples.push(frame.iter().sum::<f32>() / channels as f32);
            frame.clear();
        }
    }

    if samples.is_empty() {
        return Err(anyhow!("le fichier ne contient aucun son"));
    }
    Ok(Audio { samples, sample_rate })
}

#[cfg(test)]
mod tests {
    use crate::testing::TempDir;
    use crate::voice::wav;

    // LA REGRESSION QUI A FAIT PERDRE UNE SEANCE. L'`UniformSourceIterator` rendait zero
    // echantillon, et la forge refusait tous les fichiers en accusant la source.
    #[test]
    fn decode_rend_du_son() {
        let dir = TempDir::new("decode");
        let path = dir.child("stereo.wav");
        wav::write_test_wav(&path, 2, 44_100, 5_000);

        let audio = super::decode(&path).unwrap();

        assert_eq!(audio.samples.len(), 5_000, "une trame stereo doit donner un echantillon mono");
        assert_eq!(audio.sample_rate, 44_100, "la frequence de la source n'est pas touchee");
        assert!(audio.samples.iter().any(|s| s.abs() > 0.01), "le son ne doit pas etre muet");
    }

    #[test]
    fn ce_qui_n_est_pas_du_son_est_refuse_avec_un_message() {
        let dir = TempDir::new("decode-mauvais");
        let path = dir.child("faux.wav");
        std::fs::write(&path, b"ceci n'est pas un wav").unwrap();

        assert!(super::decode(&path).is_err());
    }
}
