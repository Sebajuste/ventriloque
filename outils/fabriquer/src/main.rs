// fabriquer -- execute le script d'un paquet Ventriloque sur une copie installee d'un jeu.
//
// CE BINAIRE N'EST PAS VENTRILOQUE, et c'est le point du dessin. Le C++ de CascLib et le script
// du paquet -- deux codes qui ne viennent pas de l'application -- tournent ici, dans un
// processus separe que Ventriloque lance sous job object et peut tuer d'un geste. L'application
// ne charge jamais ni l'un ni l'autre.
//
//   fabriquer --script <recette.rhai> --jeu <racine du jeu> --sortie <dossier de travail>
//   fabriquer --lister <motif> --jeu <racine du jeu>          pour fouiller a la main
//   fabriquer --hacher <chemin depot>                         pour ecrire une recette Cyberpunk
//
// La sortie porte `voix\*.wav` et `pnj\*.json`, que Ventriloque moissonne ensuite avec la meme
// rigueur qu'un zip. Rien n'est ecrit ailleurs, et rien n'est ecrit dans le dossier du jeu.

mod casc;
mod hote;
mod montage;
mod rdar;
mod stockage;
mod wwise;

use std::path::PathBuf;

struct Arguments {
    script: Option<PathBuf>,
    jeu: PathBuf,
    sortie: PathBuf,
    lister: Option<String>,
    hacher: Option<String>,
}

fn lire_arguments() -> Result<Arguments, String> {
    let bruts: Vec<String> = std::env::args().skip(1).collect();
    let mut args = Arguments {
        script: None,
        jeu: PathBuf::new(),
        sortie: PathBuf::new(),
        lister: None,
        hacher: None,
    };

    let mut i = 0;
    while i < bruts.len() {
        let suivant = || -> Result<String, String> {
            bruts.get(i + 1).cloned().ok_or_else(|| format!("{} attend une valeur", bruts[i]))
        };
        match bruts[i].as_str() {
            "--script" => args.script = Some(PathBuf::from(suivant()?)),
            "--jeu" => args.jeu = PathBuf::from(suivant()?),
            "--sortie" => args.sortie = PathBuf::from(suivant()?),
            "--lister" => args.lister = Some(suivant()?),
            "--hacher" => args.hacher = Some(suivant()?),
            autre => return Err(format!("argument inconnu : {autre}")),
        }
        i += 2;
    }

    // `--hacher` est du calcul pur : il ne lit ni jeu ni script.
    if args.hacher.is_some() {
        return Ok(args);
    }
    if args.jeu.as_os_str().is_empty() {
        return Err("--jeu est obligatoire".into());
    }
    if args.lister.is_none() {
        if args.script.is_none() {
            return Err("--script est obligatoire (ou --lister pour fouiller)".into());
        }
        if args.sortie.as_os_str().is_empty() {
            return Err("--sortie est obligatoire".into());
        }
    }
    Ok(args)
}

fn travailler() -> Result<(), String> {
    let args = lire_arguments()?;

    // UNE ARCHIVE CYBERPUNK NE CONNAIT QUE DES HACHAGES. Ecrire une recette pour ce jeu demande
    // donc de traduire un chemin depot en son FNV1a64 ; le faire ici evite d'avoir a refaire
    // l'algorithme, et de se tromper sur la casse.
    if let Some(chemin) = args.hacher {
        println!("{:016x}	{chemin}", rdar::hachage(&chemin));
        return Ok(());
    }

    let mut stockage = stockage::ouvrir(&args.jeu)?;
    println!("stockage ouvert : produit {}", stockage.produit());

    // Fouille a la main : pas de script, pas de sortie, juste ce que le stockage nomme.
    if let Some(motif) = args.lister {
        let compte = stockage.entrees().len();
        let bas = motif.to_lowercase().replace('\\', "/");
        let mut retenus = 0usize;
        for entree in stockage.entrees() {
            let nom = entree.nom.to_lowercase().replace('\\', "/");
            if nom.contains(&bas) {
                println!("{}\t{}", entree.taille, entree.nom);
                retenus += 1;
            }
        }
        eprintln!("{compte} entrees parcourues, {retenus} retenues.");
        return Ok(());
    }

    let chemin = args.script.expect("verifie par lire_arguments");
    let script = std::fs::read_to_string(&chemin)
        .map_err(|e| format!("script illisible ({}) : {e}", chemin.display()))?;

    // Le recensement coute une minute sur StarCraft II : on le paie avant de dire « fabrication »,
    // pour que la premiere ligne du journal ne soit pas suivie d'un long silence.
    println!("recensement : {} entrees", stockage.entrees().len());
    std::fs::create_dir_all(&args.sortie)
        .map_err(|e| format!("dossier de travail ({}) : {e}", args.sortie.display()))?;

    let recolte = hote::executer(&script, stockage, &args.sortie)?;
    if recolte.voix.is_empty() && recolte.fiches.is_empty() {
        return Err("le script n'a produit ni voix ni fiche".into());
    }
    println!(
        "fabrique : {} voix, {} fiches",
        recolte.voix.len(),
        recolte.fiches.len()
    );
    Ok(())
}

fn main() {
    if let Err(message) = travailler() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}
