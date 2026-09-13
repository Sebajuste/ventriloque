// Reprendre une installation faite sous une disposition d'avant.
//
// Les dossiers de donnees portaient des noms francais -- `voix\`, `pnj\`, `modeles\`. Une
// installation existante en est pleine, et le paquet de modeles pese un demi-gigaoctet : le
// laisser retelecharger pour un changement de vocabulaire aurait ete une facon couteuse de dire
// que le renommage compte plus que l'utilisateur.
//
// UN RENOMMAGE, PAS UNE COPIE. `rename` est instantane sur un meme volume, quelle que soit la
// taille du dossier -- ce qui rend cette migration invisible meme sur les modeles.
//
// PRUDENTE PAR CONSTRUCTION. Rien ne bouge si la cible existe deja : deux dossiers presents
// veulent dire que quelqu'un a fait quelque chose a la main, et ecraser serait alors le pire
// choix. Un echec de renommage est ignore de meme -- l'application demarre, simplement sans
// voir les anciennes donnees, ce qui se repare a la main.

use std::path::Path;

use serde_json::Value;

use crate::paths;

/// Les dossiers d'avant, et ce qu'ils sont devenus.
const RENAMED: [(&str, &str); 6] = [
    ("voix", paths::VOICES),
    ("pnj", paths::CHARACTERS),
    ("modeles", paths::MODELS),
    ("recettes", paths::RECIPES),
    ("moteur", paths::ENGINE),
    (".fabrique", paths::WORK),
];

/// Renomme ce qui doit l'etre, et rend ce qui a bouge -- pour le dire au journal.
pub fn run(root: &Path) -> Vec<String> {
    let mut moved = Vec::new();
    for (before, after) in RENAMED {
        let old = root.join(before);
        let new = root.join(after);
        if !old.is_dir() || new.exists() {
            continue;
        }
        if std::fs::rename(&old, &new).is_ok() {
            moved.push(format!("{before} -> {after}"));
        }
    }
    moved.extend(extract_tuning(root));
    moved
}

/// Sort les reglages du moteur du repere, ou ils ont d'abord vecu sous la cle `engine`.
///
/// POURQUOI ILS EN SORTENT. `ventriloque.json` est versionne dans le depot ; y ecrire a chaque
/// « Appliquer » faisait qu'un reglage de seance apparaissait comme une modification a commiter,
/// et que le reglage local d'un poste partait dans un commit. Voir `settings`.
///
/// LE REPERE N'EST REECRIT QU'EN RETIRANT UNE CLE, le reste recopie tel quel : il porte le
/// commentaire de l'utilisateur et son renvoi vers les modeles, qui ne sont pas a nous. S'il ne
/// se lit pas, on ne touche a rien -- l'ecraser sur une erreur de syntaxe serait le pire choix.
///
/// UN `tuning.json` DEJA LA GAGNE. Deux sources veulent dire que l'application a deja tourne
/// depuis la separation ; la cle restee dans le repere est alors une trace perimee, et c'est elle
/// qu'on retire.
fn extract_tuning(root: &Path) -> Option<String> {
    let marker = root.join(paths::MARKER);
    let text = std::fs::read_to_string(&marker).ok()?;
    let Ok(Value::Object(mut map)) = serde_json::from_str::<Value>(&text) else {
        return None;
    };
    let settings = map.remove("engine")?;

    let tuning = root.join(paths::TUNING);
    let carried = !tuning.exists();
    if carried {
        // On sort par `?` si la reprise echoue : le repere garde alors sa cle, et les reglages
        // ne sont perdus ni d'un cote ni de l'autre.
        let text = serde_json::to_string_pretty(&settings).ok()? + "\n";
        std::fs::write(&tuning, text).ok()?;
    }

    let cleaned = serde_json::to_string_pretty(&Value::Object(map)).ok()? + "\n";
    std::fs::write(&marker, cleaned).ok()?;

    Some(if carried {
        format!("{}:engine -> {}", paths::MARKER, paths::TUNING)
    } else {
        format!("{}:engine retire ({} fait foi)", paths::MARKER, paths::TUNING)
    })
}

#[cfg(test)]
mod tests {
    use crate::testing::TempDir;

    #[test]
    fn les_anciens_dossiers_prennent_leur_nouveau_nom() {
        let root = TempDir::new("migration");
        std::fs::create_dir_all(root.path().join("voix")).unwrap();
        std::fs::write(root.path().join("voix/barman.wav"), "RIFF").unwrap();
        std::fs::create_dir_all(root.path().join("pnj")).unwrap();

        let moved = super::run(root.path());

        assert_eq!(moved, vec!["voix -> voices", "pnj -> characters"]);
        assert!(root.path().join("voices/barman.wav").is_file());
        assert!(root.path().join("characters").is_dir());
        assert!(!root.path().join("voix").exists());
    }

    // Deux dossiers presents veulent dire une intervention manuelle : on n'ecrase rien.
    #[test]
    fn une_cible_deja_la_arrete_le_renommage() {
        let root = TempDir::new("migration-conflit");
        std::fs::create_dir_all(root.path().join("voix")).unwrap();
        std::fs::write(root.path().join("voix/ancienne.wav"), "RIFF").unwrap();
        std::fs::create_dir_all(root.path().join("voices")).unwrap();
        std::fs::write(root.path().join("voices/neuve.wav"), "RIFF").unwrap();

        assert!(super::run(root.path()).is_empty());
        assert!(root.path().join("voix/ancienne.wav").is_file());
        assert!(root.path().join("voices/neuve.wav").is_file());
    }

    #[test]
    fn une_installation_neuve_n_a_rien_a_migrer() {
        let root = TempDir::new("migration-neuve");
        assert!(super::run(root.path()).is_empty());
    }

    // Les reglages du moteur ont d'abord vecu dans le repere, qui est versionne.
    #[test]
    fn les_reglages_sortent_du_repere() {
        let root = TempDir::new("migration-tuning");
        let marker = root.path().join("ventriloque.json");
        std::fs::write(&marker, r#"{"_": "un mot", "engine": {"temperature": 0.55}}"#).unwrap();

        let moved = super::run(root.path());

        assert_eq!(moved, vec!["ventriloque.json:engine -> tuning.json"]);
        assert_eq!(crate::settings::engine(root.path()).temperature, 0.55);
        // Le commentaire de l'utilisateur reste, la cle est partie.
        let left = std::fs::read_to_string(&marker).unwrap();
        assert!(left.contains("un mot"), "{left}");
        assert!(!left.contains("engine"), "{left}");
    }

    // Deux sources : l'application a deja tourne depuis la separation, la cle est une trace
    // perimee. `tuning.json` fait foi, et le repere se nettoie quand meme.
    #[test]
    fn un_tuning_deja_la_gagne_sur_la_cle_perimee() {
        let root = TempDir::new("migration-tuning-double");
        let marker = root.path().join("ventriloque.json");
        std::fs::write(&marker, r#"{"engine": {"temperature": 0.9}}"#).unwrap();
        std::fs::write(root.path().join("tuning.json"), r#"{"temperature": 0.3}"#).unwrap();

        super::run(root.path());

        assert_eq!(crate::settings::engine(root.path()).temperature, 0.3);
        assert!(!std::fs::read_to_string(&marker).unwrap().contains("engine"));
    }

    // Un repere casse ne doit pas etre ecrase : il porte le texte de l'utilisateur.
    #[test]
    fn un_repere_illisible_est_laisse_tel_quel() {
        let root = TempDir::new("migration-repere-casse");
        let marker = root.path().join("ventriloque.json");
        std::fs::write(&marker, "{ pas du json").unwrap();

        assert!(super::run(root.path()).is_empty());
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), "{ pas du json");
    }

    // LE REPERE DU DEPOT DOIT REVENIR A L'OCTET PRES, et c'est tout l'objet de la separation.
    //
    // La migration reecrit le fichier avec `to_string_pretty`. Si celui qui est versionne n'a pas
    // exactement cette forme -- une indentation differente, un retour a la ligne final absent --
    // alors le premier demarrage sur une installation d'avant le rendrait « modifie » dans git.
    // On aurait deplace la salissure au lieu de la supprimer.
    #[test]
    fn le_repere_versionne_revient_intact() {
        let versioned =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join(crate::paths::MARKER);
        let before = std::fs::read_to_string(&versioned).expect("le repere du depot");

        let root = TempDir::new("migration-repere-du-depot");
        let marker = root.path().join(crate::paths::MARKER);
        // Le fichier tel qu'il etait avant la separation : le meme, plus la cle.
        let mut map: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&before).unwrap();
        map.insert("engine".into(), serde_json::json!({ "temperature": 0.55 }));
        std::fs::write(&marker, serde_json::to_string(&map).unwrap()).unwrap();

        super::run(root.path());

        assert_eq!(std::fs::read_to_string(&marker).unwrap(), before);
        assert_eq!(crate::settings::engine(root.path()).temperature, 0.55);
    }

    // Un repere sans reglages n'a rien a signaler, et n'est pas reecrit pour rien.
    #[test]
    fn un_repere_sans_reglages_n_est_pas_touche() {
        let root = TempDir::new("migration-repere-nu");
        let marker = root.path().join("ventriloque.json");
        let before = r#"{"_": "un mot"}"#;
        std::fs::write(&marker, before).unwrap();

        assert!(super::run(root.path()).is_empty());
        assert_eq!(std::fs::read_to_string(&marker).unwrap(), before);
    }
}
