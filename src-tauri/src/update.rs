// La mise a jour : la chercher, la poser, relancer.
//
// LA CONFIANCE TIENT A UNE SIGNATURE, et a elle seule. Le paquet telecharge est verifie contre
// la cle publique inscrite dans `tauri.conf.json` ; sans elle l'application refuse tout. C'est
// ce qui empeche un tiers qui intercepterait la connexion de servir son propre installateur.
//
// LA FENETRE SONDE, ON N'EMET RIEN. C'est la regle du projet -- voir `ipc::packs` : la commande
// ne rend la main qu'a la fin, et l'avancement se lit a intervalle. Un evenement par morceau
// recu saturerait le pont pour dessiner cent positions de barre.
//
// LE MOTEUR EST TUE AVANT L'INSTALLATEUR, et c'est le seul point delicat du fichier. Voir
// `install`.

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tauri::{AppHandle, Manager};
use tauri_plugin_updater::{Update, UpdaterExt};

/// Ce qu'il faut a la fenetre pour PROPOSER la mise a jour au lieu de l'imposer : les deux
/// numeros de version, et les notes de publication si la release en porte.
pub struct Found {
    pub version: String,
    pub current: String,
    pub notes: String,
}

/// Ou en est le telechargement. `total` a zero veut dire « en cours, sans taille annoncee » :
/// la barre se rabat alors sur les octets recus, qui disent au moins que ca avance.
pub struct Progress {
    pub active: bool,
    pub downloaded: u64,
    pub total: u64,
}

/// La mise a jour reperee, gardee entre la recherche et l'installation.
///
/// Telecharger demande l'objet rendu par `check`, pas seulement son numero de version : le
/// jeter puis rechercher a nouveau au moment d'installer ferait un second aller-retour reseau,
/// et rien ne garantit que la release repondrait la meme chose.
#[derive(Default)]
pub struct Updates {
    pending: Mutex<Option<Update>>,
    downloaded: AtomicU64,
    total: AtomicU64,
    active: AtomicBool,
}

impl Updates {
    /// Interroge la release publiee. `None` quand la version installee est deja la derniere.
    pub async fn look(&self, app: &AppHandle) -> Result<Option<Found>, String> {
        let found =
            app.updater().map_err(|e| e.to_string())?.check().await.map_err(|e| e.to_string())?;

        let Some(update) = found else {
            // Plus rien a installer : l'emplacement ne doit pas garder une mise a jour d'avant.
            self.forget();
            return Ok(None);
        };

        let seen = Found {
            version: update.version.clone(),
            current: update.current_version.clone(),
            notes: update.body.clone().unwrap_or_default(),
        };
        if let Ok(mut slot) = self.pending.lock() {
            *slot = Some(update);
        }
        Ok(Some(seen))
    }

    /// Telecharge, installe, relance.
    ///
    /// LE MOTEUR EST TUE D'ABORD, et rien ne le ferait a notre place. Tauri termine le processus
    /// sans derouler la pile : le `Drop` qui tue le fils ne part pas, et le moteur survivrait a
    /// la sortie avec un demi-gigaoctet de modeles en memoire et son port pris -- exactement le
    /// piege documente dans `AppState::shutdown`. La relance qui suit retomberait alors sur un
    /// port occupe par un moteur orphelin que plus personne ne pilote.
    pub async fn install(&self, app: &AppHandle) -> Result<(), String> {
        let update = self
            .pending
            .lock()
            .map_err(|_| "etat de mise a jour inutilisable")?
            .take()
            .ok_or("aucune mise a jour en attente")?;

        if let Some(state) = app.try_state::<crate::app::AppState>() {
            state.shutdown();
        }

        self.downloaded.store(0, Ordering::Relaxed);
        self.total.store(0, Ordering::Relaxed);
        self.active.store(true, Ordering::Relaxed);

        let outcome = update
            .download_and_install(
                |chunk, total| {
                    self.downloaded.fetch_add(chunk as u64, Ordering::Relaxed);
                    self.total.store(total.unwrap_or(0), Ordering::Relaxed);
                },
                || {},
            )
            .await;

        // Quoi qu'il arrive : la fenetre ne doit pas rester sur une barre qui n'avance plus.
        self.active.store(false, Ordering::Relaxed);
        outcome.map_err(|e| e.to_string())?;

        app.restart()
    }

    pub fn progress(&self) -> Progress {
        Progress {
            active: self.active.load(Ordering::Relaxed),
            downloaded: self.downloaded.load(Ordering::Relaxed),
            total: self.total.load(Ordering::Relaxed),
        }
    }

    fn forget(&self) {
        if let Ok(mut slot) = self.pending.lock() {
            slot.take();
        }
    }
}
