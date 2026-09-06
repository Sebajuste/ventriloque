//! Assembler une reference de clonage a partir de repliques tirees d'un jeu.
//!
//! Les prises arrivent en memoire, telles que l'archive les rend : aucune n'est ecrite sur le
//! disque avant le `.wav` final. Ce qui evite d'avoir a nettoyer un dossier temporaire, et
//! surtout d'avoir a expliquer pourquoi trois cents `.ogg` d'un jeu commercial trainent quelque
//! part.

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CODEC_TYPE_NULL, DecoderOptions};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Un extrait decode, deja en mono et debarrasse de ses silences de bord.
pub struct Extrait {
    pub nom: String,
    pub echantillons: Vec<f32>,
    pub frequence: u32,
}

impl Extrait {
    pub fn duree(&self) -> f32 {
        self.echantillons.len() as f32 / self.frequence.max(1) as f32
    }
}

/// Reglages du montage. Les valeurs par defaut sont celles qui marchent sur StarCraft II.
pub struct Reglages {
    pub duree: f32,
    pub mini: f32,
    pub maxi: f32,
}

impl Default for Reglages {
    fn default() -> Self {
        Reglages { duree: 32.0, mini: 1.0, maxi: 4.0 }
    }
}

pub fn decoder(nom: &str, donnees: Vec<u8>) -> Result<Extrait, String> {
    let source = std::io::Cursor::new(donnees);
    let flux = MediaSourceStream::new(Box::new(source), Default::default());

    let mut indice = Hint::new();
    if let Some(point) = nom.rfind('.') {
        indice.with_extension(&nom[point + 1..]);
    }

    let sonde = symphonia::default::get_probe()
        .format(&indice, flux, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| e.to_string())?;
    let mut format = sonde.format;

    let piste = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or("aucune piste audio")?;
    let id_piste = piste.id;

    let mut decodeur = symphonia::default::get_codecs()
        .make(&piste.codec_params, &DecoderOptions::default())
        .map_err(|e| e.to_string())?;

    let mut echantillons: Vec<f32> = Vec::new();
    let mut frequence = 0u32;
    let mut tampon: Option<SampleBuffer<f32>> = None;

    loop {
        // La fin de flux se signale par une erreur d'E/S, pas par un paquet vide.
        let Ok(paquet) = format.next_packet() else { break };
        if paquet.track_id() != id_piste {
            continue;
        }

        let decode = match decodeur.decode(&paquet) {
            Ok(d) => d,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(e) => return Err(e.to_string()),
        };

        let spec = *decode.spec();
        frequence = spec.rate;
        let canaux = spec.channels.count().max(1);

        let tampon = tampon.get_or_insert_with(|| SampleBuffer::new(decode.capacity() as u64, spec));
        tampon.copy_interleaved_ref(decode);

        // Repli mono par moyenne : les VO sont mono ou quasi, la somme suffirait, la moyenne
        // evite d'avoir a rattraper le gain ensuite.
        for cadre in tampon.samples().chunks(canaux) {
            echantillons.push(cadre.iter().sum::<f32>() / canaux as f32);
        }
    }

    if echantillons.is_empty() || frequence == 0 {
        return Err("flux vide".to_string());
    }

    Ok(ebarber(Extrait { nom: nom.to_string(), echantillons, frequence }))
}

/// Coupe les silences de bord, en laissant 30 ms de marge pour ne pas hacher les attaques.
fn ebarber(mut e: Extrait) -> Extrait {
    let seuil = 0.008; // environ -42 dBFS
    let marge = (e.frequence as f32 * 0.03) as usize;

    let debut = e.echantillons.iter().position(|s| s.abs() > seuil);
    let fin = e.echantillons.iter().rposition(|s| s.abs() > seuil);

    match (debut, fin) {
        (Some(d), Some(f)) => {
            let d = d.saturating_sub(marge);
            let f = (f + marge).min(e.echantillons.len() - 1);
            e.echantillons = e.echantillons[d..=f].to_vec();
        }
        _ => e.echantillons.clear(),
    }
    e
}

/// Le radical de scene d'un nom de prise : `zbriefing_korhal03_kerrigan_016.ogg` donne
/// `zbriefing_korhal03`. Sert a alterner les situations dans la reference.
fn scene(nom: &str) -> String {
    let fichier = nom.rsplit(['\\', '/']).next().unwrap_or(nom);
    let radical = fichier.split('.').next().unwrap_or(fichier);
    let morceaux: Vec<&str> = radical.split('_').collect();
    if morceaux.len() >= 3 {
        morceaux[..morceaux.len() - 2].join("_")
    } else {
        radical.to_string()
    }
}

/// Prend une prise par scene a tour de role, jusqu'a remplir la duree visee. Une reference tiree
/// d'une seule scene ne porte qu'une humeur ; l'alternance en montre plusieurs.
fn choisir(extraits: Vec<Extrait>, visee: f32) -> Vec<Extrait> {
    let mut scenes: Vec<(String, Vec<Extrait>)> = Vec::new();
    for e in extraits {
        let cle = scene(&e.nom);
        match scenes.iter_mut().find(|(c, _)| *c == cle) {
            Some((_, v)) => v.push(e),
            None => scenes.push((cle, vec![e])),
        }
    }
    scenes.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, v) in scenes.iter_mut() {
        // Les plus courtes d'abord : dix repliques breves montrent plus de manieres de dire
        // qu'un monologue de la meme duree.
        v.sort_by(|a, b| a.duree().partial_cmp(&b.duree()).unwrap_or(std::cmp::Ordering::Equal));
    }

    let mut retenus = Vec::new();
    let mut total = 0.0;
    while total < visee {
        let mut pris = false;
        for (_, v) in scenes.iter_mut() {
            if v.is_empty() {
                continue;
            }
            pris = true;
            let e = v.remove(0);
            total += e.duree();
            retenus.push(e);
            if total >= visee {
                break;
            }
        }
        if !pris {
            break;
        }
    }
    retenus
}

/// Le compte-rendu d'un montage, tel qu'il remonte au script puis au journal.
pub struct Monte {
    pub secondes: f32,
    pub prises: usize,
    pub frequence: u32,
    pub ecartes: usize,
}

/// Decode, choisit, assemble et ecrit la reference.
pub fn monter(
    sources: Vec<(String, Vec<u8>)>,
    sortie: &std::path::Path,
    reglages: &Reglages,
) -> Result<Monte, String> {
    let mut extraits = Vec::new();
    let mut ecartes = 0usize;
    for (nom, donnees) in sources {
        match decoder(&nom, donnees) {
            Ok(e) => {
                let d = e.duree();
                if d >= reglages.mini && d <= reglages.maxi {
                    extraits.push(e);
                } else {
                    ecartes += 1;
                }
            }
            Err(_) => ecartes += 1,
        }
    }
    if extraits.is_empty() {
        return Err(format!(
            "aucune prise entre {} et {} secondes ({ecartes} ecartees)",
            reglages.mini, reglages.maxi
        ));
    }

    // Melanger deux frequences dans un meme WAV ferait un montage a vitesses differentes : on
    // garde la plus repandue et on ecarte le reste.
    let frequence = frequence_dominante(&extraits);
    let avant = extraits.len();
    extraits.retain(|e| e.frequence == frequence);
    ecartes += avant - extraits.len();

    let retenus = choisir(extraits, reglages.duree);
    let secondes = ecrire(sortie, &retenus, frequence)?;
    Ok(Monte { secondes, prises: retenus.len(), frequence, ecartes })
}

fn frequence_dominante(extraits: &[Extrait]) -> u32 {
    let mut comptes: Vec<(u32, usize)> = Vec::new();
    for e in extraits {
        match comptes.iter_mut().find(|(f, _)| *f == e.frequence) {
            Some((_, n)) => *n += 1,
            None => comptes.push((e.frequence, 1)),
        }
    }
    comptes.iter().max_by_key(|(_, n)| *n).map(|(f, _)| *f).unwrap_or(44_100)
}

fn ecrire(sortie: &std::path::Path, extraits: &[Extrait], frequence: u32) -> Result<f32, String> {
    let silence = vec![0.0f32; (frequence as f32 * 0.25) as usize];
    let mut piste: Vec<f32> = Vec::new();
    for (i, e) in extraits.iter().enumerate() {
        if i > 0 {
            piste.extend_from_slice(&silence);
        }
        piste.extend_from_slice(&e.echantillons);
    }

    let crete = piste.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let gain = if crete > 0.0 { 0.89 / crete } else { 1.0 };

    if let Some(parent) = sortie.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: frequence,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut ecrivain = hound::WavWriter::create(sortie, spec).map_err(|e| e.to_string())?;
    for s in &piste {
        ecrivain
            .write_sample((s * gain * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .map_err(|e| e.to_string())?;
    }
    ecrivain.finalize().map_err(|e| e.to_string())?;

    Ok(piste.len() as f32 / frequence as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn la_scene_retire_locuteur_et_numero() {
        assert_eq!(scene("zbriefing_korhal03_kerrigan_016.ogg"), "zbriefing_korhal03");
        assert_eq!(scene("chemin\\vers\\acresponses_artanis_020.ogg"), "acresponses");
        assert_eq!(scene("court.ogg"), "court");
    }

    #[test]
    fn le_choix_alterne_les_scenes() {
        let faux = |nom: &str, secondes: f32| Extrait {
            nom: nom.to_string(),
            echantillons: vec![0.1; (secondes * 100.0) as usize],
            frequence: 100,
        };
        let retenus = choisir(
            vec![
                faux("a_x_001.ogg", 1.0),
                faux("a_x_002.ogg", 1.0),
                faux("b_x_001.ogg", 1.0),
            ],
            2.0,
        );
        // Une prise de chaque scene avant d'en reprendre une seconde dans la premiere.
        assert_eq!(retenus.len(), 2);
        assert_eq!(scene(&retenus[0].nom), "a");
        assert_eq!(scene(&retenus[1].nom), "b");
    }
}
