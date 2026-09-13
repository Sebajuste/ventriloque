// Poser le contenu d'un zip dans les dossiers de l'application.

use anyhow::{Context, Result, anyhow};
use std::io::{Read, Write};
use std::path::Path;

use crate::clock;

use super::entry::{as_slash, safe_entry};
use super::job::Job;
use super::legacy;
use super::manifest::Manifest;
use super::store;

/// Le nom du manifeste, a la racine du zip. Sans lui, ce n'est pas un paquet Ventriloque.
const MANIFEST: &str = "pack.json";

/// Un tampon de copie assez gros pour que le disque travaille, assez petit pour que la barre
/// bouge plusieurs fois par seconde sur un fichier de 204 Mo.
const COPY_BUFFER: usize = 64 * 1024;

pub fn install(root: &Path, zip: &Path, job: &Job) -> Result<Manifest> {
    let file =
        std::fs::File::open(zip).with_context(|| format!("ouverture de {}", zip.display()))?;
    let mut archive = zip::ZipArchive::new(file).context("ce fichier n'est pas un zip lisible")?;

    let mut manifest = read_manifest(&mut archive)?;
    let mut placed = Vec::new();
    let mut refused = 0usize;

    // PREMIERE PASSE : ce qu'il y a a ecrire, en kilooctets. Sans ce total, la barre ne pourrait
    // qu'aller et venir ; avec lui, elle dit vraiment ou l'on en est, meme quand une seule entree
    // pese 204 Mo.
    let mut to_write = 0u64;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).context("lecture d'une entree du zip")?;
        if !entry.is_dir() && safe_entry(entry.name()).is_some() {
            to_write += entry.size();
        }
    }
    // Au moins un pas, meme pour un paquet minuscule : un total nul voudrait dire
    // « compte inconnu », et la barre irait et viendrait pour rien.
    let steps = (to_write / 1024).max(1) as usize;
    job.start(&manifest.name, steps);

    let mut written = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).context("lecture d'une entree du zip")?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.name().to_string();
        if name == MANIFEST {
            continue;
        }
        let Some(relative) = safe_entry(&name) else {
            refused += 1;
            continue;
        };
        job.set_step(&name);

        let target = root.join(&relative);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        let mut output = std::fs::File::create(&target)
            .with_context(|| format!("ecriture de {}", target.display()))?;

        // Copie a la main plutot que `std::io::copy` : c'est le seul endroit d'ou l'on peut dire
        // qu'on avance PENDANT un fichier de 204 Mo, et pas seulement entre deux fichiers.
        let mut buffer = vec![0u8; COPY_BUFFER];
        loop {
            let read = entry.read(&mut buffer).with_context(|| format!("lecture de {name}"))?;
            if read == 0 {
                break;
            }
            output.write_all(&buffer[..read]).with_context(|| format!("copie de {name}"))?;
            written += read as u64;
            job.advance((written / 1024) as usize);
        }

        let posted = as_slash(&relative);
        job.log(&posted);
        placed.push(posted);
    }

    if placed.is_empty() {
        return Err(anyhow!(
            "ce paquet n'apporte rien d'utilisable ({refused} entree(s) hors de voices/, characters/, models/ et recipes/)"
        ));
    }

    // L'arrondi au kilooctet laisserait la barre a 99 % sur un paquet minuscule : ce qui
    // est ecrit est ecrit, on le dit.
    job.advance(steps);

    // REINSTALLER NE DOIT PAS ORPHELINER CE QU'UNE FABRICATION A POSE. Un paquet-recette voit sa
    // liste grandir apres coup, quand le script produit les voix ; ecraser l'inscription avec le
    // seul contenu du zip laisserait quarante fichiers sur le disque que plus aucune
    // desinstallation ne saurait retirer.
    //
    // Ce qui a disparu du disque, en revanche, ne se reporte pas : la liste dirait le faux.
    if let Some(before) = store::installed(root).into_iter().find(|m| m.id() == manifest.id()) {
        for kept in before.files {
            if !placed.contains(&kept) && root.join(&kept).exists() {
                placed.push(kept);
            }
        }
        manifest.built_on = before.built_on;
    }
    manifest.files = placed;
    manifest.installed_on = clock::today();
    store::record(root, &manifest)?;
    Ok(manifest)
}

fn read_manifest<R: Read + std::io::Seek>(archive: &mut zip::ZipArchive<R>) -> Result<Manifest> {
    let mut entry = archive.by_name(MANIFEST).map_err(|_| {
        anyhow!("ce zip n'a pas de pack.json a sa racine : ce n'est pas un paquet Ventriloque")
    })?;
    let mut text = String::new();
    entry.read_to_string(&mut text).context("lecture de pack.json")?;
    // `legacy::parse` refuse deja ce qui ne nomme personne : les deux causes se disent d'une
    // seule phrase, parce qu'elles se reparent de la meme facon -- ouvrir le pack.json.
    legacy::parse(&text).ok_or_else(|| anyhow!("pack.json est mal forme ou ne nomme pas le paquet"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::packs::testing::zip_with;
    use crate::testing::TempDir;

    #[test]
    fn installe_ce_qui_est_accueilli() {
        let root = TempDir::new("install");
        let zip = zip_with(
            &root,
            "paquet.zip",
            &[
                ("pack.json", r#"{"name":"essai","version":"1"}"#),
                ("voices/barman.wav", "RIFF...."),
                ("characters/barman.json", r#"{"name":"Barman","voice":"barman.wav"}"#),
            ],
        );

        let m = install(root.path(), &zip, &Job::default()).unwrap();

        assert_eq!(m.name, "essai");
        assert_eq!(m.files.len(), 2);
        assert!(root.path().join("voices/barman.wav").is_file());
        assert!(root.path().join("characters/barman.json").is_file());
        // Le paquet est inscrit, pour qu'on sache plus tard ce qu'il a pose.
        assert!(root.path().join("packs/essai.json").is_file());
        assert_eq!(m.installed_on, crate::clock::today());
    }

    // Un paquet deja distribue nomme ses dossiers en francais : il s'installe encore, et son
    // contenu atterrit sous les noms d'aujourd'hui.
    #[test]
    fn un_paquet_a_l_ancien_format_s_installe_au_nouveau() {
        let root = TempDir::new("install-ancien");
        let zip = zip_with(
            &root,
            "ancien.zip",
            &[
                ("pack.json", r#"{"nom":"ancien","auteur":"moi"}"#),
                ("voix/barman.wav", "RIFF"),
                ("pnj/barman.json", r#"{"nom":"Barman","voix":"barman.wav"}"#),
            ],
        );

        let m = install(root.path(), &zip, &Job::default()).unwrap();

        assert_eq!(m.name, "ancien");
        assert_eq!(m.author, "moi");
        assert!(root.path().join("voices/barman.wav").is_file());
        assert!(root.path().join("characters/barman.json").is_file());
        assert_eq!(m.files, vec!["voices/barman.wav", "characters/barman.json"]);
    }

    #[test]
    fn refuse_un_zip_sans_manifeste() {
        let root = TempDir::new("install-sans-manifeste");
        let zip = zip_with(&root, "nu.zip", &[("voices/x.wav", "RIFF")]);

        let e = install(root.path(), &zip, &Job::default()).unwrap_err().to_string();

        assert!(e.contains("pack.json"), "{e}");
        assert!(!root.path().join("voices").exists());
    }

    #[test]
    fn refuse_un_manifeste_qui_ne_nomme_rien() {
        let root = TempDir::new("install-anonyme");
        let zip = zip_with(&root, "anonyme.zip", &[("pack.json", r#"{"name":"  "}"#)]);

        let e = install(root.path(), &zip, &Job::default()).unwrap_err().to_string();
        assert!(e.contains("ne nomme pas"), "{e}");
    }

    #[test]
    fn le_dossier_seul_ne_suffit_pas() {
        let root = TempDir::new("install-vide");
        let zip = zip_with(
            &root,
            "vide.zip",
            &[("pack.json", r#"{"name":"vide"}"#), ("ailleurs/x.txt", "non")],
        );

        let e = install(root.path(), &zip, &Job::default()).unwrap_err().to_string();
        assert!(e.contains("n'apporte rien"), "{e}");
    }

    // Le chantier reste ARME a la sortie : c'est l'appelant qui le clot, parce que lui seul sait
    // quand le compte rendu est parti vers la fenetre. Le fermer ici ferait disparaitre la barre
    // avant que le resultat s'affiche.
    #[test]
    fn l_installation_dit_ou_elle_en_est() {
        let root = TempDir::new("install-avancement");
        let zip = zip_with(
            &root,
            "deux.zip",
            &[
                ("pack.json", r#"{"name":"essai"}"#),
                ("voices/a.wav", "RIFF"),
                ("voices/b.wav", "RIFF"),
            ],
        );
        let job = Job::default();

        install(root.path(), &zip, &job).unwrap();

        let state = job.read();
        assert!(state.active, "le chantier se clot chez l'appelant, pas ici");
        assert!(state.total > 0, "sans total, la barre ne pourrait qu'aller et venir");
        assert_eq!(state.done, state.total, "tout ce qui etait annonce a ete ecrit");
        // Le journal ne porte que ce qui a ete pose -- pas `pack.json`, qui n'est pas un fichier
        // du paquet mais son manifeste.
        assert_eq!(job.lines(), vec!["voices/a.wav", "voices/b.wav"]);
    }

    // LE VRAI MECANISME : la fenetre lit pendant qu'un autre fil ecrit. Si cette lecture ne
    // voyait pas les ecritures en cours, la barre resterait a zero jusqu'a la fin -- ce qui est
    // exactement le symptome qu'on cherche a exclure.
    #[test]
    fn on_peut_lire_l_avancement_pendant_que_l_installation_tourne() {
        use std::sync::Arc;

        let root = TempDir::new("install-concurrent");
        // Assez gros pour que la copie dure plus qu'un battement de lecture.
        let big = "x".repeat(6 * 1024 * 1024);
        let zip = zip_with(
            &root,
            "gros.zip",
            &[("pack.json", r#"{"name":"gros"}"#), ("voices/gros.wav", &big)],
        );

        let job = Arc::new(Job::default());
        let worker = {
            let job = job.clone();
            let (r, z) = (root.path().to_path_buf(), zip.clone());
            std::thread::spawn(move || install(&r, &z, &job).map(|_| ()))
        };

        // On guette une lecture ou le travail a commence sans etre fini : c'est la preuve que
        // l'avancement circule d'un fil a l'autre.
        let mut seen_running = false;
        for _ in 0..2000 {
            let state = job.read();
            if state.active && state.total > 0 && state.done > 0 && state.done < state.total {
                seen_running = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_micros(200));
        }
        worker.join().unwrap().unwrap();

        assert!(seen_running, "l'avancement n'a jamais ete visible en cours de route");
    }

    // Le cas du paquet-recette : sa liste grandit a la fabrication, et une reinstallation ne
    // doit pas la perdre de vue.
    #[test]
    fn reinstaller_garde_la_trace_de_ce_qui_a_ete_fabrique() {
        let root = TempDir::new("install-recette");
        let zip = zip_with(
            &root,
            "recette.zip",
            &[
                ("pack.json", r#"{"name":"essai","recipe":"x.rhai"}"#),
                ("recipes/x.rhai", "log(\"bonjour\");"),
            ],
        );
        install(root.path(), &zip, &Job::default()).unwrap();

        // Ce qu'une fabrication aurait pose, inscrit comme elle le fait.
        std::fs::create_dir_all(root.path().join("voices")).unwrap();
        std::fs::write(root.path().join("voices/sc2_kerrigan.wav"), "RIFF").unwrap();
        let mut after = store::installed(root.path()).remove(0);
        after.files.push("voices/sc2_kerrigan.wav".into());
        after.built_on = "2026-09-06".into();
        store::record(root.path(), &after).unwrap();

        // La meme recette, reinstallee -- par exemple pour une version corrigee.
        let m = install(root.path(), &zip, &Job::default()).unwrap();

        assert!(m.files.contains(&"recipes/x.rhai".to_string()));
        assert!(
            m.files.contains(&"voices/sc2_kerrigan.wav".to_string()),
            "la voix fabriquee est devenue orpheline : {:?}",
            m.files
        );
        assert_eq!(m.built_on, "2026-09-06", "la date de fabrication tient");
    }

    // Ce qui a disparu du disque ne se reporte pas : la liste dirait le faux.
    #[test]
    fn reinstaller_oublie_ce_qui_n_existe_plus() {
        let root = TempDir::new("install-orphelin");
        let zip = zip_with(
            &root,
            "simple.zip",
            &[("pack.json", r#"{"name":"essai"}"#), ("voices/a.wav", "RIFF")],
        );
        install(root.path(), &zip, &Job::default()).unwrap();

        let mut after = store::installed(root.path()).remove(0);
        after.files.push("voices/efface-a-la-main.wav".into());
        store::record(root.path(), &after).unwrap();

        let m = install(root.path(), &zip, &Job::default()).unwrap();
        assert!(!m.files.contains(&"voices/efface-a-la-main.wav".to_string()));
    }
}
