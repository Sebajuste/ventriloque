// Un zip fabrique a la volee, pour les tests d'installation et de retrait.
//
// Trois modules en ont besoin, et chacun l'ecrivait chez lui avant.

use std::io::Write;
use std::path::PathBuf;

use crate::testing::TempDir;

/// Ecrit un zip dans `dir` et rend son chemin. Les entrees sont donnees `(nom, contenu)`.
pub fn zip_with(dir: &TempDir, name: &str, entries: &[(&str, &str)]) -> PathBuf {
    let path = dir.child(name);
    let mut writer = zip::ZipWriter::new(std::fs::File::create(&path).unwrap());
    let options: zip::write::FileOptions<()> = zip::write::FileOptions::default();
    for (entry, content) in entries {
        writer.start_file(*entry, options).unwrap();
        writer.write_all(content.as_bytes()).unwrap();
    }
    writer.finish().unwrap();
    path
}
