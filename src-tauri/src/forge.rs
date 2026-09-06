// Fabriquer une reference de voix : d'un enregistrement quelconque au .wav que le moteur clone.
//
// La chaine est courte et son ordre compte :
//
//   decoder -> mono -> extraire -> decaler (hauteur, formants) -> normaliser -> ecrire
//
// Le decalage vient AVANT la normalisation parce qu'il change les cretes ; normaliser d'abord
// laisserait la reference sous ou au-dessus du niveau vise. Et l'extrait est pris avant le
// decalage : traiter trente minutes pour en garder trente secondes coute trente minutes.

use anyhow::{Context, Result, anyhow};
use rodio::{Decoder, Source};
use std::io::BufReader;
use std::path::Path;

use crate::stretch::Stretch;

pub struct Audio {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

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
// deux a l'UniformSourceIterator de rodio. Il rend zero echantillon sur une source dont la
// longueur de tranche vaut zero tant que le premier paquet n'est pas decode -- ce qui est le cas
// de tous les decodeurs symphonia. La forge refusait donc TOUS les fichiers, avec le message
// « le fichier ne contient aucun son », qui accusait la source.
pub fn decode(path: &Path) -> Result<Audio> {
    let file = std::fs::File::open(path).with_context(|| format!("ouverture de {}", path.display()))?;
    let decodeur = Decoder::new(BufReader::new(file)).context("format audio non reconnu")?;

    let sample_rate = decodeur.sample_rate().get();
    let canaux = decodeur.channels().get() as usize;

    // Le repli se fait au fil du decodage, pour ne jamais tenir l'entrelace entier en memoire --
    // une source peut faire une heure.
    let mut samples = Vec::new();
    let mut trame = Vec::with_capacity(canaux);
    for e in decodeur {
        trame.push(e);
        if trame.len() == canaux {
            samples.push(trame.iter().sum::<f32>() / canaux as f32);
            trame.clear();
        }
    }

    if samples.is_empty() {
        return Err(anyhow!("le fichier ne contient aucun son"));
    }
    Ok(Audio { samples, sample_rate })
}

// Le meme Lanczos que le moteur, a seize lobes.
//
// Ne sert QUE si l'on assemble des fichiers de frequences differentes -- deux extraits du meme
// jeu n'en ont jamais besoin. Refuser le melange aurait ete plus simple et plus penible ; copier
// un rechantillonneur deja eprouve evite d'en inventer un moins bon.
fn rechantillonner(entree: &[f32], src: u32, dst: u32) -> Vec<f32> {
    if src == dst || entree.is_empty() {
        return entree.to_vec();
    }
    const K: i64 = 16;
    let sinc = |x: f32| {
        if x.abs() < 1e-6 {
            1.0
        } else {
            (std::f32::consts::PI * x).sin() / (std::f32::consts::PI * x)
        }
    };
    let lanczos = |x: f32| if x.abs() >= K as f32 { 0.0 } else { sinc(x) * sinc(x / K as f32) };

    let ratio = dst as f64 / src as f64;
    let combien = (entree.len() as f64 * ratio) as usize;
    let mut sortie = Vec::with_capacity(combien);
    for i in 0..combien {
        let position = i as f64 / ratio;
        let centre = position as i64;
        let fraction = (position - centre as f64) as f32;
        let (mut somme, mut poids) = (0.0f32, 0.0f32);
        for k in (-K + 1)..=K {
            let idx = centre + k;
            if idx >= 0 && (idx as usize) < entree.len() {
                let w = lanczos(k as f32 - fraction);
                somme += entree[idx as usize] * w;
                poids += w;
            }
        }
        sortie.push(if poids > 0.0 { somme / poids } else { 0.0 });
    }
    sortie
}

// Assembler une reference a partir de plusieurs repliques.
//
// C'EST LA FORME NORMALE DE L'ATELIER. Un personnage de jeu ne parle pas trente secondes d'une
// traite : sa voix se recolte en dix ou vingt lignes courtes, et c'est leur mise bout a bout qui
// fait une reference. Trente secondes suffisent -- les references du mod en font 30 a 35 -- et
// au-dela on paie du temps de clonage sans rien gagner.
//
// Un souffle de silence entre les repliques, parce que deux lignes collees donnent une elocution
// qui ne respire pas, et le clone en herite.
pub fn assembler(fichiers: &[std::path::PathBuf], max_secondes: f32) -> Result<Audio> {
    let mut morceaux = Vec::new();
    let mut echecs = Vec::new();
    for f in fichiers {
        match decode(f) {
            Ok(a) => morceaux.push(a),
            // Un fichier illisible ne condamne pas la recolte : on le nomme a la fin.
            Err(e) => echecs.push(format!("{} ({e})", f.display())),
        }
    }
    let Some(premier) = morceaux.first() else {
        return Err(anyhow!("aucun fichier lisible. {}", echecs.join(" ; ")));
    };

    // La frequence du premier fichier fait loi : c'est presque toujours celle de tous.
    let rate = premier.sample_rate;
    let souffle = vec![0.0f32; (rate as f32 * 0.18) as usize];
    let plafond = (rate as f32 * max_secondes) as usize;

    let mut samples = Vec::new();
    for a in &morceaux {
        if !samples.is_empty() {
            samples.extend_from_slice(&souffle);
        }
        samples.extend(rechantillonner(&a.samples, a.sample_rate, rate));
        if samples.len() >= plafond {
            break;
        }
    }
    samples.truncate(plafond);
    Ok(Audio { samples, sample_rate: rate })
}

// La recette d'une voix : ce qu'on garde de la source, et de combien on la deplace.
//
// C'est CETTE structure qu'une fiche de PNJ conserve, pas seulement le .wav produit. Sans elle,
// une voix qui plait a quatre-vingt-dix pour cent est intouchable -- on ne peut que la refaire
// de zero.
#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
pub struct Recipe {
    pub source: String,
    // Bornes en secondes ; `None` prend tout.
    pub from: Option<f32>,
    pub to: Option<f32>,
    pub pitch: f32,
    pub formants: f32,
}

impl Default for Recipe {
    fn default() -> Self {
        Self { source: String::new(), from: None, to: None, pitch: 0.0, formants: 0.0 }
    }
}

// La crete visee. Les rendus du moteur du mod vont de 0,27 a 0,59 : une reference calee a 0,89
// laisse de la marge et evite qu'une source enregistree faiblement donne un clone timide.
const PEAK: f32 = 0.89;

pub fn forge(audio: &Audio, recipe: &Recipe) -> Result<Audio> {
    let rate = audio.sample_rate as f32;
    let start = ((recipe.from.unwrap_or(0.0).max(0.0)) * rate) as usize;
    let end = recipe
        .to
        .map(|t| (t.max(0.0) * rate) as usize)
        .unwrap_or(audio.samples.len())
        .min(audio.samples.len());
    if start >= end {
        return Err(anyhow!("l'extrait choisi est vide"));
    }

    let mut stretch = Stretch::new(audio.sample_rate);
    stretch.set(recipe.pitch, recipe.formants);
    let mut samples = stretch.run(&audio.samples[start..end]);

    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 1e-6 {
        let gain = PEAK / peak;
        for s in &mut samples {
            *s *= gain;
        }
    }

    Ok(Audio { samples, sample_rate: audio.sample_rate })
}

pub fn write_wav(audio: &Audio, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: audio.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .with_context(|| format!("ecriture de {}", path.display()))?;
    for s in &audio.samples {
        writer.write_sample((s.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
    }
    writer.finalize().context("fermeture du wav")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wav_temporaire(canaux: u16, rate: u32, trames: usize) -> std::path::PathBuf {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("ventriloque-forge-{n}-{canaux}-{trames}.wav"));
        let spec = hound::WavSpec {
            channels: canaux,
            sample_rate: rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut w = hound::WavWriter::create(&p, spec).unwrap();
        for i in 0..trames {
            let v = ((i as f32) * 0.05).sin() * 0.5;
            for c in 0..canaux {
                // Le canal droit a la moitie de l'amplitude du gauche : la moyenne des deux
                // n'est donc pas le canal gauche, ce qui rend le repli mono verifiable.
                let e = if c == 0 { v } else { v * 0.5 };
                w.write_sample((e * i16::MAX as f32) as i16).unwrap();
            }
        }
        w.finalize().unwrap();
        p
    }

    // LA REGRESSION QUI A FAIT PERDRE UNE SEANCE. L'UniformSourceIterator rendait zero
    // echantillon, et la forge refusait tous les fichiers en accusant la source.
    #[test]
    fn decode_rend_du_son() {
        let p = wav_temporaire(2, 44_100, 5_000);
        let a = decode(&p).unwrap();
        std::fs::remove_file(&p).ok();

        assert_eq!(a.samples.len(), 5_000, "une trame stereo doit donner un echantillon mono");
        assert_eq!(a.sample_rate, 44_100, "la frequence de la source n'est pas touchee");
        assert!(a.samples.iter().any(|s| s.abs() > 0.01), "le son ne doit pas etre muet");
    }

    #[test]
    fn assemble_avec_un_souffle_entre_les_repliques() {
        let a = wav_temporaire(1, 24_000, 12_000);
        let b = wav_temporaire(1, 24_000, 11_000);
        let fait = assembler(&[a.clone(), b.clone()], 32.0).unwrap();
        std::fs::remove_file(&a).ok();
        std::fs::remove_file(&b).ok();

        assert_eq!(fait.samples.len(), 12_000 + 4_320 + 11_000);
        assert_eq!(fait.sample_rate, 24_000);
    }

    #[test]
    fn le_plafond_borne_la_reference() {
        let a = wav_temporaire(1, 24_000, 24_000 * 60);
        let fait = assembler(&[a.clone()], 32.0).unwrap();
        std::fs::remove_file(&a).ok();
        assert_eq!(fait.samples.len(), 24_000 * 32);
    }

    #[test]
    fn un_fichier_illisible_ne_condamne_pas_la_recolte() {
        let bon = wav_temporaire(1, 24_000, 8_000);
        let mauvais = std::env::temp_dir().join("ventriloque-pas-du-son.wav");
        std::fs::write(&mauvais, b"ceci n'est pas un wav").unwrap();

        let fait = assembler(&[mauvais.clone(), bon.clone()], 32.0).unwrap();
        std::fs::remove_file(&bon).ok();
        std::fs::remove_file(&mauvais).ok();
        assert_eq!(fait.samples.len(), 8_000);

        // Mais si RIEN n'est lisible, le message nomme les fautifs.
        let seul = std::env::temp_dir().join("ventriloque-pas-du-son-2.wav");
        std::fs::write(&seul, b"non plus").unwrap();
        let rate = assembler(&[seul.clone()], 32.0).err().map(|e| e.to_string());
        std::fs::remove_file(&seul).ok();
        let e = rate.expect("un fichier seul et illisible doit faire echouer la recolte");
        assert!(e.contains("aucun fichier lisible"), "{e}");
    }

    #[test]
    fn rechantillonner_suit_le_rapport_des_frequences() {
        let entree: Vec<f32> = (0..44_100).map(|i| ((i as f32) * 0.01).sin()).collect();
        let sortie = rechantillonner(&entree, 44_100, 24_000);
        assert_eq!(sortie.len(), 24_000);
        assert!(sortie.iter().any(|s| s.abs() > 0.01));
        assert_eq!(rechantillonner(&entree, 24_000, 24_000).len(), entree.len());
    }
}
