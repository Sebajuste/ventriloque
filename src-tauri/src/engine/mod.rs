// Le moteur de parole : PocketTTS, lance a cote, pilote en JSON sur un port local.
//
// UN SEUL DOSSIER DE MODELES SERT LES DEUX PALIERS. Un nom de voix qui finit par `.wav` est une
// reference clonee, cherchee dans le dossier des voix ; tout autre nom est une voix de catalogue,
// lue dans `<models>\catalogue\<nom>.kv`. Le pack libre ne convient pas ici : il n'a pas
// d'encodeur Mimi du tout et le binaire refuse de demarrer sans.

mod api;
mod catalog;
mod port;
mod process;

pub use catalog::{catalog_voices, cloned_voices};
pub use port::free_port;
pub use process::{Engine, tie_to_process_lifetime};

/// La frequence a laquelle le moteur rend son son. Celle de Mimi, pas un choix.
///
/// Elle vit ici et plus dans la forge : la forge ne touche plus a la frequence des references,
/// c'est le moteur qui les ramene lui-meme. Seule la SORTIE est a 24 kHz.
pub const SAMPLE_RATE: u32 = 24_000;
