// LE CONTRAT AVEC L'INTERFACE, DECLARE UNE FOIS.
//
// La liste ci-dessous ne sert pas qu'a brancher les commandes : le test `bindings` plus bas la
// traverse pour produire `ui/src/ipc/bindings.ts`, ou les noms des commandes, ceux de leurs
// arguments et la forme de leurs retours deviennent des types TypeScript. Un champ renomme ici
// casse desormais la compilation de l'interface, la ou il cassait la seance.
//
// UN FICHIER PAR DOMAINE. Les commandes sont rangees comme la fenetre les appelle -- l'etat, la
// parole, les voix, les fiches, les paquets -- et rien d'autre ne vit dans ces fichiers : le
// travail est dans les modules qu'ils appellent.

mod characters;
mod packs;
mod snapshot;
mod speech;
mod voices;

pub fn contract() -> tauri_specta::Builder<tauri::Wry> {
    tauri_specta::Builder::<tauri::Wry>::new().commands(tauri_specta::collect_commands![
        snapshot::snapshot,
        speech::speak,
        speech::silence,
        speech::speech_progress,
        speech::pause,
        speech::resume,
        speech::select_device,
        speech::warm_up,
        voices::forge_voice,
        voices::pick_audio_files,
        characters::write_character,
        characters::delete_character,
        packs::install_pack,
        packs::build_pack,
        packs::uninstall_pack,
        packs::pack_progress
    ])
}

// Le contrat, ecrit.
//
// C'EST UN TEST ET PAS UN SCRIPT, parce que `cargo test` tourne deja dans `outils/preparer.ps1`,
// avant la compilation du binaire : le lien ne peut pas etre oublie. Le fichier produit est
// versionne, et l'integration echoue si `git diff` le trouve modifie -- ce qui veut alors dire
// que quelqu'un a change une commande sans regenerer.
#[cfg(test)]
mod bindings {
    const PREAMBLE: &str = "// Ecrit par `cargo test` depuis les commandes de `src-tauri`.
// NE PAS MODIFIER A LA MAIN : la prochaine execution ecrasera tout.
//
// Les enveloppes lisibles, avec leurs commentaires, sont dans `client.ts` -- ici il n'y a que la
// forme exacte de ce que Rust expose.";

    #[test]
    fn ecrire() {
        super::contract()
            .export(
                specta_typescript::Typescript::default().header(PREAMBLE),
                "../ui/src/ipc/bindings.ts",
            )
            .expect("le lien vers l'interface n'a pas pu etre ecrit");
    }
}
