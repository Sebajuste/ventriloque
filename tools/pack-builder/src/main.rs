// pack-builder -- execute le script d'un paquet Ventriloque sur une copie installee d'un jeu.
//
// CE BINAIRE N'EST PAS VENTRILOQUE, et c'est le point du dessin. Le C++ de CascLib et le script
// du paquet -- deux codes qui ne viennent pas de l'application -- tournent ici, dans un
// processus separe que Ventriloque lance sous job object et peut tuer d'un geste. L'application
// ne charge jamais ni l'un ni l'autre.
//
// La sortie porte `voices\*.wav` et `characters\*.json`, que Ventriloque moissonne ensuite avec
// la meme rigueur qu'un zip. Rien n'est ecrit ailleurs, et rien n'est ecrit dans le dossier du
// jeu.
//
// Ce fichier n'aiguille que les trois modes ; le travail est dans les modules.

mod cli;
mod script;
mod storage;

use std::path::Path;

use storage::Storage;

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = cli::parse(std::env::args().skip(1).collect())?;

    // UNE ARCHIVE CYBERPUNK NE CONNAIT QUE DES HACHAGES. Ecrire une recette pour ce jeu demande
    // donc de traduire un chemin depot en son FNV1a64 ; le faire ici evite d'avoir a refaire
    // l'algorithme, et de se tromper sur la casse.
    if let Some(path) = args.hash {
        println!("{:016x}\t{path}", storage::path_hash(&path));
        return Ok(());
    }

    let mut store = storage::open(&args.game)?;
    println!("stockage ouvert : produit {}", store.product());

    if let Some(pattern) = args.list {
        return list(&mut store, &pattern);
    }

    let path = args.script.expect("verifie par cli::parse");
    let source = std::fs::read_to_string(&path)
        .map_err(|e| format!("script illisible ({}) : {e}", path.display()))?;

    build(store, &source, &args.out_dir)
}

/// Fouille a la main : pas de script, pas de sortie, juste ce que le stockage nomme.
fn list(store: &mut Storage, pattern: &str) -> Result<(), String> {
    let total = store.entries().len();
    let wanted = pattern.to_lowercase().replace('\\', "/");
    let mut kept = 0usize;
    for entry in store.entries() {
        if entry.name.to_lowercase().replace('\\', "/").contains(&wanted) {
            println!("{}\t{}", entry.size, entry.name);
            kept += 1;
        }
    }
    eprintln!("{total} entrees parcourues, {kept} retenues.");
    Ok(())
}

/// Le stockage est pris PAR VALEUR : le script le tient pendant toute son execution, et
/// personne d'autre ne doit pouvoir le lire entre-temps.
fn build(mut store: Storage, source: &str, out_dir: &Path) -> Result<(), String> {
    // Le recensement coute une minute sur StarCraft II : on le paie avant de dire
    // « fabrication », pour que la premiere ligne du journal ne soit pas suivie d'un long
    // silence.
    println!("recensement : {} entrees", store.entries().len());
    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("dossier de travail ({}) : {e}", out_dir.display()))?;

    let harvest = script::run(source, store, out_dir)?;

    if harvest.voices.is_empty() && harvest.characters.is_empty() {
        return Err("le script n'a produit ni voix ni fiche".into());
    }
    println!(
        "fabrique : {} voix, {} fiches",
        harvest.voices.len(),
        harvest.characters.len()
    );
    Ok(())
}
