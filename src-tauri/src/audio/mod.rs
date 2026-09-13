// La sortie son : un fil dedie qui possede le peripherique, et une boite aux lettres.
//
// POURQUOI UN FIL. Le flux cpal n'est pas `Send` sous Windows -- il ne peut pas vivre dans
// l'etat partage de Tauri. Plutot que de le contourner, on lui donne un fil a lui : il ouvre le
// peripherique, garde le lecteur, et lit des ordres sur un canal. Les commandes de l'interface
// deposent un ordre et rendent la main tout de suite.
//
// CHOISIR LE PERIPHERIQUE COMPTE. Une partie se joue parfois en visio : la voix doit pouvoir
// partir dans un cable virtuel plutot que dans les enceintes du salon. C'est la seule raison
// pour laquelle la lecture est ici et pas dans la fenetre web.

mod devices;
mod output;
mod progress;
mod stream;
mod takes;

pub use devices::{default_device, device_names};
pub use output::{Output, OutputCommand};
pub use progress::PlaybackProgress;
pub use stream::{SpeechChannel, speech_channel};
pub use takes::{Said, Takes};
