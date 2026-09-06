// assemble-voice : assemble une reference de clonage a partir de repliques deja sur le disque.
//
//   assemble-voice <sortie.wav> --list <fichiers.txt> [--target 32] [--min 1.2] [--max 6.0]
//   assemble-voice <sortie.wav> <fichier.ogg> [...]
//
// CE BINAIRE NE FAIT PLUS LE MONTAGE, il le commande. Decodage, ebarbage, choix des prises et
// ecriture vivent dans `voice-assembly`, partages avec `pack-builder` : c'etaient trois cents
// lignes ecrites deux fois, et elles avaient deja diverge.
//
// Ce qui reste ici est ce que la ligne de commande est seule a savoir faire : lire des chemins,
// charger les fichiers, et raconter ce qui s'est passe.

use std::path::{Path, PathBuf};

use voice_assembly::{Settings, assemble};

const USAGE: &str = "assemble-voice <sortie.wav> --list <fichiers.txt> [--target 32] [--min 1.2] [--max 6.0]\n\
                     assemble-voice <sortie.wav> <fichier.ogg> [...]";

#[derive(Debug)]
struct Arguments {
    output: PathBuf,
    inputs: Vec<PathBuf>,
    settings: Settings,
}

/// Les bornes de la ligne de commande sont plus larges que celles d'une recette : on fouille ici
/// a la main, dans un jeu dont on ne connait pas encore le decoupage.
fn default_settings() -> Settings {
    Settings { target_secs: 32.0, min_secs: 1.2, max_secs: 6.0 }
}

fn parse_arguments(raw: Vec<String>) -> Result<Arguments, String> {
    let mut args = raw.into_iter();
    let output = args.next().ok_or(USAGE)?;
    let mut parsed =
        Arguments { output: PathBuf::from(output), inputs: Vec::new(), settings: default_settings() };

    let rest: Vec<String> = args.collect();
    let mut i = 0;
    while i < rest.len() {
        let flag = rest[i].clone();
        let value = || -> Result<String, String> {
            rest.get(i + 1).cloned().ok_or_else(|| format!("{flag} attend une valeur"))
        };
        match rest[i].as_str() {
            "--target" | "--duree" => {
                parsed.settings.target_secs = number(&value()?, "--target")?;
                i += 2;
            }
            "--min" => {
                parsed.settings.min_secs = number(&value()?, "--min")?;
                i += 2;
            }
            "--max" => {
                parsed.settings.max_secs = number(&value()?, "--max")?;
                i += 2;
            }
            // Une liste de chemins, un par ligne : la ligne de commande de Windows ne tient pas
            // trois cents fichiers.
            "--list" | "--liste" => {
                let path = value()?;
                let text = std::fs::read_to_string(&path)
                    .map_err(|e| format!("liste illisible ({path}) : {e}"))?;
                parsed.inputs.extend(
                    text.lines().map(str::trim).filter(|l| !l.is_empty()).map(PathBuf::from),
                );
                i += 2;
            }
            other => {
                parsed.inputs.push(PathBuf::from(other));
                i += 1;
            }
        }
    }

    if parsed.inputs.is_empty() {
        return Err("aucune entree".into());
    }
    Ok(parsed)
}

fn number(raw: &str, flag: &str) -> Result<f32, String> {
    raw.parse().map_err(|_| format!("{flag} attend un nombre, pas « {raw} »"))
}

/// Charge les fichiers en memoire, sous le nom que le montage utilisera pour grouper les scenes.
fn load(inputs: &[PathBuf]) -> Vec<(String, Vec<u8>)> {
    let mut sources = Vec::new();
    for path in inputs {
        match std::fs::read(path) {
            Ok(data) => sources.push((name_of(path), data)),
            Err(e) => eprintln!("  illisible : {} — {e}", path.display()),
        }
    }
    sources
}

fn name_of(path: &Path) -> String {
    path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
}

fn run() -> Result<(), String> {
    let args = parse_arguments(std::env::args().skip(1).collect())?;
    let sources = load(&args.inputs);
    if sources.is_empty() {
        return Err("aucun fichier lisible".into());
    }

    let made = assemble(sources, &args.output, &args.settings)?;
    for name in &made.kept {
        eprintln!("  {name}");
    }
    println!(
        "{} — {:.1}s, {} repliques, {} Hz mono ({} ecartees)",
        args.output.display(),
        made.seconds,
        made.takes,
        made.sample_rate,
        made.rejected
    );
    Ok(())
}

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(if message == USAGE { 2 } else { 1 });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Result<Arguments, String> {
        parse_arguments(list.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn les_fichiers_se_donnent_en_vrac_apres_la_sortie() {
        let parsed = args(&["voix.wav", "a.ogg", "b.ogg"]).unwrap();
        assert_eq!(parsed.output, PathBuf::from("voix.wav"));
        assert_eq!(parsed.inputs, vec![PathBuf::from("a.ogg"), PathBuf::from("b.ogg")]);
        assert_eq!(parsed.settings.target_secs, 32.0);
    }

    #[test]
    fn les_bornes_se_reglent() {
        let parsed = args(&["voix.wav", "a.ogg", "--target", "20", "--min", "0.5", "--max", "9"])
            .unwrap();
        assert_eq!(parsed.settings.target_secs, 20.0);
        assert_eq!(parsed.settings.min_secs, 0.5);
        assert_eq!(parsed.settings.max_secs, 9.0);
    }

    // Les noms d'avant restent acceptes : des scripts les posent deja.
    #[test]
    fn les_anciens_drapeaux_repondent_encore() {
        let parsed = args(&["voix.wav", "a.ogg", "--duree", "12"]).unwrap();
        assert_eq!(parsed.settings.target_secs, 12.0);
    }

    #[test]
    fn une_valeur_qui_n_est_pas_un_nombre_se_plaint() {
        let e = args(&["voix.wav", "a.ogg", "--min", "beaucoup"]).unwrap_err();
        assert!(e.contains("--min"), "{e}");
    }

    #[test]
    fn un_drapeau_sans_valeur_se_plaint() {
        assert!(args(&["voix.wav", "a.ogg", "--max"]).is_err());
    }

    #[test]
    fn sans_entree_il_n_y_a_rien_a_monter() {
        assert!(args(&["voix.wav"]).is_err());
        assert!(args(&[]).is_err());
    }

    #[test]
    fn le_nom_de_scene_vient_du_fichier_pas_du_chemin() {
        assert_eq!(name_of(Path::new("D:/jeux/vo/kerrigan_007.ogg")), "kerrigan_007.ogg");
    }
}
