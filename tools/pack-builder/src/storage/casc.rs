//! Le stockage CASC d'un jeu Blizzard, en lecture seule.
//!
//! Rien ici n'ecrit dans le dossier du jeu, et rien ne va sur le reseau : le CDN nomme dans
//! `.build.info` n'est jamais contacte, seuls les fichiers deja installes se lisent.
//!
//! Le travail est fait par CascLib, du C++ compile a cote (`casc_shim.cpp`) : c'est aussi la
//! raison pour laquelle ce binaire est separe de l'application.

use std::ffi::{CStr, CString, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use super::Entry;

unsafe extern "C" {
    fn casc_ouvrir(racine: *const u16) -> *mut std::ffi::c_void;
    fn casc_fermer(poignee: *mut std::ffi::c_void);
    fn casc_produit(poignee: *mut std::ffi::c_void) -> *const i8;
    fn casc_recenser(poignee: *mut std::ffi::c_void) -> usize;
    fn casc_nom(poignee: *mut std::ffi::c_void, rang: usize) -> *const i8;
    fn casc_taille(poignee: *mut std::ffi::c_void, rang: usize) -> u64;
    fn casc_lire(
        poignee: *mut std::ffi::c_void,
        nom: *const i8,
        sortie: *mut *mut u8,
        taille: *mut usize,
    ) -> i32;
    fn casc_liberer(bloc: *mut u8);
}

pub struct CascStorage {
    handle: *mut std::ffi::c_void,
    entries: Vec<Entry>,
}

// La poignee n'est touchee que par le fil qui la detient ; CascLib n'est pas reentrante sur un
// meme stockage, et rien ici ne la partage.
unsafe impl Send for CascStorage {}

impl CascStorage {
    /// Ouvre l'installation dont la racine porte `.build.info`.
    pub fn open(root: &Path) -> Result<Self, String> {
        let wide: Vec<u16> = OsStr::new(root).encode_wide().chain(Some(0)).collect();
        let handle = unsafe { casc_ouvrir(wide.as_ptr()) };
        if handle.is_null() {
            return Err(format!(
                "stockage illisible : {} (ni CASC, ni installation complete ?)",
                root.display()
            ));
        }
        Ok(CascStorage { handle, entries: Vec::new() })
    }

    /// Le nom de code du produit : `s2` pour StarCraft II, `fenris` pour Diablo IV.
    ///
    /// ATTENTION, CE N'EST PAS CELUI DE `.build.info`, qui dit `sc2` pour le meme jeu. Le
    /// manifeste d'un paquet declare celui de `.build.info` -- le seul lisible sans ouvrir le
    /// stockage -- et le script verifie celui-ci une fois l'archive ouverte.
    pub fn product(&self) -> String {
        unsafe { CStr::from_ptr(casc_produit(self.handle)).to_string_lossy().into_owned() }
    }

    pub fn entries(&mut self) -> &[Entry] {
        if self.entries.is_empty() {
            let count = unsafe { casc_recenser(self.handle) };
            self.entries.reserve(count);
            for index in 0..count {
                let name = unsafe { CStr::from_ptr(casc_nom(self.handle, index)) };
                self.entries.push(Entry {
                    name: name.to_string_lossy().into_owned(),
                    size: unsafe { casc_taille(self.handle, index) },
                });
            }
        }
        &self.entries
    }

    /// Lit un fichier en entier, decompresse.
    pub fn read(&self, name: &str) -> Result<Vec<u8>, String> {
        let Ok(c_name) = CString::new(name) else {
            return Err(format!("nom impossible a passer a CascLib : {name}"));
        };
        let mut block: *mut u8 = std::ptr::null_mut();
        let mut size: usize = 0;
        let code = unsafe { casc_lire(self.handle, c_name.as_ptr(), &mut block, &mut size) };
        if code != 0 || block.is_null() {
            return Err(format!("lecture refusee ({code}) : {name}"));
        }
        let data = unsafe { std::slice::from_raw_parts(block, size) }.to_vec();
        unsafe { casc_liberer(block) };
        Ok(data)
    }
}

impl Drop for CascStorage {
    fn drop(&mut self) {
        unsafe { casc_fermer(self.handle) };
    }
}
