// Les fiches ecrites sous les anciens noms de champs.
//
// POURQUOI UN TYPE A PART, ET PAS DES `#[serde(alias)]`. L'alias aurait tenu en un mot par
// champ -- et il aurait traverse la frontiere : `specta` decrit un type qui accepte deux noms
// par une union, et le contrat TypeScript de `Character` devenait une alternative de neuf termes
// que plus aucune vue ne savait manipuler. Le format DE LECTURE d'un fichier et le format DE
// L'IPC sont deux choses ; les confondre coutait le contrat.
//
// Ce fichier a donc une date de peremption : le jour ou plus personne n'a de fiche a l'ancien
// format, il se supprime d'un bloc, et rien d'autre ne bouge.

use serde::Deserialize;

use super::Character;

#[derive(Deserialize)]
struct LegacyCharacter {
    #[serde(default)]
    nom: String,
    #[serde(default)]
    univers: String,
    #[serde(default)]
    voix: String,
    #[serde(default)]
    repliques: Vec<String>,
}

impl From<LegacyCharacter> for Character {
    fn from(old: LegacyCharacter) -> Self {
        Character {
            id: String::new(),
            name: old.nom,
            universe: old.univers,
            voice: old.voix,
            lines: old.repliques,
            pace: super::NORMAL_PACE,
        }
    }
}

/// Lit une fiche, dans le format d'aujourd'hui ou celui d'avant.
///
/// L'ORDRE COMPTE : le format d'aujourd'hui d'abord. Une fiche recente n'a pas de champ `nom`,
/// donc l'ancien format la lirait en un personnage sans nom plutot que d'echouer.
pub fn parse(text: &str) -> Option<Character> {
    if let Ok(character) = serde_json::from_str::<Character>(text)
        && !character.name.trim().is_empty()
    {
        return Some(character);
    }
    serde_json::from_str::<LegacyCharacter>(text)
        .ok()
        .map(Character::from)
        .filter(|c| !c.name.trim().is_empty())
}

#[cfg(test)]
mod tests {
    #[test]
    fn une_fiche_d_aujourd_hui_se_lit() {
        let c = super::parse(r#"{"name":"Nova","universe":"SC2","voice":"n.wav","lines":["Ok"]}"#)
            .unwrap();
        assert_eq!(c.name, "Nova");
        assert_eq!(c.universe, "SC2");
        assert_eq!(c.voice, "n.wav");
        assert_eq!(c.lines, vec!["Ok".to_string()]);
    }

    #[test]
    fn une_fiche_d_avant_se_lit_aussi() {
        let c = super::parse(
            r#"{"nom":"Sarah Kerrigan","univers":"StarCraft II","voix":"sc2.wav","repliques":["Korhal."]}"#,
        )
        .unwrap();
        assert_eq!(c.name, "Sarah Kerrigan");
        assert_eq!(c.universe, "StarCraft II");
        assert_eq!(c.voice, "sc2.wav");
        assert_eq!(c.lines, vec!["Korhal.".to_string()]);
    }

    #[test]
    fn ce_qui_ne_nomme_personne_n_est_pas_une_fiche() {
        assert!(super::parse("{}").is_none());
        assert!(super::parse(r#"{"name":"   "}"#).is_none());
        assert!(super::parse("pas du json").is_none());
    }
}
