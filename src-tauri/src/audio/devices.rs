// Les sorties audio de la machine, nommees et rangees.
//
// L'ENUMERATION PASSE PAR CPAL DIRECTEMENT : le module `speakers` de rodio, plus joli, est
// derriere sa feature `experimental`, et on ne batit pas une seance de jeu sur une API annoncee
// comme mouvante.
//
// LE RANG FAIT OFFICE D'IDENTIFIANT. La fenetre envoie un entier, pas un nom : deux casques du
// meme modele portent le meme nom, et l'identifiant de cpal ne s'exporte pas en JSON.

use anyhow::{Result, anyhow};
use rodio::Player;
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::stream::DeviceSinkBuilder;

/// Les peripheriques de sortie, dans l'ordre ou cpal les donne -- celui des rangs.
fn devices() -> Vec<rodio::cpal::Device> {
    rodio::cpal::default_host().output_devices().map(|d| d.collect()).unwrap_or_default()
}

fn name_of(device: &rodio::cpal::Device) -> String {
    device
        .description()
        .map(|d| d.name().to_string())
        .unwrap_or_else(|_| "peripherique sans nom".into())
}

/// Ce que la fenetre affiche dans son selecteur de sortie.
pub fn device_names() -> Vec<String> {
    devices().iter().map(name_of).collect()
}

/// Le rang du peripherique par defaut du systeme, ou zero si le systeme n'en designe aucun.
pub fn default_device() -> usize {
    let default = rodio::cpal::default_host().default_output_device().map(|d| d.id());
    devices().iter().position(|d| Some(d.id()) == default).unwrap_or(0)
}

/// Ouvre le peripherique de rang `index` et rend un lecteur branche dessus.
pub fn open(index: usize) -> Result<Player> {
    let available = devices();
    let device = available
        .get(index)
        .cloned()
        .ok_or_else(|| anyhow!("aucun peripherique de sortie au rang {index}"))?;
    let mixer = DeviceSinkBuilder::from_device(device)?.open_stream()?;
    let player = Player::connect_new(mixer.mixer());
    // Le melangeur tient le flux cpal ouvert : le laisser tomber couperait le son. Il vit aussi
    // longtemps que le fil, et le fil vit aussi longtemps que l'application.
    std::mem::forget(mixer);
    Ok(player)
}
