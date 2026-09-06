// Le fil qui possede le peripherique, et les ordres qu'on lui depose.
//
// Tout ce qui touche au lecteur passe par ici, et rien d'autre ne le touche : c'est ce qui rend
// la lecture sure a piloter depuis les commandes de Tauri, qui vivent sur d'autres fils.

use std::sync::mpsc::{Sender, channel};

use super::devices;
use super::stream::SpeechStream;

pub enum OutputCommand {
    /// Une replique qui arrive au fil de l'eau, pendant que le moteur la fabrique.
    Play(SpeechStream),
    /// Le silence tout de suite : ce qui joue s'arrete et ce qui attendait est jete.
    Stop,
    /// Suspendre et reprendre la ou l'on en etait. Le calcul, lui, continue : il remplit le
    /// tampon de la replique en cours pendant la pause, et rien ne se perd.
    Pause,
    Resume,
    /// Rouvrir sur un autre peripherique, par son rang dans `device_names()`.
    SelectDevice(usize),
}

pub struct Output {
    commands: Sender<OutputCommand>,
}

impl Output {
    pub fn start() -> Self {
        let (commands, inbox) = channel::<OutputCommand>();
        std::thread::spawn(move || {
            let mut player = devices::open(devices::default_device()).ok();
            while let Ok(command) = inbox.recv() {
                match command {
                    OutputCommand::SelectDevice(index) => match devices::open(index) {
                        Ok(fresh) => player = Some(fresh),
                        Err(e) => eprintln!("peripherique refuse : {e}"),
                    },
                    OutputCommand::Stop => {
                        if let Some(p) = &player {
                            p.clear();
                            // `clear` laisse le lecteur en pause : sans ce `play`, la replique
                            // suivante serait mise en file et n'en sortirait jamais.
                            p.play();
                        }
                    }
                    OutputCommand::Pause => {
                        if let Some(p) = &player {
                            p.pause();
                        }
                    }
                    OutputCommand::Resume => {
                        if let Some(p) = &player {
                            p.play();
                        }
                    }
                    // `append` met en file : une replique envoyee pendant qu'une autre joue
                    // s'enchaine au lieu de la couper, ce qui est le bon comportement pour un
                    // personnage qui parle en plusieurs phrases.
                    OutputCommand::Play(stream) => {
                        if let Some(p) = &player {
                            p.append(stream);
                            p.play();
                        }
                    }
                }
            }
        });
        Self { commands }
    }

    pub fn send(&self, command: OutputCommand) {
        let _ = self.commands.send(command);
    }

    /// De quoi parler depuis un autre fil : la synthese vit dans un fil a elle et doit pouvoir
    /// deposer sa replique sans passer par l'etat de l'application.
    pub fn sender(&self) -> Sender<OutputCommand> {
        self.commands.clone()
    }
}
