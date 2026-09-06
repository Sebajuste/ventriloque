// Les paquets : des donnees qui arrivent par un zip, installees depuis la fenetre.
//
// CE QU'EST UN PAQUET. Un zip qui porte un `pack.json` a sa racine et des dossiers dont les noms
// sont ceux de Ventriloque :
//
//   pack.json      le manifeste -- nom, version, description
//   voix/          des references .wav, pretes a etre clonees
//   pnj/           des fiches de personnages
//   modeles/       le moteur de parole, un demi-gigaoctet, qui ne peut pas etre livre autrement
//
// LE MANIFESTE EST OBLIGATOIRE, et c'est une protection plutot qu'une formalite : sans lui, un
// zip quelconque tombe sur le bouton « installer » repandrait son contenu dans les dossiers de
// l'application. Un zip sans `pack.json` est refuse avant d'etre ouvert plus avant.
//
// AUCUNE ENTREE NE SORT DE LA RACINE. Un zip peut nommer `..\..\Windows\System32\...` ou un
// chemin absolu -- c'est une attaque connue et elle est ancienne. Chaque entree est verifiee :
// un des trois dossiers en tete, aucun `..`, aucune racine. Ce qui ne passe pas est ignore et
// compte, plutot que d'arreter l'installation d'un paquet par ailleurs sain.

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

// Les seuls dossiers qu'un paquet peut remplir.
const ACCUEIL: [&str; 3] = ["voix", "pnj", "modeles"];

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Manifeste {
    pub nom: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub auteur: String,
    // Rempli a l'installation, pas par l'auteur du paquet.
    #[serde(default)]
    pub fichiers: Vec<String>,
    #[serde(default)]
    pub installe_le: String,
}

// Un chemin d'entree de zip, rendu sur si possible.
//
// Rend `None` pour tout ce qui n'a rien a faire chez nous : un dossier hors des trois accueillis,
// un `..`, un chemin absolu, une lettre de lecteur.
fn accueillir(brut: &str) -> Option<PathBuf> {
    let normalise = brut.replace(0x5c as char, "/");
    let chemin = Path::new(&normalise);

    let mut morceaux = chemin.components();
    let tete = match morceaux.next() {
        Some(Component::Normal(t)) => t.to_str()?,
        _ => return None,
    };
    if !ACCUEIL.contains(&tete) {
        return None;
    }
    // Tout le reste doit etre du nom de fichier ordinaire.
    if chemin.components().any(|c| !matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(chemin.to_path_buf())
}

pub fn installer(racine: &Path, zip: &Path) -> Result<Manifeste> {
    let fichier = std::fs::File::open(zip).with_context(|| format!("ouverture de {}", zip.display()))?;
    let mut archive = zip::ZipArchive::new(fichier).context("ce fichier n'est pas un zip lisible")?;

    let mut manifeste: Manifeste = {
        let mut entree = archive
            .by_name("pack.json")
            .map_err(|_| anyhow!("ce zip n'a pas de pack.json a sa racine : ce n'est pas un paquet Ventriloque"))?;
        let mut texte = String::new();
        std::io::Read::read_to_string(&mut entree, &mut texte).context("lecture de pack.json")?;
        serde_json::from_str(&texte).context("pack.json est mal forme")?
    };
    if manifeste.nom.trim().is_empty() {
        return Err(anyhow!("pack.json ne nomme pas le paquet"));
    }

    let mut poses = Vec::new();
    let mut refuses = 0usize;

    for i in 0..archive.len() {
        let mut entree = archive.by_index(i).context("lecture d'une entree du zip")?;
        if entree.is_dir() {
            continue;
        }
        let nom = entree.name().to_string();
        if nom == "pack.json" {
            continue;
        }
        let Some(relatif) = accueillir(&nom) else {
            refuses += 1;
            continue;
        };

        let cible = racine.join(&relatif);
        if let Some(parent) = cible.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut sortie = std::fs::File::create(&cible).with_context(|| format!("ecriture de {}", cible.display()))?;
        std::io::copy(&mut entree, &mut sortie).with_context(|| format!("copie de {nom}"))?;
        poses.push(relatif.to_string_lossy().replace(0x5c as char, "/"));
    }

    if poses.is_empty() {
        return Err(anyhow!(
            "ce paquet n'apporte rien d'utilisable ({refuses} entree(s) hors de voix/, pnj/ et modeles/)"
        ));
    }

    manifeste.fichiers = poses;
    manifeste.installe_le = horodatage();
    inscrire(racine, &manifeste)?;
    Ok(manifeste)
}

// Ce qui a ete installe, garde a cote des donnees. Sert a l'affichage, et rend une
// desinstallation possible plus tard sans deviner ce qui appartient a quoi.
fn inscrire(racine: &Path, manifeste: &Manifeste) -> Result<()> {
    let dossier = racine.join("packs");
    std::fs::create_dir_all(&dossier).ok();
    let id: String = manifeste
        .nom
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '_' })
        .collect();
    let texte = serde_json::to_string_pretty(manifeste)?;
    std::fs::write(dossier.join(format!("{}.json", id.trim_matches('_'))), texte)
        .context("inscription du paquet")
}

pub fn installes(racine: &Path) -> Vec<Manifeste> {
    let mut liste: Vec<Manifeste> = std::fs::read_dir(racine.join("packs"))
        .map(|entrees| {
            entrees
                .flatten()
                .filter(|e| e.path().extension().is_some_and(|x| x == "json"))
                .filter_map(|e| serde_json::from_str(&std::fs::read_to_string(e.path()).ok()?).ok())
                .collect()
        })
        .unwrap_or_default();
    liste.sort_by(|a: &Manifeste, b: &Manifeste| a.nom.to_lowercase().cmp(&b.nom.to_lowercase()));
    liste
}

// Une date lisible, sans dependance de plus : le temps systeme suffit pour dire quel jour.
fn horodatage() -> String {
    let secondes = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let jours = secondes / 86_400;
    let (mut a, mut reste) = (1970i64, jours as i64);
    loop {
        let bissextile = (a % 4 == 0 && a % 100 != 0) || a % 400 == 0;
        let longueur = if bissextile { 366 } else { 365 };
        if reste < longueur {
            break;
        }
        reste -= longueur;
        a += 1;
    }
    let bissextile = (a % 4 == 0 && a % 100 != 0) || a % 400 == 0;
    let mois = [31, if bissextile { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut m = 0usize;
    while m < 12 && reste >= mois[m] {
        reste -= mois[m];
        m += 1;
    }
    format!("{a:04}-{:02}-{:02}", m + 1, reste + 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn zip_de(entrees: &[(&str, &str)]) -> tempo::Fichier {
        let f = tempo::Fichier::neuf("zip");
        let mut z = zip::ZipWriter::new(std::fs::File::create(f.chemin()).unwrap());
        let o: zip::write::FileOptions<()> = zip::write::FileOptions::default();
        for (nom, contenu) in entrees {
            z.start_file(*nom, o).unwrap();
            z.write_all(contenu.as_bytes()).unwrap();
        }
        z.finish().unwrap();
        f
    }

    // Un dossier jetable, sans dependance de plus.
    mod tempo {
        use std::path::PathBuf;
        pub struct Fichier(PathBuf);
        impl Fichier {
            pub fn neuf(suffixe: &str) -> Self {
                let n = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .subsec_nanos();
                let p = std::env::temp_dir().join(format!("ventriloque-test-{n}-{suffixe}"));
                if suffixe == "dir" {
                    std::fs::create_dir_all(&p).unwrap();
                }
                Self(p)
            }
            pub fn chemin(&self) -> &std::path::Path {
                &self.0
            }
        }
        impl Drop for Fichier {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }
    }

    #[test]
    fn installe_ce_qui_est_accueilli() {
        let racine = tempo::Fichier::neuf("dir");
        let zip = zip_de(&[
            ("pack.json", r#"{"nom":"essai","version":"1"}"#),
            ("voix/barman.wav", "RIFF...."),
            ("pnj/barman.json", r#"{"nom":"Barman","voix":"barman.wav"}"#),
        ]);
        let m = installer(racine.chemin(), zip.chemin()).unwrap();
        assert_eq!(m.nom, "essai");
        assert_eq!(m.fichiers.len(), 2);
        assert!(racine.chemin().join("voix/barman.wav").is_file());
        assert!(racine.chemin().join("pnj/barman.json").is_file());
        // Le paquet est inscrit, pour qu'on sache plus tard ce qu'il a pose.
        assert!(racine.chemin().join("packs/essai.json").is_file());
    }

    #[test]
    fn refuse_un_zip_sans_manifeste() {
        let racine = tempo::Fichier::neuf("dir");
        let zip = zip_de(&[("voix/x.wav", "RIFF")]);
        let e = installer(racine.chemin(), zip.chemin()).unwrap_err().to_string();
        assert!(e.contains("pack.json"), "{e}");
        assert!(!racine.chemin().join("voix").exists());
    }

    // L'attaque connue : un zip qui remonte hors de la racine ou vise un chemin absolu.
    #[test]
    fn ne_sort_jamais_de_la_racine() {
        assert!(accueillir("voix/ok.wav").is_some());
        assert!(accueillir("modeles/catalogue/jean.kv").is_some());

        for mechant in [
            "../dehors.txt",
            "voix/../../dehors.txt",
            "/etc/passwd",
            "C:/Windows/System32/dehors.dll",
            "autre/x.wav",
            "pack.json.bak",
        ] {
            assert!(accueillir(mechant).is_none(), "accepte a tort : {mechant}");
        }
    }

    #[test]
    fn le_dossier_seul_ne_suffit_pas() {
        let racine = tempo::Fichier::neuf("dir");
        let zip = zip_de(&[("pack.json", r#"{"nom":"vide"}"#), ("ailleurs/x.txt", "non")]);
        let e = installer(racine.chemin(), zip.chemin()).unwrap_err().to_string();
        assert!(e.contains("n'apporte rien"), "{e}");
    }
}
