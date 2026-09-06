//! Choisir les prises qui composeront la reference.
//!
//! UNE PRISE PAR SCENE, A TOUR DE ROLE. Une reference tiree d'une seule scene ne porte qu'une
//! humeur -- le personnage y crie, ou y chuchote, pendant trente secondes. L'alternance en montre
//! plusieurs, et c'est ce que le clone reprend.
//!
//! LES PLUS COURTES D'ABORD. Dix repliques breves montrent plus de manieres de dire qu'un
//! monologue de la meme duree. Les deux copies de ce code divergeaient exactement ici -- l'autre
//! prenait les plus longues, pour « moins de coupes » : c'est le montage qui compte, pas le
//! nombre de coupes.

use super::decode::Take;

/// Le radical de scene d'un nom de prise : `zbriefing_korhal03_kerrigan_016.ogg` donne
/// `zbriefing_korhal03`. Les deux derniers morceaux sont le locuteur et le numero.
///
/// Un nom qui ne suit pas cette forme -- un hachage de Cyberpunk, par exemple -- devient sa
/// propre scene. C'est le bon repli : sans structure de nom, chaque prise vaut pour elle-meme et
/// l'alternance se ramene a l'ordre.
fn scene_of(name: &str) -> String {
    let file = name.rsplit(['\\', '/']).next().unwrap_or(name);
    let stem = file.split('.').next().unwrap_or(file);
    let parts: Vec<&str> = stem.split('_').collect();
    if parts.len() >= 3 {
        parts[..parts.len() - 2].join("_")
    } else {
        stem.to_string()
    }
}

/// Prend une prise par scene a tour de role, jusqu'a remplir la duree visee.
pub fn choose(takes: Vec<Take>, target_secs: f32) -> Vec<Take> {
    let mut scenes: Vec<(String, Vec<Take>)> = Vec::new();
    for take in takes {
        let key = scene_of(&take.name);
        match scenes.iter_mut().find(|(name, _)| *name == key) {
            Some((_, group)) => group.push(take),
            None => scenes.push((key, vec![take])),
        }
    }
    // Un ordre stable : deux fabrications de la meme copie du jeu doivent donner la meme
    // reference, sans quoi une voix changerait sans qu'on ait rien demande.
    scenes.sort_by(|a, b| a.0.cmp(&b.0));
    for (_, group) in scenes.iter_mut() {
        group.sort_by(|a, b| {
            a.duration_secs()
                .partial_cmp(&b.duration_secs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    let mut kept = Vec::new();
    let mut total = 0.0;
    while total < target_secs {
        let mut took_any = false;
        for (_, group) in scenes.iter_mut() {
            if group.is_empty() {
                continue;
            }
            took_any = true;
            let take = group.remove(0);
            total += take.duration_secs();
            kept.push(take);
            if total >= target_secs {
                break;
            }
        }
        // Plus rien a prendre : la recolte est maigre, mais on ne tourne pas en rond.
        if !took_any {
            break;
        }
    }
    kept
}

#[cfg(test)]
mod tests {
    use super::*;

    fn take(name: &str, seconds: f32) -> Take {
        Take {
            name: name.to_string(),
            samples: vec![0.1; (seconds * 100.0) as usize],
            sample_rate: 100,
        }
    }

    #[test]
    fn la_scene_retire_locuteur_et_numero() {
        assert_eq!(scene_of("zbriefing_korhal03_kerrigan_016.ogg"), "zbriefing_korhal03");
        assert_eq!(scene_of("chemin\\vers\\acresponses_artanis_020.ogg"), "acresponses");
        assert_eq!(scene_of("court.ogg"), "court");
    }

    // Cyberpunk nomme ses entrees par un hachage : aucune structure a lire.
    #[test]
    fn un_nom_sans_structure_devient_sa_propre_scene() {
        assert_eq!(scene_of("019291c8e47b9861"), "019291c8e47b9861");
    }

    #[test]
    fn le_choix_alterne_les_scenes() {
        let kept = choose(
            vec![take("a_x_001.ogg", 1.0), take("a_x_002.ogg", 1.0), take("b_x_001.ogg", 1.0)],
            2.0,
        );

        // Une prise de chaque scene avant d'en reprendre une seconde dans la premiere.
        assert_eq!(kept.len(), 2);
        assert_eq!(scene_of(&kept[0].name), "a");
        assert_eq!(scene_of(&kept[1].name), "b");
    }

    #[test]
    fn les_plus_courtes_partent_les_premieres() {
        let kept = choose(vec![take("a_x_001.ogg", 4.0), take("a_x_002.ogg", 1.0)], 1.5);

        // La breve d'abord, et la longue seulement parce que la cible n'etait pas atteinte :
        // le montage remplit jusqu'a la duree visee, il ne s'arrete pas au-dessous.
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].name, "a_x_002.ogg");
        assert_eq!(kept[1].name, "a_x_001.ogg");
    }

    // Une seule prise suffit quand elle couvre la cible : on ne prend pas la suivante pour rien.
    #[test]
    fn on_s_arrete_des_que_la_cible_est_couverte() {
        let kept = choose(vec![take("a_x_001.ogg", 4.0), take("a_x_002.ogg", 1.0)], 0.5);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].name, "a_x_002.ogg");
    }

    // Moins de matiere que demande : on prend tout, sans boucler sans fin.
    #[test]
    fn une_recolte_maigre_ne_boucle_pas() {
        assert_eq!(choose(vec![take("a_x_001.ogg", 1.0)], 60.0).len(), 1);
    }

    #[test]
    fn sans_prise_il_n_y_a_rien_a_choisir() {
        assert!(choose(Vec::new(), 32.0).is_empty());
    }
}
