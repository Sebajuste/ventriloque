//! Ce qu'un script a le droit de nommer.
//!
//! CE QUI SORT DU SCRIPT EST TRAITE COMME CE QUI SORT D'UN ZIP. Le script ne recoit jamais de
//! chemin et ne peut pas en fabriquer : il propose des noms, et un nom qui n'est pas ordinaire
//! est refuse ici, avant que quoi que ce soit touche le disque.

/// La longueur maximale d'un nom propose. Large pour un nom de personnage, courte devant les
/// limites du systeme de fichiers.
const MAX_NAME: usize = 80;

/// Un nom de fichier propose par le script, rendu sur : pas de dossier, pas de `..`, rien
/// d'invisible.
pub fn safe_name(raw: &str) -> Result<String, String> {
    let clean = raw.trim();
    let acceptable = !clean.is_empty()
        && clean.len() <= MAX_NAME
        && clean.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
        && !clean.starts_with('.')
        && !clean.contains("..");
    if acceptable {
        Ok(clean.to_string())
    } else {
        Err(format!("nom refuse : « {raw} » (lettres, chiffres, _ - . seulement)"))
    }
}

/// Joker simple et insensible a la casse : `*` couvre n'importe quoi, `?` un caractere.
/// Sans joker, le motif est traite comme un fragment.
pub fn matches(pattern: &[u8], text: &[u8]) -> bool {
    match pattern.first() {
        None => text.is_empty(),
        Some(b'*') => {
            if pattern.len() == 1 {
                return true;
            }
            (0..=text.len()).any(|i| matches(&pattern[1..], &text[i..]))
        }
        Some(&c) => match text.first() {
            Some(&t) if c == b'?' || c == t => matches(&pattern[1..], &text[1..]),
            _ => false,
        },
    }
}

/// Met un motif sous la forme que `matches` attend : minuscules, barres obliques, et entoure de
/// jokers s'il n'en porte aucun.
pub fn normalise(raw: &str) -> String {
    let lower = raw.to_lowercase().replace('\\', "/");
    if lower.contains('*') || lower.contains('?') { lower } else { format!("*{lower}*") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_joker_couvre_les_cas_usuels() {
        assert!(matches(b"*kerrigan*", b"vo/zbriefing_kerrigan_007.ogg"));
        assert!(matches(b"*.ogg", b"a.ogg"));
        assert!(!matches(b"*.ogg", b"a.wav"));
        assert!(matches(b"a?c", b"abc"));
        assert!(!matches(b"a?c", b"ac"));
        assert!(matches(b"*", b""));
    }

    #[test]
    fn un_motif_sans_joker_devient_un_fragment() {
        assert_eq!(normalise("Kerrigan"), "*kerrigan*");
        assert_eq!(normalise("vo\\*.ogg"), "vo/*.ogg");
    }

    #[test]
    fn les_noms_dangereux_sont_refuses() {
        assert!(safe_name("sc2_kerrigan").is_ok());
        assert_eq!(safe_name("  judy  ").unwrap(), "judy");
        assert!(safe_name("../../windows/system32").is_err());
        assert!(safe_name("voices/ailleurs").is_err());
        assert!(safe_name("..").is_err());
        assert!(safe_name(".cache").is_err());
        assert!(safe_name("").is_err());
        assert!(safe_name(&"x".repeat(81)).is_err());
    }
}
