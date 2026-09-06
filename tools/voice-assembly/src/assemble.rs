//! Decoder, choisir, mettre bout a bout, ecrire.

use std::path::Path;

use super::decode::{Take, decode};
use super::select::choose;

/// Le silence entre deux prises, en secondes. Deux repliques collees donnent une elocution qui ne
/// respire pas, et le clone en herite.
const GAP_SECS: f32 = 0.25;

/// La crete visee, comme dans la forge de l'application : de la marge sous la saturation, et
/// assez de niveau pour qu'une source faible ne donne pas un clone timide.
const PEAK: f32 = 0.89;

/// Ce qu'on demande au montage. Les valeurs par defaut sont celles qui marchent sur StarCraft II.
#[derive(Debug, Clone)]
pub struct Settings {
    /// La duree visee de la reference, en secondes.
    pub target_secs: f32,
    /// La fenetre de duree d'une prise retenue : trop courte, elle ne dit rien du timbre ; trop
    /// longue, c'est un monologue qui mange la place de dix repliques.
    pub min_secs: f32,
    pub max_secs: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { target_secs: 32.0, min_secs: 1.0, max_secs: 4.0 }
    }
}

/// Le compte rendu d'un montage, tel qu'il remonte au journal.
#[derive(Debug)]
pub struct Assembled {
    pub seconds: f32,
    pub takes: usize,
    pub sample_rate: u32,
    /// Ce qui n'a pas ete retenu : illisible, muet, hors fenetre, ou d'une autre frequence.
    pub rejected: usize,
    /// Les prises retenues, dans l'ordre du montage. Le journal les nomme.
    pub kept: Vec<String>,
}

/// Decode les sources, choisit, assemble et ecrit la reference en WAV mono 16 bits.
pub fn assemble(
    sources: Vec<(String, Vec<u8>)>,
    output: &Path,
    settings: &Settings,
) -> Result<Assembled, String> {
    let mut takes = Vec::new();
    let mut rejected = 0usize;
    for (name, data) in sources {
        match decode(&name, data) {
            Ok(take) => {
                let seconds = take.duration_secs();
                if seconds >= settings.min_secs && seconds <= settings.max_secs {
                    takes.push(take);
                } else {
                    rejected += 1;
                }
            }
            Err(_) => rejected += 1,
        }
    }
    if takes.is_empty() {
        return Err(format!(
            "aucune prise entre {} et {} secondes ({rejected} ecartees)",
            settings.min_secs, settings.max_secs
        ));
    }

    // Melanger deux frequences dans un meme WAV ferait un montage a vitesses differentes : on
    // garde la plus repandue et on ecarte le reste.
    let sample_rate = dominant_rate(&takes);
    let before = takes.len();
    takes.retain(|t| t.sample_rate == sample_rate);
    rejected += before - takes.len();

    let kept = choose(takes, settings.target_secs);
    let names = kept.iter().map(|t| t.name.clone()).collect();
    let seconds = write_wav(output, &kept, sample_rate)?;
    Ok(Assembled { seconds, takes: kept.len(), sample_rate, rejected, kept: names })
}

fn dominant_rate(takes: &[Take]) -> u32 {
    let mut counts: Vec<(u32, usize)> = Vec::new();
    for take in takes {
        match counts.iter_mut().find(|(rate, _)| *rate == take.sample_rate) {
            Some((_, n)) => *n += 1,
            None => counts.push((take.sample_rate, 1)),
        }
    }
    counts.iter().max_by_key(|(_, n)| *n).map(|(rate, _)| *rate).unwrap_or(44_100)
}

fn write_wav(output: &Path, takes: &[Take], sample_rate: u32) -> Result<f32, String> {
    let gap = vec![0.0f32; (sample_rate as f32 * GAP_SECS) as usize];
    let mut track: Vec<f32> = Vec::new();
    for (i, take) in takes.iter().enumerate() {
        if i > 0 {
            track.extend_from_slice(&gap);
        }
        track.extend_from_slice(&take.samples);
    }

    let peak = track.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let gain = if peak > 0.0 { PEAK / peak } else { 1.0 };

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(output, spec).map_err(|e| e.to_string())?;
    for sample in &track {
        let value = (sample * gain * 32767.0).clamp(-32768.0, 32767.0) as i16;
        writer.write_sample(value).map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())?;

    Ok(track.len() as f32 / sample_rate as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("voice-assembly-{nanos}-{label}"));
        std::fs::create_dir_all(&path).unwrap();
        path
    }

    fn take(name: &str, seconds: f32, sample_rate: u32) -> Take {
        Take {
            name: name.into(),
            samples: vec![0.4; (seconds * sample_rate as f32) as usize],
            sample_rate,
        }
    }

    #[test]
    fn la_frequence_dominante_l_emporte() {
        let takes = vec![
            take("a_x_1.ogg", 1.0, 44_100),
            take("b_x_1.ogg", 1.0, 22_050),
            take("c_x_1.ogg", 1.0, 22_050),
        ];
        assert_eq!(dominant_rate(&takes), 22_050);
    }

    #[test]
    fn sans_prise_on_se_rabat_sur_une_frequence_plausible() {
        assert_eq!(dominant_rate(&[]), 44_100);
    }

    #[test]
    fn le_montage_intercale_un_silence_et_normalise() {
        let dir = temp_dir("write");
        let output = dir.join("voix.wav");

        let seconds = write_wav(
            &output,
            &[take("a_x_1.ogg", 1.0, 1_000), take("b_x_1.ogg", 1.0, 1_000)],
            1_000,
        )
        .unwrap();

        // Deux secondes de prise, plus le silence intercale.
        assert!((seconds - (2.0 + GAP_SECS)).abs() < 0.01, "{seconds}");

        let reader = hound::WavReader::open(&output).unwrap();
        assert_eq!(reader.spec().channels, 1);
        assert_eq!(reader.spec().sample_rate, 1_000);
        let peak = reader
            .into_samples::<i16>()
            .map(|s| (s.unwrap() as f32 / 32767.0).abs())
            .fold(0.0f32, f32::max);
        assert!((peak - PEAK).abs() < 0.01, "crete a {peak}");

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ce_qui_n_est_pas_du_son_est_ecarte_et_compte() {
        let dir = temp_dir("rejet");
        let e = assemble(
            vec![("a.ogg".into(), b"pas du son".to_vec())],
            &dir.join("voix.wav"),
            &Settings::default(),
        )
        .unwrap_err();

        assert!(e.contains("aucune prise"), "{e}");
        assert!(e.contains("1 ecartees"), "{e}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
