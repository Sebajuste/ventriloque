//! Wwise Vorbis -> Ogg Vorbis, pour que symphonia puisse decoder le doublage de Cyberpunk.
//!
//! POURQUOI CE FICHIER EXISTE. Audiokinetic n'ecrit pas de l'Ogg : elle prend un flux Vorbis et
//! lui retire tout ce que le jeu n'a pas besoin de relire — les pages Ogg, les en-tetes
//! d'identification et de commentaire, et surtout les codebooks, remplaces par des numeros dans
//! une table que le decodeur du jeu porte deja. Ce qui reste est du Vorbis valide auquel il
//! manque son mode d'emploi. Le rendre lisible consiste a le remettre, bit pour bit.
//!
//! C'est un portage de `ww2ogg` (Adam Gashlin, BSD 3 clauses), reduit a ce que le doublage de
//! Cyberpunk 2077 utilise reellement : en-tete `fmt` de 0x42 octets sans chunk `vorb`, paquets a
//! deux octets sans granule, codebooks externes. Les autres variantes — la triade d'en-tetes des
//! vieux jeux, les paquets a huit octets, les codebooks en ligne — ne sont pas portees : elles
//! seraient du code non exerce, et `refuser` dit clairement ce qui n'est pas traite.
//!
//! CE QUI A ETE MESURE, le 2026-09-06, sur `base\localization\fr-fr\vo\*.wem` :
//! `fmt` 0x42, `tag` 0xFFFF, mono 48 kHz, `mod_signal` 0xDD — donc `mod_packets` — blocs 2^8 et
//! 2^11, et 42 codebooks nommes par identifiants de 10 bits (50 a 254) dans la bibliotheque
//! `packed_codebooks_aoTuV_603.bin`. Rien ici ne devine : tout se relit dans le fichier.

/// La bibliotheque de codebooks d'aoTuV, vendorisee depuis ww2ogg. Voir `build.rs`.
const CODEBOOKS: &[u8] = include_bytes!("../vendor/ww2ogg/packed_codebooks_aoTuV_603.bin");

/// Lit des bits dans l'ordre ou Vorbis les ecrit : poids faible d'abord, octet par octet.
struct Lecteur<'a> {
    donnees: &'a [u8],
    bit: usize,
}

impl<'a> Lecteur<'a> {
    fn nouveau(donnees: &'a [u8]) -> Self {
        Lecteur { donnees, bit: 0 }
    }

    fn lire(&mut self, combien: u32) -> Result<u32, String> {
        let mut valeur = 0u32;
        for rang in 0..combien {
            let octet = self
                .donnees
                .get(self.bit >> 3)
                .ok_or("flux Wwise tronque")?;
            if (octet >> (self.bit & 7)) & 1 == 1 {
                valeur |= 1 << rang;
            }
            self.bit += 1;
        }
        Ok(valeur)
    }

    fn bits_lus(&self) -> usize {
        self.bit
    }
}

/// Ecrit des bits et les decoupe en pages Ogg. Un seul flux, numero de serie 1, comme ww2ogg.
struct Flux {
    sortie: Vec<u8>,
    tampon: u8,
    bits: u32,
    charge: Vec<u8>,
    premiere: bool,
    suite: bool,
    granule: u32,
    rang: u32,
}

const ENTETE: usize = 27;
const SEGMENTS_MAX: usize = 255;
const SEGMENT: usize = 255;

impl Flux {
    fn nouveau() -> Self {
        Flux {
            sortie: Vec::new(),
            tampon: 0,
            bits: 0,
            charge: Vec::new(),
            premiere: true,
            suite: false,
            granule: 0,
            rang: 0,
        }
    }

    fn bit(&mut self, pose: bool) {
        if pose {
            self.tampon |= 1 << self.bits;
        }
        self.bits += 1;
        if self.bits == 8 {
            self.vider_bits();
        }
    }

    fn ecrire(&mut self, valeur: u32, combien: u32) {
        for rang in 0..combien {
            self.bit((valeur >> rang) & 1 == 1);
        }
    }

    fn octets(&mut self, valeurs: &[u8]) {
        for v in valeurs {
            self.ecrire(*v as u32, 8);
        }
    }

    /// L'en-tete commun aux trois paquets de configuration : le type, puis « vorbis ».
    fn entete_vorbis(&mut self, type_paquet: u8) {
        self.ecrire(type_paquet as u32, 8);
        self.octets(b"vorbis");
    }

    fn vider_bits(&mut self) {
        if self.bits != 0 {
            self.charge.push(self.tampon);
            self.bits = 0;
            self.tampon = 0;
        }
    }

    fn vider_page(&mut self, suite_ensuite: bool, derniere: bool) {
        if self.charge.len() != SEGMENT * SEGMENTS_MAX {
            self.vider_bits();
        }
        if self.charge.is_empty() {
            return;
        }

        // Arrondi au superieur volontaire : une charge multiple de 255 finit par un segment
        // vide, qui est ce qui dit « le paquet s'arrete ici ».
        let mut segments = (self.charge.len() + SEGMENT) / SEGMENT;
        if segments == SEGMENTS_MAX + 1 {
            segments = SEGMENTS_MAX;
        }

        let mut page = vec![0u8; ENTETE + segments + self.charge.len()];
        page[0..4].copy_from_slice(b"OggS");
        page[4] = 0;
        page[5] = (self.suite as u8) | ((self.premiere as u8) << 1) | ((derniere as u8) << 2);
        page[6..10].copy_from_slice(&self.granule.to_le_bytes());
        // Le granule de Wwise tient sur 32 bits ; `0xFFFFFFFF` veut dire « inconnu », et Ogg
        // l'ecrit sur ses 64 bits.
        let haut: u32 = if self.granule == u32::MAX { u32::MAX } else { 0 };
        page[10..14].copy_from_slice(&haut.to_le_bytes());
        page[14..18].copy_from_slice(&1u32.to_le_bytes());
        page[18..22].copy_from_slice(&self.rang.to_le_bytes());
        page[22..26].copy_from_slice(&0u32.to_le_bytes());
        page[26] = segments as u8;

        let mut reste = self.charge.len();
        for i in 0..segments {
            let pris = reste.min(SEGMENT);
            page[ENTETE + i] = pris as u8;
            reste -= pris;
        }
        page[ENTETE + segments..].copy_from_slice(&self.charge);

        let somme = controle(&page);
        page[22..26].copy_from_slice(&somme.to_le_bytes());

        self.sortie.extend_from_slice(&page);
        self.rang += 1;
        self.premiere = false;
        self.suite = suite_ensuite;
        self.charge.clear();
    }
}

/// La somme de controle d'une page Ogg : CRC-32 non reflechi, polynome 0x04C11DB7, sans
/// inversion ni finale. Ce n'est pas le CRC-32 de zlib, et s'y tromper ne se voit qu'a la
/// lecture, quand le decodeur jette la page sans rien dire.
fn controle(page: &[u8]) -> u32 {
    let mut somme = 0u32;
    for octet in page {
        somme ^= (*octet as u32) << 24;
        for _ in 0..8 {
            somme = if somme & 0x8000_0000 != 0 {
                (somme << 1) ^ 0x04C1_1DB7
            } else {
                somme << 1
            };
        }
    }
    somme
}

fn ilog(mut v: u32) -> u32 {
    let mut n = 0;
    while v != 0 {
        n += 1;
        v >>= 1;
    }
    n
}

/// Le nombre de valeurs quantifiees d'une table de correspondance de type 1. Repris tel quel de
/// Vorbis : c'est une recherche, pas une formule fermee.
fn quantvals(entrees: u32, dimensions: u32) -> u32 {
    if dimensions == 0 {
        return 0;
    }
    let bits = ilog(entrees);
    let mut vals = entrees >> (((bits - 1) * (dimensions - 1)) / dimensions);
    loop {
        let mut acc = 1u64;
        let mut acc1 = 1u64;
        for _ in 0..dimensions {
            acc = acc.saturating_mul(vals as u64);
            acc1 = acc1.saturating_mul(vals as u64 + 1);
        }
        if acc <= entrees as u64 && acc1 > entrees as u64 {
            return vals;
        }
        if acc > entrees as u64 {
            vals -= 1;
        } else {
            vals += 1;
        }
    }
}

/// Un codebook de la bibliotheque, par son rang.
fn codebook(rang: u32) -> Result<&'static [u8], String> {
    let taille = CODEBOOKS.len();
    let depart = u32::from_le_bytes(CODEBOOKS[taille - 4..].try_into().unwrap()) as usize;
    let compte = (taille - depart) / 4 - 1;
    if rang as usize >= compte {
        return Err(format!("codebook {rang} hors de la bibliotheque ({compte})"));
    }
    let borne = |i: usize| {
        u32::from_le_bytes(CODEBOOKS[depart + i * 4..depart + i * 4 + 4].try_into().unwrap())
            as usize
    };
    Ok(&CODEBOOKS[borne(rang as usize)..borne(rang as usize + 1)])
}

/// Reconstitue un codebook complet a partir de la forme abregee que Wwise range dans sa
/// bibliotheque : les champs y sont ecrits sur moins de bits, et la signature manque.
fn rebatir_codebook(l: &mut Lecteur, f: &mut Flux) -> Result<(), String> {
    let dimensions = l.lire(4)?;
    let entrees = l.lire(14)?;

    f.ecrire(0x56_4342, 24); // « BCV »
    f.ecrire(dimensions, 16);
    f.ecrire(entrees, 24);

    let ordonne = l.lire(1)?;
    f.ecrire(ordonne, 1);
    if ordonne != 0 {
        let longueur = l.lire(5)?;
        f.ecrire(longueur, 5);
        let mut courant = 0u32;
        while courant < entrees {
            let combien = ilog(entrees - courant);
            let nombre = l.lire(combien)?;
            f.ecrire(nombre, combien);
            courant += nombre;
        }
        if courant > entrees {
            return Err("codebook : compte d'entrees hors bornes".into());
        }
    } else {
        let taille_longueur = l.lire(3)?;
        let creux = l.lire(1)?;
        if taille_longueur == 0 || taille_longueur > 5 {
            return Err("codebook : longueur de mot absurde".into());
        }
        f.ecrire(creux, 1);
        for _ in 0..entrees {
            let present = if creux != 0 {
                let p = l.lire(1)?;
                f.ecrire(p, 1);
                p != 0
            } else {
                true
            };
            if present {
                // Wwise ecrit la longueur sur le minimum de bits ; Vorbis en veut cinq.
                let longueur = l.lire(taille_longueur)?;
                f.ecrire(longueur, 5);
            }
        }
    }

    let type_table = l.lire(1)?;
    f.ecrire(type_table, 4);
    if type_table == 1 {
        let min = l.lire(32)?;
        let max = l.lire(32)?;
        let taille_valeur = l.lire(4)?;
        let sequence = l.lire(1)?;
        f.ecrire(min, 32);
        f.ecrire(max, 32);
        f.ecrire(taille_valeur, 4);
        f.ecrire(sequence, 1);
        for _ in 0..quantvals(entrees, dimensions) {
            let v = l.lire(taille_valeur + 1)?;
            f.ecrire(v, taille_valeur + 1);
        }
    } else if type_table != 0 {
        return Err(format!("codebook : table de correspondance {type_table} non traitee"));
    }
    Ok(())
}

/// Ce qu'un `.wem` Wwise Vorbis annonce de lui-meme.
struct Entete {
    canaux: u16,
    frequence: u32,
    octets_par_seconde: u32,
    bloc_0: u8,
    bloc_1: u8,
    /// Le premier octet de chaque paquet audio a ete recompose par Wwise et doit l'etre a
    /// l'envers : type de paquet, numero de mode, et les deux bits de fenetre.
    paquets_modifies: bool,
    setup: usize,
    premier_audio: usize,
}

fn lire_u16(d: &[u8], p: usize) -> Result<u16, String> {
    d.get(p..p + 2)
        .map(|o| u16::from_le_bytes(o.try_into().unwrap()))
        .ok_or_else(|| "RIFF tronque".to_string())
}

fn lire_u32(d: &[u8], p: usize) -> Result<u32, String> {
    d.get(p..p + 4)
        .map(|o| u32::from_le_bytes(o.try_into().unwrap()))
        .ok_or_else(|| "RIFF tronque".to_string())
}

/// Dit si ces octets sont du Wwise Vorbis, c'est-a-dire un RIFF WAVE dont le format est 0xFFFF.
/// C'est la donnee qui decide, pas le nom du fichier : les entrees d'une archive Cyberpunk n'ont
/// pas de nom, seulement un hachage.
pub fn est_wwise(donnees: &[u8]) -> bool {
    if donnees.len() < 24 || &donnees[0..4] != b"RIFF" || &donnees[8..12] != b"WAVE" {
        return false;
    }
    let mut p = 12;
    while p + 8 <= donnees.len() {
        let taille = match lire_u32(donnees, p + 4) {
            Ok(t) => t as usize,
            Err(_) => return false,
        };
        if &donnees[p..p + 4] == b"fmt " {
            return lire_u16(donnees, p + 8).map(|t| t == 0xFFFF).unwrap_or(false);
        }
        p += 8 + taille + (taille & 1);
    }
    false
}

fn depouiller(donnees: &[u8]) -> Result<(Entete, &[u8]), String> {
    if donnees.len() < 12 || &donnees[0..4] != b"RIFF" || &donnees[8..12] != b"WAVE" {
        return Err("ce n'est pas un RIFF WAVE".into());
    }

    let mut fmt: Option<&[u8]> = None;
    let mut data: Option<&[u8]> = None;
    let mut p = 12;
    while p + 8 <= donnees.len() {
        let taille = lire_u32(donnees, p + 4)? as usize;
        let corps = donnees
            .get(p + 8..p + 8 + taille)
            .ok_or("chunk RIFF tronque")?;
        match &donnees[p..p + 4] {
            b"fmt " => fmt = Some(corps),
            b"data" => data = Some(corps),
            b"vorb" => {
                return Err("chunk « vorb » separe : variante Wwise ancienne, non traitee".into());
            }
            _ => {}
        }
        p += 8 + taille + (taille & 1);
    }

    let fmt = fmt.ok_or("pas de chunk « fmt »")?;
    let data = data.ok_or("pas de chunk « data »")?;

    // 0x42 est la seule taille ou le bloc `vorb` est fondu dans `fmt`, a l'offset 0x18. Les
    // autres tailles vont avec un chunk `vorb` separe, ecarte plus haut.
    if fmt.len() != 0x42 {
        return Err(format!("« fmt » de {} octets, 0x42 attendu", fmt.len()));
    }
    if lire_u16(fmt, 0)? != 0xFFFF {
        return Err("ce n'est pas du Wwise Vorbis (format != 0xFFFF)".into());
    }

    let vorb = &fmt[0x18..0x42];
    let signal = lire_u32(vorb, 0x04)?;
    Ok((
        Entete {
            canaux: lire_u16(fmt, 2)?,
            frequence: lire_u32(fmt, 4)?,
            octets_par_seconde: lire_u32(fmt, 8)?,
            bloc_0: vorb[0x28],
            bloc_1: vorb[0x29],
            // Les valeurs qui vont avec des paquets intacts ont ete relevees par ww2ogg ; tout
            // le reste demande la recomposition. Cyberpunk dit 0xDD.
            paquets_modifies: !matches!(signal, 0x4A | 0x4B | 0x69 | 0x70),
            setup: lire_u32(vorb, 0x10)? as usize,
            premier_audio: lire_u32(vorb, 0x14)? as usize,
        },
        data,
    ))
}

/// Un paquet Wwise « moderne » : deux octets de taille, pas de granule.
fn paquet(data: &[u8], position: usize) -> Result<(usize, usize), String> {
    let taille = lire_u16(data, position)? as usize;
    Ok((position + 2, taille))
}

/// Convertit un `.wem` Wwise Vorbis en flux Ogg Vorbis complet, pret pour symphonia.
pub fn vers_ogg(donnees: &[u8]) -> Result<Vec<u8>, String> {
    let (e, data) = depouiller(donnees)?;
    let mut f = Flux::nouveau();

    // ---------------------------------------------------------- identification
    f.entete_vorbis(1);
    f.ecrire(0, 32); // version
    f.ecrire(e.canaux as u32, 8);
    f.ecrire(e.frequence, 32);
    f.ecrire(0, 32); // debit maximum
    f.ecrire(e.octets_par_seconde.saturating_mul(8), 32);
    f.ecrire(0, 32); // debit minimum
    f.ecrire(e.bloc_0 as u32, 4);
    f.ecrire(e.bloc_1 as u32, 4);
    f.ecrire(1, 1); // bit de cadrage
    f.vider_page(false, false);

    // ---------------------------------------------------------- commentaire
    f.entete_vorbis(3);
    let signature = b"Ventriloque, d'apres ww2ogg";
    f.ecrire(signature.len() as u32, 32);
    f.octets(signature);
    f.ecrire(0, 32); // aucun commentaire d'utilisateur
    f.ecrire(1, 1);
    f.vider_page(false, false);

    // ---------------------------------------------------------- configuration
    let (debut_setup, taille_setup) = paquet(data, e.setup)?;
    let corps = data
        .get(debut_setup..debut_setup + taille_setup)
        .ok_or("paquet de configuration tronque")?;
    let mut l = Lecteur::nouveau(corps);

    f.entete_vorbis(5);

    let codebooks_moins_un = l.lire(8)?;
    f.ecrire(codebooks_moins_un, 8);
    let compte_codebooks = codebooks_moins_un + 1;
    for _ in 0..compte_codebooks {
        let rang = l.lire(10)?;
        let brut = codebook(rang)?;
        rebatir_codebook(&mut Lecteur::nouveau(brut), &mut f)?;
    }

    // Les transformees temporelles n'existent pas en Vorbis I : Wwise les omet, la specification
    // exige de les declarer vides.
    f.ecrire(0, 6);
    f.ecrire(0, 16);

    let plancher_moins_un = l.lire(6)?;
    f.ecrire(plancher_moins_un, 6);
    let compte_planchers = plancher_moins_un + 1;
    for _ in 0..compte_planchers {
        f.ecrire(1, 16); // toujours un plancher de type 1
        let partitions = l.lire(5)?;
        f.ecrire(partitions, 5);

        let mut classes = Vec::with_capacity(partitions as usize);
        let mut classe_max = 0;
        for _ in 0..partitions {
            let classe = l.lire(4)?;
            f.ecrire(classe, 4);
            classe_max = classe_max.max(classe);
            classes.push(classe);
        }

        let mut dimensions = vec![0u32; classe_max as usize + 1];
        for d in dimensions.iter_mut() {
            let moins_un = l.lire(3)?;
            f.ecrire(moins_un, 3);
            *d = moins_un + 1;
            let sous_classes = l.lire(2)?;
            f.ecrire(sous_classes, 2);
            if sous_classes != 0 {
                let maitre = l.lire(8)?;
                f.ecrire(maitre, 8);
                if maitre >= compte_codebooks {
                    return Err("plancher : codebook maitre invalide".into());
                }
            }
            for _ in 0..(1u32 << sous_classes) {
                let livre = l.lire(8)?;
                f.ecrire(livre, 8);
                if livre >= 1 && livre - 1 >= compte_codebooks {
                    return Err("plancher : codebook de sous-classe invalide".into());
                }
            }
        }

        let multiplicateur = l.lire(2)?;
        f.ecrire(multiplicateur, 2);
        let bits = l.lire(4)?;
        f.ecrire(bits, 4);
        for classe in &classes {
            for _ in 0..dimensions[*classe as usize] {
                let x = l.lire(bits)?;
                f.ecrire(x, bits);
            }
        }
    }

    let residus_moins_un = l.lire(6)?;
    f.ecrire(residus_moins_un, 6);
    let compte_residus = residus_moins_un + 1;
    for _ in 0..compte_residus {
        // Wwise ecrit le type sur deux bits, Vorbis sur seize.
        let type_residu = l.lire(2)?;
        f.ecrire(type_residu, 16);
        if type_residu > 2 {
            return Err("residu : type invalide".into());
        }

        let debut = l.lire(24)?;
        let fin = l.lire(24)?;
        let taille_partition = l.lire(24)?;
        let classifications_moins_un = l.lire(6)?;
        let classbook = l.lire(8)?;
        f.ecrire(debut, 24);
        f.ecrire(fin, 24);
        f.ecrire(taille_partition, 24);
        f.ecrire(classifications_moins_un, 6);
        f.ecrire(classbook, 8);
        if classbook >= compte_codebooks {
            return Err("residu : classbook invalide".into());
        }

        let classifications = classifications_moins_un + 1;
        let mut cascade = Vec::with_capacity(classifications as usize);
        for _ in 0..classifications {
            let bas = l.lire(3)?;
            f.ecrire(bas, 3);
            let drapeau = l.lire(1)?;
            f.ecrire(drapeau, 1);
            let haut = if drapeau != 0 {
                let h = l.lire(5)?;
                f.ecrire(h, 5);
                h
            } else {
                0
            };
            cascade.push(haut * 8 + bas);
        }
        for c in &cascade {
            for k in 0..8 {
                if c & (1 << k) != 0 {
                    let livre = l.lire(8)?;
                    f.ecrire(livre, 8);
                    if livre >= compte_codebooks {
                        return Err("residu : codebook invalide".into());
                    }
                }
            }
        }
    }

    let mappings_moins_un = l.lire(6)?;
    f.ecrire(mappings_moins_un, 6);
    let compte_mappings = mappings_moins_un + 1;
    for _ in 0..compte_mappings {
        f.ecrire(0, 16); // toujours un mapping de type 0

        let drapeau_sous_cartes = l.lire(1)?;
        f.ecrire(drapeau_sous_cartes, 1);
        let sous_cartes = if drapeau_sous_cartes != 0 {
            let moins_un = l.lire(4)?;
            f.ecrire(moins_un, 4);
            moins_un + 1
        } else {
            1
        };

        let couplage = l.lire(1)?;
        f.ecrire(couplage, 1);
        if couplage != 0 {
            let pas_moins_un = l.lire(8)?;
            f.ecrire(pas_moins_un, 8);
            let bits = ilog(e.canaux as u32 - 1);
            for _ in 0..pas_moins_un + 1 {
                let amplitude = l.lire(bits)?;
                let angle = l.lire(bits)?;
                f.ecrire(amplitude, bits);
                f.ecrire(angle, bits);
                if angle == amplitude
                    || amplitude >= e.canaux as u32
                    || angle >= e.canaux as u32
                {
                    return Err("mapping : couplage invalide".into());
                }
            }
        }

        // Un champ reserve que Wwise n'a pas retire, contrairement a tout le reste.
        let reserve = l.lire(2)?;
        f.ecrire(reserve, 2);
        if reserve != 0 {
            return Err("mapping : champ reserve non nul".into());
        }

        if sous_cartes > 1 {
            for _ in 0..e.canaux {
                let mux = l.lire(4)?;
                f.ecrire(mux, 4);
                if mux >= sous_cartes {
                    return Err("mapping : multiplexage hors bornes".into());
                }
            }
        }
        for _ in 0..sous_cartes {
            let temps = l.lire(8)?;
            f.ecrire(temps, 8);
            let plancher = l.lire(8)?;
            f.ecrire(plancher, 8);
            if plancher >= compte_planchers {
                return Err("mapping : plancher invalide".into());
            }
            let residu = l.lire(8)?;
            f.ecrire(residu, 8);
            if residu >= compte_residus {
                return Err("mapping : residu invalide".into());
            }
        }
    }

    // Les modes, et c'est le seul champ dont la suite a besoin : le drapeau de bloc de chaque
    // mode dit si un paquet audio porte une fenetre longue, donc s'il faut lui rendre ses deux
    // bits de fenetre.
    let modes_moins_un = l.lire(6)?;
    f.ecrire(modes_moins_un, 6);
    let compte_modes = modes_moins_un + 1;
    let bits_mode = ilog(compte_modes - 1);
    let mut fenetre_longue = Vec::with_capacity(compte_modes as usize);
    for _ in 0..compte_modes {
        let drapeau = l.lire(1)?;
        f.ecrire(drapeau, 1);
        fenetre_longue.push(drapeau != 0);
        f.ecrire(0, 16); // type de fenetre
        f.ecrire(0, 16); // type de transformee
        let mapping = l.lire(8)?;
        f.ecrire(mapping, 8);
        if mapping >= compte_mappings {
            return Err("mode : mapping invalide".into());
        }
    }
    f.ecrire(1, 1); // bit de cadrage
    f.vider_page(false, false);

    if l.bits_lus().div_ceil(8) != taille_setup {
        return Err(format!(
            "configuration : {} octets lus sur {taille_setup}",
            l.bits_lus().div_ceil(8)
        ));
    }

    // ---------------------------------------------------------- l'audio
    let mut position = e.premier_audio;
    // La fenetre du paquet precedent : une fenetre longue doit dire a quoi elle se raccorde des
    // deux cotes, et le cote gauche ne se relit nulle part.
    let mut precedent_long = false;
    while position < data.len() {
        let (debut, taille) = paquet(data, position)?;
        let suivant = debut + taille;
        let corps = data.get(debut..suivant).ok_or("paquet audio tronque")?;

        // Le granule n'est pas ecrit dans cette variante : Ogg s'en passe page par page, et le
        // decodeur recompte lui-meme.
        f.granule = 0;

        if e.paquets_modifies {
            // Wwise a retire le bit de type et les deux bits de fenetre du premier octet, et a
            // tasse le reste. On les remet, ce qui decale tout le paquet d'un a trois bits.
            let mut premier = Lecteur::nouveau(corps);
            f.ecrire(0, 1); // type de paquet : audio
            let mode = premier.lire(bits_mode)?;
            f.ecrire(mode, bits_mode);
            let reste = premier.lire(8 - bits_mode)?;
            let long = *fenetre_longue
                .get(mode as usize)
                .ok_or("paquet audio : numero de mode hors table")?;

            if long {
                // Une fenetre longue a besoin de savoir a quoi elle se raccorde des deux cotes.
                // Le cote gauche, on s'en souvient ; le cote droit se lit en avance sur le
                // paquet suivant.
                let mut apres = false;
                if suivant + 2 <= data.len() {
                    let (debut_suivant, taille_suivant) = paquet(data, suivant)?;
                    if taille_suivant > 0 {
                        if let Some(c) = data.get(debut_suivant..debut_suivant + taille_suivant) {
                            let mode_suivant = Lecteur::nouveau(c).lire(bits_mode)?;
                            apres = fenetre_longue[mode_suivant as usize];
                        }
                    }
                }
                f.ecrire(precedent_long as u32, 1);
                f.ecrire(apres as u32, 1);
            }
            precedent_long = long;

            f.ecrire(reste, 8 - bits_mode);
            f.octets(&corps[1..]);
        } else {
            f.octets(corps);
        }

        position = suivant;
        f.vider_page(false, position >= data.len());
    }
    f.vider_page(false, true);

    Ok(std::mem::take(&mut f.sortie))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_crc_ogg_est_celui_de_la_specification() {
        // Le piege est de prendre celui de zlib, qui rendrait 0xCBF43926 sur la meme chaine :
        // une page mal signee n'est pas refusee bruyamment, elle est ignoree en silence.
        assert_eq!(controle(b"123456789"), 0x89A1_897F);
        assert_eq!(controle(b"OggS"), 0x5FB0_A94F);
    }

    #[test]
    fn ilog_compte_les_bits() {
        assert_eq!(ilog(0), 0);
        assert_eq!(ilog(1), 1);
        assert_eq!(ilog(7), 3);
        assert_eq!(ilog(8), 4);
    }

    #[test]
    fn la_bibliotheque_porte_les_codebooks_de_cyberpunk() {
        // Les identifiants releves sur le doublage francais vont de 50 a 254.
        assert!(codebook(50).is_ok());
        assert!(codebook(254).is_ok());
        assert!(codebook(597).is_ok());
        assert!(codebook(598).is_err());
    }

    #[test]
    fn les_bits_se_relisent_dans_le_meme_ordre() {
        let mut f = Flux::nouveau();
        f.ecrire(0b101, 3);
        f.ecrire(0xABCD, 16);
        f.vider_bits();
        let mut l = Lecteur::nouveau(&f.charge);
        assert_eq!(l.lire(3).unwrap(), 0b101);
        assert_eq!(l.lire(16).unwrap(), 0xABCD);
    }

    #[test]
    fn ce_qui_n_est_pas_du_wwise_est_reconnu_comme_tel() {
        assert!(!est_wwise(b"OggS\0\0\0\0"));
        assert!(!est_wwise(b""));
    }
}
