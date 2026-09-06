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
//!
//! CE MODULE N'EST PAS ENCORE BRANCHE. Il decode, il est teste, et rien ne l'appelle : le
//! chemin Cyberpunk lit pour l'instant des entrees deja exploitables. La jointure se fera dans
//! le lecteur -- convertir les octets d'un `.wem` en Ogg AVANT de les passer au montage --, ce
//! qui est aussi la raison pour laquelle ce code vit ici et pas dans `voice-assembly` : c'est
//! une affaire de conteneur de jeu, pas de montage. D'ou le `dead_code` ci-dessous, qui dit
//! l'etat plutot que de le taire.
#![allow(dead_code)]

/// La bibliotheque de codebooks d'aoTuV, vendorisee depuis ww2ogg. Voir `build.rs`.
const CODEBOOKS: &[u8] = include_bytes!("../../vendor/ww2ogg/packed_codebooks_aoTuV_603.bin");

/// Lit des bits dans l'ordre ou Vorbis les ecrit : poids faible d'abord, byte par byte.
struct BitReader<'a> {
    data: &'a [u8],
    bit: usize,
}

impl<'a> BitReader<'a> {
    fn new(data: &'a [u8]) -> Self {
        BitReader { data, bit: 0 }
    }

    fn read(&mut self, count: u32) -> Result<u32, String> {
        let mut value = 0u32;
        for page_index in 0..count {
            let byte = self
                .data
                .get(self.bit >> 3)
                .ok_or("flux Wwise tronque")?;
            if (byte >> (self.bit & 7)) & 1 == 1 {
                value |= 1 << page_index;
            }
            self.bit += 1;
        }
        Ok(value)
    }

    fn bits_read(&self) -> usize {
        self.bit
    }
}

/// Ecrit des bits et les decoupe en pages Ogg. Un seul flux, numero de serie 1, comme ww2ogg.
struct OggStream {
    out: Vec<u8>,
    buffer: u8,
    bits: u32,
    payload: Vec<u8>,
    first_page: bool,
    continued: bool,
    granule: u32,
    page_index: u32,
}

const PAGE_HEADER: usize = 27;
const MAX_SEGMENTS: usize = 255;
const MAX_SEGMENT_SIZE: usize = 255;

impl OggStream {
    fn new() -> Self {
        OggStream {
            out: Vec::new(),
            buffer: 0,
            bits: 0,
            payload: Vec::new(),
            first_page: true,
            continued: false,
            granule: 0,
            page_index: 0,
        }
    }

    fn bit(&mut self, pose: bool) {
        if pose {
            self.buffer |= 1 << self.bits;
        }
        self.bits += 1;
        if self.bits == 8 {
            self.vider_bits();
        }
    }

    fn ecrire(&mut self, value: u32, count: u32) {
        for page_index in 0..count {
            self.bit((value >> page_index) & 1 == 1);
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
            self.payload.push(self.buffer);
            self.bits = 0;
            self.buffer = 0;
        }
    }

    fn vider_page(&mut self, suite_ensuite: bool, derniere: bool) {
        if self.payload.len() != MAX_SEGMENT_SIZE * MAX_SEGMENTS {
            self.vider_bits();
        }
        if self.payload.is_empty() {
            return;
        }

        // Arrondi au superieur volontaire : une payload multiple de 255 finit par un segment
        // vide, qui est ce qui dit « le packet s'arrete ici ».
        let mut segments = (self.payload.len() + MAX_SEGMENT_SIZE) / MAX_SEGMENT_SIZE;
        if segments == MAX_SEGMENTS + 1 {
            segments = MAX_SEGMENTS;
        }

        let mut page = vec![0u8; PAGE_HEADER + segments + self.payload.len()];
        page[0..4].copy_from_slice(b"OggS");
        page[4] = 0;
        page[5] = (self.continued as u8) | ((self.first_page as u8) << 1) | ((derniere as u8) << 2);
        page[6..10].copy_from_slice(&self.granule.to_le_bytes());
        // Le granule de Wwise tient sur 32 bits ; `0xFFFFFFFF` veut dire « inconnu », et Ogg
        // l'ecrit sur ses 64 bits.
        let haut: u32 = if self.granule == u32::MAX { u32::MAX } else { 0 };
        page[10..14].copy_from_slice(&haut.to_le_bytes());
        page[14..18].copy_from_slice(&1u32.to_le_bytes());
        page[18..22].copy_from_slice(&self.page_index.to_le_bytes());
        page[22..26].copy_from_slice(&0u32.to_le_bytes());
        page[26] = segments as u8;

        let mut reste = self.payload.len();
        for i in 0..segments {
            let pris = reste.min(MAX_SEGMENT_SIZE);
            page[PAGE_HEADER + i] = pris as u8;
            reste -= pris;
        }
        page[PAGE_HEADER + segments..].copy_from_slice(&self.payload);

        let somme = crc(&page);
        page[22..26].copy_from_slice(&somme.to_le_bytes());

        self.out.extend_from_slice(&page);
        self.page_index += 1;
        self.first_page = false;
        self.continued = suite_ensuite;
        self.payload.clear();
    }
}

/// La somme de crc d'une page Ogg : CRC-32 non reflechi, polynome 0x04C11DB7, sans
/// inversion ni finale. Ce n'est pas le CRC-32 de zlib, et s'y tromper ne se voit qu'a la
/// lecture, quand le decodeur jette la page sans rien dire.
fn crc(page: &[u8]) -> u32 {
    let mut somme = 0u32;
    for byte in page {
        somme ^= (*byte as u32) << 24;
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
fn quantvals(entries: u32, dimensions: u32) -> u32 {
    if dimensions == 0 {
        return 0;
    }
    let bits = ilog(entries);
    let mut vals = entries >> (((bits - 1) * (dimensions - 1)) / dimensions);
    loop {
        let mut acc = 1u64;
        let mut acc1 = 1u64;
        for _ in 0..dimensions {
            acc = acc.saturating_mul(vals as u64);
            acc1 = acc1.saturating_mul(vals as u64 + 1);
        }
        if acc <= entries as u64 && acc1 > entries as u64 {
            return vals;
        }
        if acc > entries as u64 {
            vals -= 1;
        } else {
            vals += 1;
        }
    }
}

/// Un codebook de la bibliotheque, par son page_index.
fn codebook(page_index: u32) -> Result<&'static [u8], String> {
    let taille = CODEBOOKS.len();
    let depart = u32::from_le_bytes(CODEBOOKS[taille - 4..].try_into().unwrap()) as usize;
    let compte = (taille - depart) / 4 - 1;
    if page_index as usize >= compte {
        return Err(format!("codebook {page_index} hors de la bibliotheque ({compte})"));
    }
    let borne = |i: usize| {
        u32::from_le_bytes(CODEBOOKS[depart + i * 4..depart + i * 4 + 4].try_into().unwrap())
            as usize
    };
    Ok(&CODEBOOKS[borne(page_index as usize)..borne(page_index as usize + 1)])
}

/// Reconstitue un codebook complet a partir de la forme abregee que Wwise range dans sa
/// bibliotheque : les champs y sont ecrits sur moins de bits, et la signature manque.
fn rebuild_codebook(l: &mut BitReader, f: &mut OggStream) -> Result<(), String> {
    let dimensions = l.read(4)?;
    let entries = l.read(14)?;

    f.ecrire(0x56_4342, 24); // « BCV »
    f.ecrire(dimensions, 16);
    f.ecrire(entries, 24);

    let ordonne = l.read(1)?;
    f.ecrire(ordonne, 1);
    if ordonne != 0 {
        let longueur = l.read(5)?;
        f.ecrire(longueur, 5);
        let mut courant = 0u32;
        while courant < entries {
            let count = ilog(entries - courant);
            let nombre = l.read(count)?;
            f.ecrire(nombre, count);
            courant += nombre;
        }
        if courant > entries {
            return Err("codebook : compte d'entries hors bornes".into());
        }
    } else {
        let taille_longueur = l.read(3)?;
        let creux = l.read(1)?;
        if taille_longueur == 0 || taille_longueur > 5 {
            return Err("codebook : longueur de mot absurde".into());
        }
        f.ecrire(creux, 1);
        for _ in 0..entries {
            let present = if creux != 0 {
                let p = l.read(1)?;
                f.ecrire(p, 1);
                p != 0
            } else {
                true
            };
            if present {
                // Wwise ecrit la longueur sur le minimum de bits ; Vorbis en veut cinq.
                let longueur = l.read(taille_longueur)?;
                f.ecrire(longueur, 5);
            }
        }
    }

    let lookup_type = l.read(1)?;
    f.ecrire(lookup_type, 4);
    if lookup_type == 1 {
        let min = l.read(32)?;
        let max = l.read(32)?;
        let taille_valeur = l.read(4)?;
        let sequence = l.read(1)?;
        f.ecrire(min, 32);
        f.ecrire(max, 32);
        f.ecrire(taille_valeur, 4);
        f.ecrire(sequence, 1);
        for _ in 0..quantvals(entries, dimensions) {
            let v = l.read(taille_valeur + 1)?;
            f.ecrire(v, taille_valeur + 1);
        }
    } else if lookup_type != 0 {
        return Err(format!("codebook : table de correspondance {lookup_type} non traitee"));
    }
    Ok(())
}

/// Ce qu'un `.wem` Wwise Vorbis annonce de lui-meme.
struct WemHeader {
    channels: u16,
    sample_rate: u32,
    bytes_per_second: u32,
    block_0: u8,
    block_1: u8,
    /// Le premier byte de chaque packet audio a ete recompose par Wwise et doit l'etre a
    /// l'envers : type de packet, numero de mode, et les deux bits de fenetre.
    repacked_packets: bool,
    setup: usize,
    first_audio: usize,
}

fn u16_at(d: &[u8], p: usize) -> Result<u16, String> {
    d.get(p..p + 2)
        .map(|o| u16::from_le_bytes(o.try_into().unwrap()))
        .ok_or_else(|| "RIFF tronque".to_string())
}

fn u32_at(d: &[u8], p: usize) -> Result<u32, String> {
    d.get(p..p + 4)
        .map(|o| u32::from_le_bytes(o.try_into().unwrap()))
        .ok_or_else(|| "RIFF tronque".to_string())
}

/// Dit si ces octets sont du Wwise Vorbis, c'est-a-dire un RIFF WAVE dont le format est 0xFFFF.
/// C'est la donnee qui decide, pas le nom du fichier : les entries d'une archive Cyberpunk n'ont
/// pas de nom, seulement un hachage.
pub fn is_wwise(data: &[u8]) -> bool {
    if data.len() < 24 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return false;
    }
    let mut p = 12;
    while p + 8 <= data.len() {
        let size = match u32_at(data, p + 4) {
            Ok(t) => t as usize,
            Err(_) => return false,
        };
        if &data[p..p + 4] == b"fmt " {
            return u16_at(data, p + 8).map(|t| t == 0xFFFF).unwrap_or(false);
        }
        p += 8 + size + (size & 1);
    }
    false
}

fn split_header(data: &[u8]) -> Result<(WemHeader, &[u8]), String> {
    if data.len() < 12 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
        return Err("ce n'est pas un RIFF WAVE".into());
    }

    // `fmt ` decrit le flux, `data` le porte. Deux chunks du RIFF, a ne pas confondre avec les
    // octets du fichier entier -- d'ou le suffixe.
    let mut fmt_chunk: Option<&[u8]> = None;
    let mut data_chunk: Option<&[u8]> = None;
    let mut p = 12;
    while p + 8 <= data.len() {
        let size = u32_at(data, p + 4)? as usize;
        let body = data
            .get(p + 8..p + 8 + size)
            .ok_or("chunk RIFF tronque")?;
        match &data[p..p + 4] {
            b"fmt " => fmt_chunk = Some(body),
            b"data" => data_chunk = Some(body),
            b"vorb" => {
                return Err("chunk « vorb » separe : variante Wwise ancienne, non traitee".into());
            }
            _ => {}
        }
        p += 8 + size + (size & 1);
    }

    let fmt = fmt_chunk.ok_or("pas de chunk « fmt »")?;
    let audio = data_chunk.ok_or("pas de chunk « data »")?;

    // 0x42 est la seule taille ou le bloc `vorb` est fondu dans `fmt`, a l'offset 0x18. Les
    // autres tailles vont avec un chunk `vorb` separe, ecarte plus haut.
    if fmt.len() != 0x42 {
        return Err(format!("« fmt » de {} octets, 0x42 attendu", fmt.len()));
    }
    if u16_at(fmt, 0)? != 0xFFFF {
        return Err("ce n'est pas du Wwise Vorbis (format != 0xFFFF)".into());
    }

    let vorb = &fmt[0x18..0x42];
    let signal = u32_at(vorb, 0x04)?;
    Ok((
        WemHeader {
            channels: u16_at(fmt, 2)?,
            sample_rate: u32_at(fmt, 4)?,
            bytes_per_second: u32_at(fmt, 8)?,
            block_0: vorb[0x28],
            block_1: vorb[0x29],
            // Les valeurs qui vont avec des paquets intacts ont ete relevees par ww2ogg ; tout
            // le reste demande la recomposition. Cyberpunk dit 0xDD.
            repacked_packets: !matches!(signal, 0x4A | 0x4B | 0x69 | 0x70),
            setup: u32_at(vorb, 0x10)? as usize,
            first_audio: u32_at(vorb, 0x14)? as usize,
        },
        audio,
    ))
}

/// Un packet Wwise « moderne » : deux octets de taille, pas de granule.
fn packet(data: &[u8], position: usize) -> Result<(usize, usize), String> {
    let taille = u16_at(data, position)? as usize;
    Ok((position + 2, taille))
}

/// Convertit un `.wem` Wwise Vorbis en flux Ogg Vorbis complet, pret pour symphonia.
pub fn to_ogg(data: &[u8]) -> Result<Vec<u8>, String> {
    let (e, data) = split_header(data)?;
    let mut f = OggStream::new();

    // ---------------------------------------------------------- identification
    f.entete_vorbis(1);
    f.ecrire(0, 32); // version
    f.ecrire(e.channels as u32, 8);
    f.ecrire(e.sample_rate, 32);
    f.ecrire(0, 32); // debit maximum
    f.ecrire(e.bytes_per_second.saturating_mul(8), 32);
    f.ecrire(0, 32); // debit minimum
    f.ecrire(e.block_0 as u32, 4);
    f.ecrire(e.block_1 as u32, 4);
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
    let (debut_setup, taille_setup) = packet(data, e.setup)?;
    let corps = data
        .get(debut_setup..debut_setup + taille_setup)
        .ok_or("packet de configuration tronque")?;
    let mut l = BitReader::new(corps);

    f.entete_vorbis(5);

    let codebooks_moins_un = l.read(8)?;
    f.ecrire(codebooks_moins_un, 8);
    let compte_codebooks = codebooks_moins_un + 1;
    for _ in 0..compte_codebooks {
        let page_index = l.read(10)?;
        let brut = codebook(page_index)?;
        rebuild_codebook(&mut BitReader::new(brut), &mut f)?;
    }

    // Les transformees temporelles n'existent pas en Vorbis I : Wwise les omet, la specification
    // exige de les declarer vides.
    f.ecrire(0, 6);
    f.ecrire(0, 16);

    let plancher_moins_un = l.read(6)?;
    f.ecrire(plancher_moins_un, 6);
    let compte_planchers = plancher_moins_un + 1;
    for _ in 0..compte_planchers {
        f.ecrire(1, 16); // toujours un plancher de type 1
        let partitions = l.read(5)?;
        f.ecrire(partitions, 5);

        let mut classes = Vec::with_capacity(partitions as usize);
        let mut classe_max = 0;
        for _ in 0..partitions {
            let classe = l.read(4)?;
            f.ecrire(classe, 4);
            classe_max = classe_max.max(classe);
            classes.push(classe);
        }

        let mut dimensions = vec![0u32; classe_max as usize + 1];
        for d in dimensions.iter_mut() {
            let moins_un = l.read(3)?;
            f.ecrire(moins_un, 3);
            *d = moins_un + 1;
            let sous_classes = l.read(2)?;
            f.ecrire(sous_classes, 2);
            if sous_classes != 0 {
                let maitre = l.read(8)?;
                f.ecrire(maitre, 8);
                if maitre >= compte_codebooks {
                    return Err("plancher : codebook maitre invalide".into());
                }
            }
            for _ in 0..(1u32 << sous_classes) {
                let livre = l.read(8)?;
                f.ecrire(livre, 8);
                if livre >= 1 && livre - 1 >= compte_codebooks {
                    return Err("plancher : codebook de sous-classe invalide".into());
                }
            }
        }

        let multiplicateur = l.read(2)?;
        f.ecrire(multiplicateur, 2);
        let bits = l.read(4)?;
        f.ecrire(bits, 4);
        for classe in &classes {
            for _ in 0..dimensions[*classe as usize] {
                let x = l.read(bits)?;
                f.ecrire(x, bits);
            }
        }
    }

    let residus_moins_un = l.read(6)?;
    f.ecrire(residus_moins_un, 6);
    let compte_residus = residus_moins_un + 1;
    for _ in 0..compte_residus {
        // Wwise ecrit le type sur deux bits, Vorbis sur seize.
        let type_residu = l.read(2)?;
        f.ecrire(type_residu, 16);
        if type_residu > 2 {
            return Err("residu : type invalide".into());
        }

        let debut = l.read(24)?;
        let fin = l.read(24)?;
        let taille_partition = l.read(24)?;
        let classifications_moins_un = l.read(6)?;
        let classbook = l.read(8)?;
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
            let bas = l.read(3)?;
            f.ecrire(bas, 3);
            let drapeau = l.read(1)?;
            f.ecrire(drapeau, 1);
            let haut = if drapeau != 0 {
                let h = l.read(5)?;
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
                    let livre = l.read(8)?;
                    f.ecrire(livre, 8);
                    if livre >= compte_codebooks {
                        return Err("residu : codebook invalide".into());
                    }
                }
            }
        }
    }

    let mappings_moins_un = l.read(6)?;
    f.ecrire(mappings_moins_un, 6);
    let compte_mappings = mappings_moins_un + 1;
    for _ in 0..compte_mappings {
        f.ecrire(0, 16); // toujours un mapping de type 0

        let drapeau_sous_cartes = l.read(1)?;
        f.ecrire(drapeau_sous_cartes, 1);
        let sous_cartes = if drapeau_sous_cartes != 0 {
            let moins_un = l.read(4)?;
            f.ecrire(moins_un, 4);
            moins_un + 1
        } else {
            1
        };

        let couplage = l.read(1)?;
        f.ecrire(couplage, 1);
        if couplage != 0 {
            let pas_moins_un = l.read(8)?;
            f.ecrire(pas_moins_un, 8);
            let bits = ilog(e.channels as u32 - 1);
            for _ in 0..pas_moins_un + 1 {
                let amplitude = l.read(bits)?;
                let angle = l.read(bits)?;
                f.ecrire(amplitude, bits);
                f.ecrire(angle, bits);
                if angle == amplitude
                    || amplitude >= e.channels as u32
                    || angle >= e.channels as u32
                {
                    return Err("mapping : couplage invalide".into());
                }
            }
        }

        // Un champ reserve que Wwise n'a pas retire, contrairement a tout le reste.
        let reserve = l.read(2)?;
        f.ecrire(reserve, 2);
        if reserve != 0 {
            return Err("mapping : champ reserve non nul".into());
        }

        if sous_cartes > 1 {
            for _ in 0..e.channels {
                let mux = l.read(4)?;
                f.ecrire(mux, 4);
                if mux >= sous_cartes {
                    return Err("mapping : multiplexage hors bornes".into());
                }
            }
        }
        for _ in 0..sous_cartes {
            let temps = l.read(8)?;
            f.ecrire(temps, 8);
            let plancher = l.read(8)?;
            f.ecrire(plancher, 8);
            if plancher >= compte_planchers {
                return Err("mapping : plancher invalide".into());
            }
            let residu = l.read(8)?;
            f.ecrire(residu, 8);
            if residu >= compte_residus {
                return Err("mapping : residu invalide".into());
            }
        }
    }

    // Les modes, et c'est le seul champ dont la continued a besoin : le drapeau de bloc de chaque
    // mode dit si un packet audio porte une fenetre longue, donc s'il faut lui rendre ses deux
    // bits de fenetre.
    let modes_moins_un = l.read(6)?;
    f.ecrire(modes_moins_un, 6);
    let compte_modes = modes_moins_un + 1;
    let bits_mode = ilog(compte_modes - 1);
    let mut fenetre_longue = Vec::with_capacity(compte_modes as usize);
    for _ in 0..compte_modes {
        let drapeau = l.read(1)?;
        f.ecrire(drapeau, 1);
        fenetre_longue.push(drapeau != 0);
        f.ecrire(0, 16); // type de fenetre
        f.ecrire(0, 16); // type de transformee
        let mapping = l.read(8)?;
        f.ecrire(mapping, 8);
        if mapping >= compte_mappings {
            return Err("mode : mapping invalide".into());
        }
    }
    f.ecrire(1, 1); // bit de cadrage
    f.vider_page(false, false);

    if l.bits_read().div_ceil(8) != taille_setup {
        return Err(format!(
            "configuration : {} octets lus sur {taille_setup}",
            l.bits_read().div_ceil(8)
        ));
    }

    // ---------------------------------------------------------- l'audio
    let mut position = e.first_audio;
    // La fenetre du packet precedent : une fenetre longue doit dire a quoi elle se raccorde des
    // deux cotes, et le cote gauche ne se relit nulle part.
    let mut precedent_long = false;
    while position < data.len() {
        let (debut, taille) = packet(data, position)?;
        let suivant = debut + taille;
        let corps = data.get(debut..suivant).ok_or("packet audio tronque")?;

        // Le granule n'est pas ecrit dans cette variante : Ogg s'en passe page par page, et le
        // decodeur recompte lui-meme.
        f.granule = 0;

        if e.repacked_packets {
            // Wwise a retire le bit de type et les deux bits de fenetre du premier byte, et a
            // tasse le reste. On les remet, ce qui decale tout le packet d'un a trois bits.
            let mut premier = BitReader::new(corps);
            f.ecrire(0, 1); // type de packet : audio
            let mode = premier.read(bits_mode)?;
            f.ecrire(mode, bits_mode);
            let reste = premier.read(8 - bits_mode)?;
            let long = *fenetre_longue
                .get(mode as usize)
                .ok_or("packet audio : numero de mode hors table")?;

            if long {
                // Une fenetre longue a besoin de savoir a quoi elle se raccorde des deux cotes.
                // Le cote gauche, on s'en souvient ; le cote droit se lit en avance sur le
                // packet suivant.
                let mut apres = false;
                if suivant + 2 <= data.len() {
                    let (debut_suivant, taille_suivant) = packet(data, suivant)?;
                    if taille_suivant > 0 {
                        if let Some(c) = data.get(debut_suivant..debut_suivant + taille_suivant) {
                            let mode_suivant = BitReader::new(c).read(bits_mode)?;
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

    Ok(std::mem::take(&mut f.out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn le_crc_ogg_est_celui_de_la_specification() {
        // Le piege est de prendre celui de zlib, qui rendrait 0xCBF43926 sur la meme chaine :
        // une page mal signee n'est pas refusee bruyamment, elle est ignoree en silence.
        assert_eq!(crc(b"123456789"), 0x89A1_897F);
        assert_eq!(crc(b"OggS"), 0x5FB0_A94F);
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
        let mut f = OggStream::new();
        f.ecrire(0b101, 3);
        f.ecrire(0xABCD, 16);
        f.vider_bits();
        let mut l = BitReader::new(&f.payload);
        assert_eq!(l.read(3).unwrap(), 0b101);
        assert_eq!(l.read(16).unwrap(), 0xABCD);
    }

    #[test]
    fn ce_qui_n_est_pas_du_wwise_est_reconnu_comme_tel() {
        assert!(!is_wwise(b"OggS\0\0\0\0"));
        assert!(!is_wwise(b""));
    }
}
