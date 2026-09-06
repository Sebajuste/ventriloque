//! Ce que la ligne de commande accepte.
//!
//! LES NOMS FRANCAIS REPONDENT ENCORE. L'application deployee sur une machine appelle ce binaire
//! avec les drapeaux qu'elle connaissait au moment de sa compilation : les deux se rebatissent
//! separement, et l'un peut avoir un tour d'avance sur l'autre.

use std::path::PathBuf;

pub const USAGE: &str = "\
pack-builder --script <recette.rhai> --game <racine du jeu> --out <dossier de travail>
pack-builder --list <motif> --game <racine du jeu>     pour fouiller a la main
pack-builder --hash <chemin depot>                     pour ecrire une recette Cyberpunk";

#[derive(Debug, Default, PartialEq)]
pub struct Arguments {
    pub script: Option<PathBuf>,
    pub game: PathBuf,
    pub out_dir: PathBuf,
    pub list: Option<String>,
    pub hash: Option<String>,
}

pub fn parse(raw: Vec<String>) -> Result<Arguments, String> {
    let mut args = Arguments::default();

    let mut i = 0;
    while i < raw.len() {
        let flag = raw[i].clone();
        let value = || -> Result<String, String> {
            raw.get(i + 1).cloned().ok_or_else(|| format!("{flag} attend une valeur"))
        };
        match raw[i].as_str() {
            "--script" => args.script = Some(PathBuf::from(value()?)),
            "--game" | "--jeu" => args.game = PathBuf::from(value()?),
            "--out" | "--sortie" => args.out_dir = PathBuf::from(value()?),
            "--list" | "--lister" => args.list = Some(value()?),
            "--hash" | "--hacher" => args.hash = Some(value()?),
            other => return Err(format!("argument inconnu : {other}\n\n{USAGE}")),
        }
        i += 2;
    }

    // `--hash` est du calcul pur : il ne lit ni jeu ni script.
    if args.hash.is_some() {
        return Ok(args);
    }
    if args.game.as_os_str().is_empty() {
        return Err("--game est obligatoire".into());
    }
    if args.list.is_none() {
        if args.script.is_none() {
            return Err("--script est obligatoire (ou --list pour fouiller)".into());
        }
        if args.out_dir.as_os_str().is_empty() {
            return Err("--out est obligatoire".into());
        }
    }
    Ok(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_of(list: &[&str]) -> Result<Arguments, String> {
        parse(list.iter().map(|s| s.to_string()).collect())
    }

    #[test]
    fn une_fabrication_demande_les_trois_drapeaux() {
        let args = parse_of(&["--script", "sc2.rhai", "--game", "D:/Jeux/SC2", "--out", "D:/tmp"])
            .unwrap();
        assert_eq!(args.script, Some(PathBuf::from("sc2.rhai")));
        assert_eq!(args.game, PathBuf::from("D:/Jeux/SC2"));
        assert_eq!(args.out_dir, PathBuf::from("D:/tmp"));
    }

    // L'application deployee peut avoir un tour de retard : elle appelle encore en francais.
    #[test]
    fn les_anciens_drapeaux_repondent_encore() {
        let old = parse_of(&["--script", "sc2.rhai", "--jeu", "D:/Jeux/SC2", "--sortie", "D:/tmp"])
            .unwrap();
        let new = parse_of(&["--script", "sc2.rhai", "--game", "D:/Jeux/SC2", "--out", "D:/tmp"])
            .unwrap();
        assert_eq!(old, new);
    }

    #[test]
    fn fouiller_ne_demande_ni_script_ni_sortie() {
        let args = parse_of(&["--list", "*kerrigan*", "--game", "D:/Jeux/SC2"]).unwrap();
        assert_eq!(args.list.as_deref(), Some("*kerrigan*"));
    }

    // Du calcul pur : aucun jeu a ouvrir.
    #[test]
    fn hacher_ne_demande_rien_d_autre() {
        let args = parse_of(&["--hash", "base\\vo\\judy.wem"]).unwrap();
        assert_eq!(args.hash.as_deref(), Some("base\\vo\\judy.wem"));
        assert!(args.game.as_os_str().is_empty());
    }

    #[test]
    fn ce_qui_manque_est_dit() {
        assert!(parse_of(&["--script", "sc2.rhai"]).unwrap_err().contains("--game"));
        assert!(parse_of(&["--game", "D:/Jeux"]).unwrap_err().contains("--script"));
        assert!(
            parse_of(&["--script", "sc2.rhai", "--game", "D:/Jeux"])
                .unwrap_err()
                .contains("--out")
        );
    }

    #[test]
    fn un_argument_inconnu_rappelle_l_usage() {
        let e = parse_of(&["--balade"]).unwrap_err();
        assert!(e.contains("--balade"), "{e}");
        assert!(e.contains("pack-builder --script"), "{e}");
    }

    #[test]
    fn un_drapeau_sans_valeur_se_plaint() {
        assert!(parse_of(&["--script"]).unwrap_err().contains("attend une valeur"));
    }
}
