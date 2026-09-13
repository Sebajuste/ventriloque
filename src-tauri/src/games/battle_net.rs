// Le réglage du lanceur Battle.net, pour savoir où il pose les jeux.
//
// POURQUOI CETTE SOURCE EN PLUS DU REGISTRE. Le registre de désinstallation dit où sont les jeux
// **déjà installés** ; ce fichier dit où le lanceur les **met**. Les deux se complètent : sur une
// machine où quelqu'un a choisi un dossier d'installation inhabituel, une liste de noms usuels
// (`Games`, `Program Files`) ne devinerait rien, et le registre ne servirait que si le jeu était
// déjà là au bon endroit.
//
// Mesuré sur la machine du projet : `DefaultInstallPath` vaut `D:/Jeux`, écrit `D:\/Jeux` dans
// le JSON. Le fichier ne porte PAS de chemin par jeu — sa section `Games` ne contient que des
// identifiants de service (`s2`, `fenris`) et des horodatages.

use std::path::PathBuf;

/// Le dossier d'installation par défaut, lu dans le texte du réglage. Séparé de la lecture du
/// fichier pour rester vérifiable sans dépendre d'un Battle.net installé.
pub fn default_install_path_in(config: &str) -> Option<PathBuf> {
    let json: serde_json::Value = serde_json::from_str(config).ok()?;
    let path = json.get("Client")?.get("Install")?.get("DefaultInstallPath")?.as_str()?;
    let path = path.trim();
    (!path.is_empty()).then(|| PathBuf::from(path))
}

/// Le même, lu où Battle.net le range. `None` si le lanceur n'est pas installé.
pub fn default_install_path() -> Option<PathBuf> {
    let appdata = std::env::var("APPDATA").ok()?;
    let config = PathBuf::from(appdata).join("Battle.net").join("Battle.net.config");
    default_install_path_in(&std::fs::read_to_string(config).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    // La forme exacte relevée sur la machine du projet : le séparateur y est écrit `\/`, que
    // JSON décode en `/`. Windows accepte les deux.
    #[test]
    fn le_dossier_par_defaut_se_lit() {
        let config = r#"{"Client":{"Install":{"DefaultInstallPath":"D:\/Jeux"}},"Games":{}}"#;
        assert_eq!(default_install_path_in(config), Some(PathBuf::from("D:/Jeux")));
    }

    #[test]
    fn un_reglage_sans_chemin_ne_rend_rien() {
        assert_eq!(default_install_path_in(r#"{"Client":{}}"#), None);
        assert_eq!(default_install_path_in(r#"{"Client":{"Install":{}}}"#), None);
    }

    // Un champ vide ou blanc vaut « pas de réponse » : sinon la racine des candidats serait le
    // dossier courant, et l'on énumérerait n'importe quoi.
    #[test]
    fn un_chemin_blanc_ne_compte_pas() {
        let config = r#"{"Client":{"Install":{"DefaultInstallPath":"   "}}}"#;
        assert_eq!(default_install_path_in(config), None);
    }

    #[test]
    fn un_reglage_illisible_ne_fait_pas_paniquer() {
        assert_eq!(default_install_path_in("pas du json"), None);
        assert_eq!(default_install_path_in(""), None);
    }
}
