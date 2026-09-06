// Fabriquer une reference de voix : d'un enregistrement quelconque au `.wav` que le moteur clone.
//
// La chaine est courte et son ordre compte :
//
//   decoder -> mono -> extraire -> decaler (hauteur, formants) -> normaliser -> ecrire
//
// Le decalage vient AVANT la normalisation parce qu'il change les cretes ; normaliser d'abord
// laisserait la reference sous ou au-dessus du niveau vise. Et l'extrait est pris avant le
// decalage : traiter trente minutes pour en garder trente secondes coute trente minutes.

mod assemble;
mod buffer;
mod decode;
mod recipe;
mod resample;
mod stretch;
mod wav;

pub use assemble::assemble;
pub use recipe::{Recipe, render};
pub use wav::write_wav;
