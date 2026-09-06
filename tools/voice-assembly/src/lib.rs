//! Assembler une reference de clonage a partir de repliques d'un jeu.
//!
//! CE CODE VIVAIT EN DEUX EXEMPLAIRES. `pack-builder` le tenait pour les recettes,
//! `assemble-voice` pour la ligne de commande : trois cents lignes ecrites deux fois, qui avaient
//! deja diverge -- l'un rangeait les prises de la plus courte a la plus longue, l'autre
//! l'inverse, et personne n'aurait su dire laquelle des deux etait la bonne. Elles sont ici une
//! fois.
//!
//! LA MATIERE ARRIVE EN MEMOIRE, pas par des chemins : une recette lit dans une archive de jeu,
//! la ligne de commande lit sur le disque, et seule la seconde a des fichiers a nommer. Ce qui
//! evite aussi d'avoir a expliquer pourquoi trois cents `.ogg` d'un jeu commercial trainent
//! quelque part.

mod assemble;
mod decode;
mod select;

pub use assemble::{Assembled, Settings, assemble};
pub use decode::{Take, decode};
