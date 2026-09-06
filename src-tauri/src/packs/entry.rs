// AUCUNE ENTREE NE SORT DE LA RACINE.
//
// Un zip peut nommer `..\..\Windows\System32\...` ou un chemin absolu -- c'est une attaque
// connue et elle est ancienne. Chaque entree est verifiee : un des dossiers accueillis en tete,
// aucun `..`, aucune racine. Ce qui ne passe pas est ignore et compte, plutot que d'arreter
// l'installation d'un paquet par ailleurs sain.
//
// LE MEME FILTRE SERT TROIS FOIS : a l'installation d'un zip, au retrait -- ou la liste vient
// d'un manifeste sur le disque, modifiable a la main -- et a la moisson d'une fabrication.
//
// LES NOMS D'AVANT SONT TRADUITS, PAS REFUSES. Un paquet distribue nomme ses dossiers `voix/`
// et `pnj/` : le refuser aurait rendu ininstallable ce qui est deja chez les gens, pour un
// changement de vocabulaire. L'entree est donc accueillie et posee sous le nom d'aujourd'hui.

use std::path::{Component, Path, PathBuf};

use crate::paths;

/// Les dossiers d'avant, et celui ou leur contenu atterrit maintenant.
const LEGACY: [(&str, &str); 4] = [
    ("voix", paths::VOICES),
    ("pnj", paths::CHARACTERS),
    ("modeles", paths::MODELS),
    ("recettes", paths::RECIPES),
];

/// Le dossier d'accueil d'une tete d'entree, sous son nom d'aujourd'hui. `None` si ce n'est pas
/// un dossier que nous remplissons.
fn welcome(head: &str) -> Option<&'static str> {
    if let Some(known) = paths::PACK_DIRS.iter().find(|d| **d == head) {
        return Some(known);
    }
    LEGACY.iter().find(|(before, _)| *before == head).map(|(_, after)| *after)
}

/// Un chemin d'entree de zip, rendu sur si possible et traduit au vocabulaire d'aujourd'hui.
///
/// Rend `None` pour tout ce qui n'a rien a faire chez nous : un dossier hors des accueillis, un
/// `..`, un chemin absolu, une lettre de lecteur.
pub fn safe_entry(raw: &str) -> Option<PathBuf> {
    let normalised = raw.replace('\\', "/");
    let path = Path::new(&normalised);

    let mut parts = path.components();
    let head = match parts.next() {
        Some(Component::Normal(head)) => head.to_str()?,
        _ => return None,
    };
    let folder = welcome(head)?;
    // Tout le reste doit etre du nom de fichier ordinaire.
    if path.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    // Un dossier seul n'est pas une entree a poser.
    let rest: PathBuf = parts.collect();
    if rest.as_os_str().is_empty() {
        return None;
    }
    Some(Path::new(folder).join(rest))
}

/// Le meme chemin, dit avec des barres obliques : c'est la forme qu'un manifeste inscrit.
pub fn as_slash(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ce_qui_est_accueilli_passe() {
        assert_eq!(safe_entry("voices/ok.wav"), Some(PathBuf::from("voices/ok.wav")));
        assert_eq!(
            safe_entry("models/catalogue/jean.kv"),
            Some(PathBuf::from("models/catalogue/jean.kv"))
        );
        assert_eq!(
            safe_entry("characters/barman.json"),
            Some(PathBuf::from("characters/barman.json"))
        );
        assert_eq!(safe_entry("recipes/sc2.rhai"), Some(PathBuf::from("recipes/sc2.rhai")));
    }

    // Un paquet deja distribue nomme ses dossiers en francais : il doit encore s'installer.
    #[test]
    fn les_anciens_dossiers_sont_traduits() {
        assert_eq!(safe_entry("voix/ok.wav"), Some(PathBuf::from("voices/ok.wav")));
        assert_eq!(safe_entry("pnj/barman.json"), Some(PathBuf::from("characters/barman.json")));
        assert_eq!(
            safe_entry("modeles/catalogue/jean.kv"),
            Some(PathBuf::from("models/catalogue/jean.kv"))
        );
        assert_eq!(safe_entry("recettes/sc2.rhai"), Some(PathBuf::from("recipes/sc2.rhai")));
        // Y compris ecrit a la mode Windows, comme le fait un zip fabrique sous Windows.
        assert_eq!(safe_entry("voix\\ok.wav"), Some(PathBuf::from("voices/ok.wav")));
    }

    // L'attaque connue : un zip qui remonte hors de la racine ou vise un chemin absolu.
    #[test]
    fn ne_sort_jamais_de_la_racine() {
        for hostile in [
            "../dehors.txt",
            "voices/../../dehors.txt",
            "voix/../../dehors.txt",
            "/etc/passwd",
            "C:/Windows/System32/dehors.dll",
            "autre/x.wav",
            "pack.json.bak",
            "",
        ] {
            assert!(safe_entry(hostile).is_none(), "accepte a tort : {hostile}");
        }
    }

    // Un dossier seul n'apporte rien : il ne doit pas compter comme une entree posee.
    #[test]
    fn un_dossier_sans_fichier_n_est_pas_une_entree() {
        assert!(safe_entry("voices").is_none());
        assert!(safe_entry("voix/").is_none());
    }
}
