// Reprendre une installation faite sous les anciens noms de dossiers.
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
    moved
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
}
