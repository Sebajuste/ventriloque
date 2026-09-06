// Un dossier jetable, pour les tests, sans dependance de plus.
//
// `tempfile` aurait fait l'affaire ; il aurait aussi ajoute une dependance de compilation a un
// projet qui n'en demande qu'un dossier avec un nom unique et un `Drop` qui l'efface. Trois
// modules l'ecrivaient chacun de leur cote avant que ce fichier existe.

use std::path::{Path, PathBuf};

pub struct TempDir(PathBuf);

impl TempDir {
    /// Un dossier vide, cree tout de suite, nomme d'apres `label` et l'instant present.
    pub fn new(label: &str) -> Self {
        let path = std::env::temp_dir().join(format!("ventriloque-test-{}-{label}", unique()));
        std::fs::create_dir_all(&path).expect("dossier de test");
        Self(path)
    }

    /// Un chemin dans le dossier temporaire, sans creer de fichier : de quoi nommer un zip a
    /// ecrire, par exemple.
    pub fn child(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Assez unique pour deux tests qui tournent en parallele : l'horloge en nanosecondes, et le
/// numero du fil pour le cas ou deux appels tombent sur la meme graduation.
fn unique() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{nanos}-{:?}", std::thread::current().id())
        .replace(['(', ')', ' '], "")
        .replace("ThreadId", "t")
}
