//! La fiche de personnage qu'une recette produit, ecrite en JSON.
//!
//! A LA MAIN, PARCE QUE `serde_json` N'EST PAS DANS CE BINAIRE. Le format tient en cinq champs
//! dont un tableau de chaines ; tirer une dependance de plus dans un processus qui fait deja
//! tourner du C++ et un interpreteur ne se justifiait pas.
//!
//! Les noms de champs sont ceux que l'application lit aujourd'hui -- voir `characters` cote
//! Rust. Elle sait encore lire les anciens, mais rien ne sert d'en produire de nouveaux.

/// Ce qu'une fiche porte, et rien de plus.
pub struct Character {
    pub id: String,
    pub name: String,
    pub universe: String,
    pub voice: String,
    pub lines: Vec<String>,
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

pub fn to_json(character: &Character) -> String {
    let lines: Vec<String> =
        character.lines.iter().map(|l| format!("    \"{}\"", escape(l))).collect();
    format!(
        "{{\n  \"id\": \"{}\",\n  \"name\": \"{}\",\n  \"universe\": \"{}\",\n  \"voice\": \"{}\",\n  \"lines\": [\n{}\n  ]\n}}\n",
        escape(&character.id),
        escape(&character.name),
        escape(&character.universe),
        escape(&character.voice),
        lines.join(",\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn judy() -> Character {
        Character {
            id: "judy_alvarez".into(),
            name: "Judy Alvarez".into(),
            universe: "Cyberpunk 2077".into(),
            voice: "cp77_judy.wav".into(),
            lines: vec!["On y va ?".into()],
        }
    }

    #[test]
    fn la_fiche_porte_les_champs_que_l_application_lit() {
        let json = to_json(&judy());
        for field in ["\"id\"", "\"name\"", "\"universe\"", "\"voice\"", "\"lines\""] {
            assert!(json.contains(field), "{field} manque : {json}");
        }
        assert!(json.contains("\"Judy Alvarez\""));
    }

    #[test]
    fn la_fiche_echappe_ce_qui_casserait_le_json() {
        let awkward = Character {
            name: "Jean \"le Gros\"".into(),
            lines: vec!["a\\b".into(), "deux\nlignes".into()],
            ..judy()
        };
        let json = to_json(&awkward);
        assert!(json.contains("\\\"le Gros\\\""));
        assert!(json.contains("a\\\\b"));
        assert!(json.contains("deux\\nlignes"));
    }

    #[test]
    fn une_fiche_sans_replique_reste_du_json_valable() {
        let json = to_json(&Character { lines: Vec::new(), ..judy() });
        assert!(json.contains("\"lines\": [\n\n  ]"), "{json}");
    }
}
