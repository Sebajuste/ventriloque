// monter-voix : assemble une reference de clonage a partir de repliques d'un jeu.
//
//   monter-voix <sortie.wav> --liste <fichiers.txt> [--duree 32] [--min 1.2] [--max 6.0]
//   monter-voix <sortie.wav> <fichier.ogg> [...]
//
// Chaque replique est decodee, ramenee en mono, ebarbee de ses silences, puis retenue si sa
// duree tombe dans la fenetre demandee. On empile jusqu'a `--duree` secondes, en alternant les
// scenes pour que la reference ne soit pas quinze fois la meme intonation.
//
// La sortie est un WAV mono 16 bits, a la frequence des sources. Ventriloque reechantillonne
// lui-meme ; ce qui compte ici est la matiere, pas le format.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// Un extrait decode, deja en mono et debarrasse de ses silences de bord.
struct Extrait {
    chemin: PathBuf,
    echantillons: Vec<f32>,
    frequence: u32,
}

impl Extrait {
    fn duree(&self) -> f32 {
        self.echantillons.len() as f32 / self.frequence as f32
    }
}

fn decode(chemin: &Path) -> Result<Extrait, String> {
    let fichier = std::fs::File::open(chemin).map_err(|e| e.to_string())?;
    let flux = MediaSourceStream::new(Box::new(fichier), Default::default());

    let mut indice = Hint::new();
    if let Some(ext) = chemin.extension().and_then(|e| e.to_str()) {
        indice.with_extension(ext);
    }

    let sonde = symphonia::default::get_probe()
        .format(&indice, flux, &FormatOptions::default(), &MetadataOptions::default())
        .map_err(|e| e.to_string())?;
    let mut format = sonde.format;

    let piste = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
        .ok_or("aucune piste audio")?;
    let id_piste = piste.id;

    let mut decodeur = symphonia::default::get_codecs()
        .make(&piste.codec_params, &DecoderOptions::default())
        .map_err(|e| e.to_string())?;

    let mut echantillons: Vec<f32> = Vec::new();
    let mut frequence = 0u32;
    let mut tampon: Option<SampleBuffer<f32>> = None;

    loop {
        let paquet = match format.next_packet() {
            Ok(p) => p,
            // Fin de flux : symphonia la signale par une erreur d'E/S, pas par un Ok vide.
            Err(_) => break,
        };
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
        let canaux = spec.channels.count();

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

    Ok(Extrait { chemin: chemin.to_path_buf(), echantillons, frequence })
}

/// Coupe les silences de bord, puis laisse 30 ms de marge pour ne pas hacher les attaques.
fn ebarbe(mut e: Extrait) -> Extrait {
    let seuil = 0.008; // environ -42 dBFS
    let marge = (e.frequence as f32 * 0.03) as usize;

    let debut = e.echantillons.iter().position(|s| s.abs() > seuil);
    let fin = e.echantillons.iter().rposition(|s| s.abs() > seuil);

    if let (Some(d), Some(f)) = (debut, fin) {
        let d = d.saturating_sub(marge);
        let f = (f + marge).min(e.echantillons.len() - 1);
        e.echantillons = e.echantillons[d..=f].to_vec();
    } else {
        e.echantillons.clear();
    }
    e
}

/// Nom de la scene : `zbriefing_korhal03_kerrigan_016.ogg` -> `zbriefing_korhal03`.
fn scene(chemin: &Path) -> String {
    let radical = chemin.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let morceaux: Vec<&str> = radical.split('_').collect();
    if morceaux.len() >= 3 {
        morceaux[..morceaux.len() - 2].join("_")
    } else {
        radical.to_string()
    }
}

/// Prend un extrait par scene a tour de role, jusqu'a remplir la duree visee. Une reference
/// tiree d'une seule scene ne porte qu'une humeur ; l'alternance en montre plusieurs.
fn choisit(mut extraits: Vec<Extrait>, visee: f32) -> Vec<Extrait> {
    let mut par_scene: HashMap<String, Vec<Extrait>> = HashMap::new();
    for e in extraits.drain(..) {
        par_scene.entry(scene(&e.chemin)).or_default().push(e);
    }

    let mut scenes: Vec<(String, Vec<Extrait>)> = par_scene.into_iter().collect();
    scenes.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, v) in scenes.iter_mut() {
        // Les plus longues d'abord : moins de coupes pour la meme matiere.
        v.sort_by(|a, b| b.duree().partial_cmp(&a.duree()).unwrap());
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

fn ecrit(sortie: &Path, extraits: &[Extrait], frequence: u32) -> Result<f32, String> {
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

    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: frequence,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut ecrivain = hound::WavWriter::create(sortie, spec).map_err(|e| e.to_string())?;
    for s in &piste {
        let v = (s * gain * 32767.0).clamp(-32768.0, 32767.0) as i16;
        ecrivain.write_sample(v).map_err(|e| e.to_string())?;
    }
    ecrivain.finalize().map_err(|e| e.to_string())?;

    Ok(piste.len() as f32 / frequence as f32)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!(
            "monter-voix <sortie.wav> --liste <fichiers.txt> [--duree 32] [--min 1.2] [--max 6.0]\n\
             monter-voix <sortie.wav> <fichier.ogg> [...]"
        );
        std::process::exit(2);
    }

    let sortie = PathBuf::from(&args[0]);
    let mut visee = 32.0f32;
    let mut mini = 1.2f32;
    let mut maxi = 6.0f32;
    let mut entrees: Vec<PathBuf> = Vec::new();

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--duree" | "--min" | "--max" | "--liste" if i + 1 < args.len() => {
                let valeur = args[i + 1].clone();
                match args[i].as_str() {
                    "--duree" => visee = valeur.parse().unwrap_or(visee),
                    "--min" => mini = valeur.parse().unwrap_or(mini),
                    "--max" => maxi = valeur.parse().unwrap_or(maxi),
                    _ => match std::fs::read_to_string(&valeur) {
                        Ok(contenu) => entrees.extend(
                            contenu.lines().map(str::trim).filter(|l| !l.is_empty()).map(PathBuf::from),
                        ),
                        Err(e) => {
                            eprintln!("liste illisible ({valeur}) : {e}");
                            std::process::exit(1);
                        }
                    },
                }
                i += 2;
            }
            autre => {
                entrees.push(PathBuf::from(autre));
                i += 1;
            }
        }
    }

    if entrees.is_empty() {
        eprintln!("aucune entree");
        std::process::exit(2);
    }

    let mut extraits = Vec::new();
    let mut frequences: HashMap<u32, usize> = HashMap::new();
    let mut refuses = 0;
    for chemin in &entrees {
        match decode(chemin) {
            Ok(e) => {
                let e = ebarbe(e);
                let d = e.duree();
                if d >= mini && d <= maxi {
                    *frequences.entry(e.frequence).or_default() += 1;
                    extraits.push(e);
                } else {
                    refuses += 1;
                }
            }
            Err(msg) => eprintln!("  illisible : {} — {msg}", chemin.display()),
        }
    }

    if extraits.is_empty() {
        eprintln!("aucun extrait ne tombe entre {mini} et {maxi} secondes");
        std::process::exit(1);
    }

    // Melanger deux frequences dans un meme WAV ferait un montage a vitesses differentes :
    // on garde la plus repandue et on ecarte le reste.
    let frequence = *frequences.iter().max_by_key(|(_, n)| **n).map(|(f, _)| f).unwrap();
    let avant = extraits.len();
    extraits.retain(|e| e.frequence == frequence);
    if extraits.len() != avant {
        eprintln!("{} extraits ecartes : frequence autre que {frequence} Hz", avant - extraits.len());
    }

    let retenus = choisit(extraits, visee);
    for e in &retenus {
        eprintln!("  {:5.2}s  {}", e.duree(), e.chemin.file_name().unwrap().to_string_lossy());
    }

    match ecrit(&sortie, &retenus, frequence) {
        Ok(duree) => {
            println!(
                "{} — {:.1}s, {} repliques, {} Hz mono ({refuses} hors fenetre)",
                sortie.display(),
                duree,
                retenus.len(),
                frequence
            );
        }
        Err(msg) => {
            eprintln!("ecriture impossible : {msg}");
            std::process::exit(1);
        }
    }
}
