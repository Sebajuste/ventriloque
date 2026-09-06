// Les paquets : des donnees qui arrivent par un zip, installees depuis la fenetre.
//
// CE QU'EST UN PAQUET. Un zip qui porte un `pack.json` a sa racine et des dossiers dont les noms
// sont ceux de Ventriloque :
//
//   pack.json      le manifeste -- nom, version, description
//   voices/        des references .wav, pretes a etre clonees
//   characters/    des fiches de personnages
//   models/        le moteur de parole, un demi-gigaoctet, qui ne peut pas etre livre autrement
//   recipes/       un script d'extraction, pour un paquet qui ne porte pas ses voix
//
// UN PAQUET-RECETTE NE PORTE AUCUN SON. Il porte la connaissance de ou chercher dans un jeu
// installe, et fabrique les voix sur la machine du joueur, depuis sa propre copie. C'est ce qui
// le rend partageable la ou un paquet de voix ne l'est pas : le script est du texte, il ne
// contient pas une seconde d'enregistrement d'acteur.
//
// RIEN NE S'EXECUTE A L'INSTALLATION. Le script est pose comme un fichier de donnees et n'est
// lance que par un geste explicite, depuis l'onglet Paquets -- et jamais dans ce processus :
// voir `build.rs`.
//
// LE MANIFESTE EST OBLIGATOIRE, et c'est une protection plutot qu'une formalite : sans lui, un
// zip quelconque tombe sur le bouton « installer » repandrait son contenu dans les dossiers de
// l'application. Un zip sans `pack.json` est refuse avant d'etre ouvert plus avant.
//
// AUCUNE ENTREE NE SORT DE LA RACINE. Un zip peut nommer `..\..\Windows\System32\...` ou un
// chemin absolu -- c'est une attaque connue et elle est ancienne. Voir `entry.rs`.

mod build;
mod entry;
mod install;
mod job;
mod legacy;
mod manifest;
mod store;
#[cfg(test)]
mod testing;
mod uninstall;

pub use build::build;
pub use install::install;
pub use job::Job;
pub use manifest::Manifest;
pub use store::installed;
pub use uninstall::uninstall;
