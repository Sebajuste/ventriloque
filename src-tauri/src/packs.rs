// Les paquets : des donnees qui arrivent par un zip, installees depuis la fenetre.
//
// CE QU'EST UN PAQUET. Un zip qui porte un `pack.json` a sa racine et des dossiers dont les noms
// sont ceux de Ventriloque :
//
//   pack.json      le manifeste -- nom, version, description
//   voix/          des references .wav, pretes a etre clonees
//   pnj/           des fiches de personnages
//   modeles/       le moteur de parole, un demi-gigaoctet, qui ne peut pas etre livre autrement
//   recettes/      un script d'extraction, pour un paquet qui ne porte pas ses voix
//
// UN PAQUET-RECETTE NE PORTE AUCUN SON. Il porte la connaissance de ou chercher dans un jeu
// installe, et fabrique les voix sur la machine du joueur, depuis sa propre copie. C'est ce qui
// le rend partageable la ou un paquet de voix ne l'est pas : le script est du texte, il ne
// contient pas une seconde d'enregistrement d'acteur.
//
// RIEN NE S'EXECUTE A L'INSTALLATION. Le script est pose comme un fichier de donnees et n'est
// lance que par un geste explicite, depuis l'onglet Paquets -- et jamais dans ce processus :
// voir `recette.rs`.
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
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

// L'avancement de l'operation en cours sur les paquets, lu par la fenetre a intervalle.
//
// UN SEUL EMPLACEMENT SUFFIT, comme pour l'avancement d'une replique : les boutons se ferment
// pendant qu'une operation tourne, donc il n'y en a jamais deux a la fois.
//
// C'est un etat partage plutot qu'un evenement parce que c'est ce que le projet fait deja pour
// la parole (`audio::Avancement`) : la fenetre bat toutes les 200 ms et lit. Un evenement de
// plus aurait demande un second mecanisme pour la meme chose.
//
// LES PAS N'ONT PAS D'UNITE. Une installation les compte en kilooctets, un retrait en fichiers :
// la fenetre n'en tire qu'une proportion, et c'est `quoi` qui dit ce qui se passe en francais.
// Compter les entrees d'un zip aurait donne une barre inutile -- le paquet moteur a dix-sept
// entrees dont une de 204 Mo, donc dix-sept sauts dont un qui dure cinq secondes.
#[derive(Default)]
pub struct Chantier {
    actif: AtomicBool,
    faits: AtomicUsize,
    total: AtomicUsize,
    quoi: Mutex<String>,
    journal: Mutex<Vec<String>>,
}

impl Chantier {
    pub fn commencer(&self, quoi: &str, total: usize) {
        self.faits.store(0, Ordering::Relaxed);
        self.total.store(total, Ordering::Relaxed);
        if let Ok(mut garde) = self.journal.lock() {
            garde.clear();
        }
        self.dire(quoi);
        self.actif.store(true, Ordering::Relaxed);
    }

    /// Une ligne de plus au journal, que la fenetre montre au fur et a mesure.
    pub fn poser(&self, ligne: &str) {
        if let Ok(mut garde) = self.journal.lock() {
            garde.push(ligne.to_string());
        }
    }

    pub fn journal(&self) -> Vec<String> {
        self.journal.lock().map(|j| j.clone()).unwrap_or_default()
    }

    /// Ce qui se passe maintenant. Un total de zero veut dire « en cours, sans compte » : c'est
    /// le cas d'une fabrication, dont on ne connait pas le nombre d'etapes a l'avance.
    pub fn dire(&self, quoi: &str) {
        if let Ok(mut garde) = self.quoi.lock() {
            quoi.clone_into(&mut garde);
        }
    }

    pub fn avancer(&self, faits: usize) {
        self.faits.store(faits, Ordering::Relaxed);
    }

    // Le journal SURVIT a la fin : la fenetre le montre encore le temps d'afficher le resultat.
    pub fn finir(&self) {
        self.actif.store(false, Ordering::Relaxed);
        self.dire("");
    }

    /// Actif, faits, total, et ce qui se passe.
    pub fn lire(&self) -> (bool, usize, usize, String) {
        (
            self.actif.load(Ordering::Relaxed),
            self.faits.load(Ordering::Relaxed),
            self.total.load(Ordering::Relaxed),
            self.quoi.lock().map(|q| q.clone()).unwrap_or_default(),
        )
    }
}


// Les seuls dossiers qu'un paquet peut remplir.
const ACCUEIL: [&str; 4] = ["voix", "pnj", "modeles", "recettes"];

#[derive(Serialize, Deserialize, Clone, Debug, Default, specta::Type)]
pub struct Manifeste {
    pub nom: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub auteur: String,
    // Le script d'extraction, nomme sans son dossier : « starcraft2.rhai ». Vide pour un paquet
    // qui porte deja ses voix.
    #[serde(default)]
    pub recette: String,
    // Ce que la recette attend comme jeu, pour le dire a l'utilisateur avant qu'il cherche le
    // dossier : « StarCraft II ».
    #[serde(default)]
    pub jeu: String,
    // Le code du jeu tel que `.build.info` le porte -- « sc2 », « fenris » --, qui permet de
    // trouver l'installation sans rien demander. Voir `jeux.rs` : ce n'est PAS le code que rend
    // CascLib, qui dit « s2 » pour le meme jeu.
    #[serde(default)]
    pub produit: String,
    // Rempli a l'installation, pas par l'auteur du paquet.
    #[serde(default)]
    pub fichiers: Vec<String>,
    #[serde(default)]
    pub installe_le: String,
    // Rempli par la fabrication. Vide tant qu'un paquet-recette n'a pas tourne.
    #[serde(default)]
    pub fabrique_le: String,
}

impl Manifeste {
    /// L'identifiant de fichier sous lequel le paquet est inscrit dans `packs\`.
    pub fn identifiant(&self) -> String {
        let brut: String = self
            .nom
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        brut.trim_matches('_').to_string()
    }

    /// Un paquet-recette qui n'a pas encore tourne : il n'apporte rien tant qu'on ne l'a pas
    /// fabrique depuis une copie du jeu.
    pub fn a_fabriquer(&self) -> bool {
        !self.recette.is_empty() && self.fabrique_le.is_empty()
    }
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

pub fn installer(racine: &Path, zip: &Path, chantier: &Chantier) -> Result<Manifeste> {
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

    // PREMIERE PASSE : ce qu'il y a a ecrire, en kilooctets. Sans ce total, la barre ne pourrait
    // qu'aller et venir ; avec lui, elle dit vraiment ou l'on en est, meme quand une seule entree
    // pese 204 Mo.
    let mut a_ecrire = 0u64;
    for i in 0..archive.len() {
        let entree = archive.by_index(i).context("lecture d'une entree du zip")?;
        if !entree.is_dir() && accueillir(entree.name()).is_some() {
            a_ecrire += entree.size();
        }
    }
    // Au moins un pas, meme pour un paquet minuscule : un total nul voudrait dire
    // « compte inconnu », et la barre irait et viendrait pour rien.
    let pas = (a_ecrire / 1024).max(1) as usize;
    chantier.commencer(&manifeste.nom, pas);

    let mut ecrits = 0u64;
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
        chantier.dire(&nom);

        let cible = racine.join(&relatif);
        if let Some(parent) = cible.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut sortie = std::fs::File::create(&cible).with_context(|| format!("ecriture de {}", cible.display()))?;

        // Copie a la main plutot que `std::io::copy` : c'est le seul endroit d'ou l'on peut dire
        // qu'on avance PENDANT un fichier de 204 Mo, et pas seulement entre deux fichiers.
        let mut tampon = vec![0u8; 64 * 1024];
        loop {
            let lu = std::io::Read::read(&mut entree, &mut tampon)
                .with_context(|| format!("lecture de {nom}"))?;
            if lu == 0 {
                break;
            }
            std::io::Write::write_all(&mut sortie, &tampon[..lu])
                .with_context(|| format!("copie de {nom}"))?;
            ecrits += lu as u64;
            chantier.avancer((ecrits / 1024) as usize);
        }

        let pose = relatif.to_string_lossy().replace(0x5c as char, "/");
        chantier.poser(&pose);
        poses.push(pose);
    }

    if poses.is_empty() {
        return Err(anyhow!(
            "ce paquet n'apporte rien d'utilisable ({refuses} entree(s) hors de voix/, pnj/ et modeles/)"
        ));
    }

    // L'arrondi au kilooctet laisserait la barre a 99 % sur un paquet minuscule : ce qui
    // est ecrit est ecrit, on le dit.
    chantier.avancer(pas);

    // REINSTALLER NE DOIT PAS ORPHELINER CE QU'UNE FABRICATION A POSE. Un paquet-recette voit sa
    // liste grandir apres coup, quand le script produit les voix ; ecraser l'inscription avec le
    // seul contenu du zip laisserait quarante fichiers sur le disque que plus aucune
    // desinstallation ne saurait retirer.
    if let Some(avant) = installes(racine).into_iter().find(|m| m.identifiant() == manifeste.identifiant()) {
        for garde in avant.fichiers {
            if !poses.contains(&garde) && racine.join(&garde).exists() {
                poses.push(garde);
            }
        }
        manifeste.fabrique_le = avant.fabrique_le;
    }
    manifeste.fichiers = poses;
    manifeste.installe_le = horodatage();
    inscrire(racine, &manifeste)?;
    Ok(manifeste)
}

// Ce qui a ete installe, garde a cote des donnees. Sert a l'affichage, et rend une
// desinstallation possible plus tard sans deviner ce qui appartient a quoi.
pub fn inscrire(racine: &Path, manifeste: &Manifeste) -> Result<()> {
    let dossier = racine.join("packs");
    std::fs::create_dir_all(&dossier).ok();
    let texte = serde_json::to_string_pretty(manifeste)?;
    std::fs::write(dossier.join(format!("{}.json", manifeste.identifiant())), texte)
        .context("inscription du paquet")
}

// Retire ce qu'un paquet a pose, puis son inscription. Rend le nombre de fichiers effaces.
//
// LA LISTE DU MANIFESTE N'EST PAS CRUE SUR PAROLE. C'est un fichier sur le disque, modifiable a
// la main : chaque entree repasse par `accueillir` avant qu'on efface quoi que ce soit, tout
// comme une entree de zip a l'installation. Un manifeste bricole ne peut donc pas faire effacer
// autre chose que ce qu'un paquet aurait eu le droit de poser.
//
// CE QU'UN AUTRE PAQUET REVENDIQUE RESTE. Deux paquets peuvent livrer le meme fichier -- deux
// jeux d'un meme univers, un modele partage. Retirer l'un ne doit pas casser l'autre.
pub fn desinstaller(racine: &Path, nom: &str, chantier: &Chantier) -> Result<usize> {
    let tous = installes(racine);
    let manifeste = tous
        .iter()
        .find(|m| m.nom == nom)
        .ok_or_else(|| anyhow!("paquet inconnu : {nom}"))?;

    let garde: Vec<&String> = tous
        .iter()
        .filter(|m| m.nom != nom)
        .flat_map(|m| m.fichiers.iter())
        .collect();

    chantier.commencer(&manifeste.nom, manifeste.fichiers.len());
    let mut effaces = 0usize;
    for (rang, fichier) in manifeste.fichiers.iter().enumerate() {
        chantier.avancer(rang);
        chantier.dire(fichier);
        let Some(relatif) = accueillir(fichier) else { continue };
        if garde.contains(&fichier) {
            continue;
        }
        let cible = racine.join(&relatif);
        if std::fs::remove_file(&cible).is_ok() {
            effaces += 1;
            chantier.poser(fichier);
        }

        // Le cache de clonage vit sous le seul radical du fichier. Le laisser derriere serait
        // le piege documente : un paquet ulterieur qui livre le meme nom heriterait d'un etat
        // qui n'est pas le sien.
        if relatif.extension().is_some_and(|e| e.eq_ignore_ascii_case("wav")) {
            if let Some(radical) = relatif.file_stem() {
                for reste in ["emb", "kv"] {
                    let _ = std::fs::remove_file(
                        racine.join("voix").join(".cache").join(radical).with_extension(reste),
                    );
                }
            }
        }
    }

    chantier.avancer(manifeste.fichiers.len());
    std::fs::remove_file(racine.join("packs").join(format!("{}.json", manifeste.identifiant())))
        .context("retrait de l'inscription du paquet")?;
    Ok(effaces)
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
pub fn horodatage() -> String {
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
        let m = installer(racine.chemin(), zip.chemin(), &Chantier::default()).unwrap();
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
        let e = installer(racine.chemin(), zip.chemin(), &Chantier::default()).unwrap_err().to_string();
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

    // Le chantier reste ARME a la sortie : c'est l'appelant qui le clot, parce que lui seul sait
    // quand le compte rendu est parti vers la fenetre. Le fermer ici ferait disparaitre la barre
    // avant que le resultat s'affiche.
    // Le chantier reste ARME a la sortie : c'est l'appelant qui le clot, parce que lui seul
    // sait quand le compte rendu est parti vers la fenetre.
    #[test]
    fn l_installation_dit_ou_elle_en_est() {
        let racine = tempo::Fichier::neuf("dir");
        let zip = zip_de(&[
            ("pack.json", r#"{"nom":"essai"}"#),
            ("voix/a.wav", "RIFF"),
            ("voix/b.wav", "RIFF"),
        ]);
        let chantier = Chantier::default();
        assert_eq!(chantier.lire(), (false, 0, 0, String::new()));

        installer(racine.chemin(), zip.chemin(), &chantier).unwrap();

        let (actif, faits, total, _) = chantier.lire();
        assert!(actif, "le chantier se clot chez l'appelant, pas ici");
        assert!(total > 0, "sans total, la barre ne pourrait qu'aller et venir");
        assert_eq!(faits, total, "tout ce qui etait annonce a ete ecrit");
        // Le journal ne porte que ce qui a ete pose -- pas `pack.json`, qui n'est pas un fichier
        // du paquet mais son manifeste.
        assert_eq!(chantier.journal(), vec!["voix/a.wav", "voix/b.wav"]);
    }

    // LE VRAI MECANISME : la fenetre lit pendant qu'un autre fil ecrit. Si cette lecture ne
    // voyait pas les ecritures en cours, la barre resterait a zero jusqu'a la fin -- ce qui est
    // exactement le symptome qu'on cherche a exclure.
    #[test]
    fn on_peut_lire_l_avancement_pendant_que_l_installation_tourne() {
        use std::sync::Arc;

        let racine = tempo::Fichier::neuf("dir");
        // Assez gros pour que la copie dure plus qu'un battement de lecture.
        let gros = "x".repeat(6 * 1024 * 1024);
        let zip = zip_de(&[("pack.json", r#"{"nom":"gros"}"#), ("voix/gros.wav", &gros)]);

        let chantier = Arc::new(Chantier::default());
        let ouvrier = {
            let chantier = chantier.clone();
            let (r, z) = (racine.chemin().to_path_buf(), zip.chemin().to_path_buf());
            std::thread::spawn(move || installer(&r, &z, &chantier).map(|_| ()))
        };

        // On guette une lecture ou le travail a commence sans etre fini : c'est la preuve que
        // l'avancement circule d'un fil a l'autre.
        let mut vu_en_cours = false;
        for _ in 0..2000 {
            let (actif, faits, total, _) = chantier.lire();
            if actif && total > 0 && faits > 0 && faits < total {
                vu_en_cours = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_micros(200));
        }
        ouvrier.join().unwrap().unwrap();

        assert!(vu_en_cours, "l'avancement n'a jamais ete visible en cours de route");
    }

    // Le cas du paquet-recette : sa liste grandit a la fabrication, et une reinstallation ne
    // doit pas la perdre de vue.
    #[test]
    fn reinstaller_garde_la_trace_de_ce_qui_a_ete_fabrique() {
        let racine = tempo::Fichier::neuf("dir");
        let zip = zip_de(&[
            ("pack.json", r#"{"nom":"essai","recette":"x.rhai"}"#),
            ("recettes/x.rhai", "dire(\"bonjour\");"),
        ]);
        installer(racine.chemin(), zip.chemin(), &Chantier::default()).unwrap();

        // Ce qu'une fabrication aurait pose, inscrit comme elle le fait.
        std::fs::create_dir_all(racine.chemin().join("voix")).unwrap();
        std::fs::write(racine.chemin().join("voix/sc2_kerrigan.wav"), "RIFF").unwrap();
        let mut apres = installes(racine.chemin()).remove(0);
        apres.fichiers.push("voix/sc2_kerrigan.wav".into());
        apres.fabrique_le = "2026-09-06".into();
        inscrire(racine.chemin(), &apres).unwrap();

        // La meme recette, reinstallee -- par exemple pour une version corrigee.
        let m = installer(racine.chemin(), zip.chemin(), &Chantier::default()).unwrap();

        assert!(m.fichiers.contains(&"recettes/x.rhai".to_string()));
        assert!(
            m.fichiers.contains(&"voix/sc2_kerrigan.wav".to_string()),
            "la voix fabriquee est devenue orpheline : {:?}",
            m.fichiers
        );
        assert_eq!(m.fabrique_le, "2026-09-06", "la date de fabrication tient");
    }

    // Ce qui a disparu du disque ne se reporte pas : la liste dirait le faux.
    #[test]
    fn reinstaller_oublie_ce_qui_n_existe_plus() {
        let racine = tempo::Fichier::neuf("dir");
        let zip = zip_de(&[("pack.json", r#"{"nom":"essai"}"#), ("voix/a.wav", "RIFF")]);
        installer(racine.chemin(), zip.chemin(), &Chantier::default()).unwrap();

        let mut apres = installes(racine.chemin()).remove(0);
        apres.fichiers.push("voix/efface-a-la-main.wav".into());
        inscrire(racine.chemin(), &apres).unwrap();

        let m = installer(racine.chemin(), zip.chemin(), &Chantier::default()).unwrap();
        assert!(!m.fichiers.contains(&"voix/efface-a-la-main.wav".to_string()));
    }

    #[test]
    fn desinstalle_ce_qui_a_ete_pose_et_le_cache_qui_va_avec() {
        let racine = tempo::Fichier::neuf("dir");
        let zip = zip_de(&[
            ("pack.json", r#"{"nom":"essai"}"#),
            ("voix/barman.wav", "RIFF"),
            ("pnj/barman.json", "{}"),
        ]);
        installer(racine.chemin(), zip.chemin(), &Chantier::default()).unwrap();

        // Le moteur aurait laisse ceci a cote de la reference.
        let cache = racine.chemin().join("voix/.cache");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("barman.emb"), "x").unwrap();

        assert_eq!(desinstaller(racine.chemin(), "essai", &Chantier::default()).unwrap(), 2);
        assert!(!racine.chemin().join("voix/barman.wav").exists());
        assert!(!racine.chemin().join("pnj/barman.json").exists());
        assert!(!cache.join("barman.emb").exists());
        assert!(!racine.chemin().join("packs/essai.json").exists());
        assert!(installes(racine.chemin()).is_empty());
    }

    // Deux paquets peuvent livrer le meme fichier. Retirer l'un ne doit pas desarmer l'autre.
    #[test]
    fn laisse_ce_qu_un_autre_paquet_revendique() {
        let racine = tempo::Fichier::neuf("dir");
        let commun = zip_de(&[("pack.json", r#"{"nom":"commun"}"#), ("voix/partagee.wav", "RIFF")]);
        let autre = zip_de(&[("pack.json", r#"{"nom":"autre"}"#), ("voix/partagee.wav", "RIFF")]);
        installer(racine.chemin(), commun.chemin(), &Chantier::default()).unwrap();
        installer(racine.chemin(), autre.chemin(), &Chantier::default()).unwrap();

        assert_eq!(desinstaller(racine.chemin(), "commun", &Chantier::default()).unwrap(), 0);
        assert!(racine.chemin().join("voix/partagee.wav").is_file());
        assert_eq!(installes(racine.chemin()).len(), 1);
    }

    // Le manifeste est un fichier ordinaire : quelqu'un peut y ecrire ce qu'il veut.
    #[test]
    fn un_manifeste_bricole_ne_fait_pas_sortir_de_la_racine() {
        let racine = tempo::Fichier::neuf("dir");
        let zip = zip_de(&[("pack.json", r#"{"nom":"mechant"}"#), ("voix/vrai.wav", "RIFF")]);
        installer(racine.chemin(), zip.chemin(), &Chantier::default()).unwrap();

        let temoin = racine.chemin().join("ne-pas-toucher.txt");
        std::fs::write(&temoin, "je reste").unwrap();
        std::fs::write(
            racine.chemin().join("packs/mechant.json"),
            r#"{"nom":"mechant","fichiers":["../ne-pas-toucher.txt","voix/../ne-pas-toucher.txt","voix/vrai.wav"]}"#,
        )
        .unwrap();

        assert_eq!(desinstaller(racine.chemin(), "mechant", &Chantier::default()).unwrap(), 1);
        assert!(temoin.is_file(), "un chemin qui remonte a ete suivi");
    }

    #[test]
    fn le_dossier_seul_ne_suffit_pas() {
        let racine = tempo::Fichier::neuf("dir");
        let zip = zip_de(&[("pack.json", r#"{"nom":"vide"}"#), ("ailleurs/x.txt", "non")]);
        let e = installer(racine.chemin(), zip.chemin(), &Chantier::default()).unwrap_err().to_string();
        assert!(e.contains("n'apporte rien"), "{e}");
    }
}
