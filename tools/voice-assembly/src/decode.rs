//! Decoder une prise et l'ebarber de ses silences de bord.

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Environ -42 dBFS : en dessous, c'est du souffle de salle, pas de la parole.
const SILENCE_THRESHOLD: f32 = 0.008;

/// La marge laissee de part et d'autre, en secondes : couper au ras hacherait les attaques.
const EDGE_MARGIN: f32 = 0.03;

/// Une prise decodee, deja en mono et debarrassee de ses silences de bord.
pub struct Take {
    /// D'ou elle vient -- entree d'archive ou chemin de fichier. Sert au journal et, surtout, au
    /// regroupement par scene : c'est le nom qui porte la scene et le numero de prise.
    pub name: String,
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}

impl Take {
    pub fn duration_secs(&self) -> f32 {
        self.samples.len() as f32 / self.sample_rate.max(1) as f32
    }
}

/// Decode `data`, replie en mono, ebarbe. `name` sert d'indice de format et d'etiquette.
pub fn decode(name: &str, data: Vec<u8>) -> Result<Take, String> {
    let stream = MediaSourceStream::new(Box::new(std::io::Cursor::new(data)), Default::default());

    let mut hint = Hint::new();
    if let Some(dot) = name.rfind('.') {
        hint.with_extension(&name[dot + 1..]);
    }

    let probed = symphonia::default::get_probe()
        .format(&hint, stream, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| e.to_string())?;
    let mut format = probed.format;

    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("aucune piste audio")?;
    let track_id = track.id;

    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| e.to_string())?;

    let mut samples: Vec<f32> = Vec::new();
    let mut sample_rate = 0u32;
    let mut buffer: Option<SampleBuffer<f32>> = None;

    loop {
        // La fin de flux se signale par une erreur d'E/S, pas par un paquet vide.
        let Ok(packet) = format.next_packet() else { break };
        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e.to_string()),
        };

        let spec = *decoded.spec();
        sample_rate = spec.rate;
        let channels = spec.channels.count().max(1);

        let buffer =
            buffer.get_or_insert_with(|| SampleBuffer::new(decoded.capacity() as u64, spec));
        buffer.copy_interleaved_ref(decoded);

        // Repli mono par moyenne : les VO sont mono ou quasi, la somme suffirait, la moyenne
        // evite d'avoir a rattraper le gain ensuite.
        for frame in buffer.samples().chunks(channels) {
            samples.push(frame.iter().sum::<f32>() / channels as f32);
        }
    }

    if samples.is_empty() || sample_rate == 0 {
        return Err("flux vide".to_string());
    }

    Ok(trim(Take { name: name.to_string(), samples, sample_rate }))
}

/// Coupe les silences de bord, en laissant une marge pour ne pas hacher les attaques.
fn trim(mut take: Take) -> Take {
    let margin = (take.sample_rate as f32 * EDGE_MARGIN) as usize;
    let first = take.samples.iter().position(|s| s.abs() > SILENCE_THRESHOLD);
    let last = take.samples.iter().rposition(|s| s.abs() > SILENCE_THRESHOLD);

    match (first, last) {
        (Some(start), Some(end)) => {
            let start = start.saturating_sub(margin);
            let end = (end + margin).min(take.samples.len() - 1);
            take.samples = take.samples[start..=end].to_vec();
        }
        // Rien au-dessus du seuil : la prise est muette, elle ne vaut rien comme reference.
        _ => take.samples.clear(),
    }
    take
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take(samples: Vec<f32>) -> Take {
        Take { name: "essai.ogg".into(), samples, sample_rate: 1_000 }
    }

    #[test]
    fn les_silences_de_bord_sont_coupes_avec_une_marge() {
        // 100 echantillons muets, 100 de parole, 100 muets. La marge fait 30 echantillons.
        let mut samples = vec![0.0f32; 100];
        samples.extend(vec![0.5f32; 100]);
        samples.extend(vec![0.0f32; 100]);

        assert_eq!(trim(take(samples)).samples.len(), 100 + 2 * 30);
    }

    #[test]
    fn une_prise_muette_ne_garde_rien() {
        assert!(trim(take(vec![0.0; 500])).samples.is_empty());
    }

    // La marge ne doit pas sortir du tampon quand la parole touche un bord.
    #[test]
    fn la_marge_ne_deborde_pas() {
        assert_eq!(trim(take(vec![0.5; 50])).samples.len(), 50);
    }

    #[test]
    fn la_duree_se_lit_en_secondes() {
        assert_eq!(take(vec![0.0; 2_000]).duration_secs(), 2.0);
    }

    #[test]
    fn ce_qui_n_est_pas_du_son_est_refuse() {
        assert!(decode("faux.ogg", b"ceci n'est pas du son".to_vec()).is_err());
    }
}
