//! Le stockage CASC d'un jeu Blizzard, en lecture seule.
//!
//! Rien ici n'ecrit dans le dossier du jeu, et rien ne va sur le reseau : le CDN nomme dans
//! `.build.info` n'est jamais contacte, seuls les fichiers deja installes se lisent.

use std::ffi::{CStr, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use crate::stockage::Entree;

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


pub struct Stockage {
    poignee: *mut std::ffi::c_void,
    entrees: Vec<Entree>,
}

// La poignee n'est touchee que par le fil qui la detient ; CascLib n'est pas reentrante sur un
// meme stockage, et rien ici ne la partage.
unsafe impl Send for Stockage {}

impl Stockage {
    /// Ouvre l'installation dont la racine porte `.build.info`.
    pub fn ouvrir(racine: &Path) -> Result<Self, String> {
        let large: Vec<u16> = OsStr::new(racine).encode_wide().chain(Some(0)).collect();
        let poignee = unsafe { casc_ouvrir(large.as_ptr()) };
        if poignee.is_null() {
            return Err(format!(
                "stockage illisible : {} (ni CASC, ni installation complete ?)",
                racine.display()
            ));
        }
        Ok(Stockage { poignee, entrees: Vec::new() })
    }

    /// Le nom de code du produit : `s2` pour StarCraft II, `fenris` pour Diablo IV.
    pub fn produit(&self) -> String {
        unsafe { CStr::from_ptr(casc_produit(self.poignee)).to_string_lossy().into_owned() }
    }

    /// L'index de tout ce que le stockage nomme. Etabli au premier appel, garde ensuite : le
    /// parcours coute une minute sur StarCraft II et ses 780 000 entrees.
    pub fn entrees(&mut self) -> &[Entree] {
        if self.entrees.is_empty() {
            let compte = unsafe { casc_recenser(self.poignee) };
            self.entrees.reserve(compte);
            for rang in 0..compte {
                let nom = unsafe { CStr::from_ptr(casc_nom(self.poignee, rang)) };
                self.entrees.push(Entree {
                    nom: nom.to_string_lossy().into_owned(),
                    taille: unsafe { casc_taille(self.poignee, rang) },
                });
            }
        }
        &self.entrees
    }

    /// Lit un fichier en entier, decompresse.
    pub fn lire(&self, nom: &str) -> Result<Vec<u8>, String> {
        let Ok(nom_c) = std::ffi::CString::new(nom) else {
            return Err(format!("nom impossible a passer a CascLib : {nom}"));
        };
        let mut bloc: *mut u8 = std::ptr::null_mut();
        let mut taille: usize = 0;
        let code = unsafe { casc_lire(self.poignee, nom_c.as_ptr(), &mut bloc, &mut taille) };
        if code != 0 || bloc.is_null() {
            return Err(format!("lecture refusee ({code}) : {nom}"));
        }
        let donnees = unsafe { std::slice::from_raw_parts(bloc, taille) }.to_vec();
        unsafe { casc_liberer(bloc) };
        Ok(donnees)
    }
}

impl Drop for Stockage {
    fn drop(&mut self) {
        unsafe { casc_fermer(self.poignee) };
    }
}
