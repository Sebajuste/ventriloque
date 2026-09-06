// Cyberpunk 2077 — la recette du paquet de voice.
//
// Ce script tourne dans `fabriquer.exe`, pas dans Ventriloque. Il n'a aucune entrée-sortie : il
// nomme des entrées d'archive et des personnages, l'hôte lit, décode, monte et écrit.
//
// CE QU'UNE PRISE S'APPELLE ICI. Une archive REDengine ne range pas les chemins de ses fichiers,
// seulement le FNV1a64 du chemin en minuscules. Un name d'entrée est donc un hachage de seize
// chiffres hexadécimaux, et non un chemin comme dans la recette de StarCraft II. Le chemin
// d'origine est en commentaire au bout de chaque ligne — pour qu'on puisse lire à qui l'on a
// affaire, et le revérifier :
//
//     fabriquer --hacher "base\localization\fr-fr\vo\judy_q105_f_17a687de8d2b6000.wem"
//     019291c8e47b9861
//
// POURQUOI DES HACHAGES ÉCRITS EN DUR, ET PAS UN FILTRE. Sélectionner par motif — `^judy_`,
// `^sobchak_` — demanderait le dictionnaire de chemins de WolvenKit : 1,7 million d'entrées,
// 135 Mo, à côté d'un paquet qui pèse 25 Ko. Une recette n'a pas à refaire le choix, elle a à
// le porter. Les 68 takes ci-dessous *sont* le résultat du choix, fait ailleurs et à l'oreille.
//
// D'OÙ VIENT CE CASTING. Il a été établi par l'extracteur d'`ai_npc-holo`, écouté personnage par
// personnage le 2026-08-31, et ce qui n'a pas marché y est écrit noir sur blanc. Trois choses en
// sont sorties et tiennent encore ici :
//
//   - les takes retenues font 2 à 8 s, au-dessus de -26 dBFS ; en dessous ce sont les messages
//     holo de l'épilogue, enregistrés trois à cinq fois plus bas, et leur remontée en niveau
//     donne à toute la référence une dynamique de ligne téléphonique ;
//   - les *voice sets* — interjections de combat et d'ambiance — sont écartés : celle de Jackie
//     était deux fois plus forte et deux fois plus étroite de bande que ses répliques de scène ;
//   - `songbird` reste en dessous des autres, et ce n'est pas un défaut de la recette : dans
//     Phantom Liberty, Song So Mi est presque toujours entendue par lien relic ou par comms, et
//     son doublage porte déjà le filtre. Il n'y a pas trente secondes de voice nue à prendre.
//
// LA LANGUE EST LE FRANÇAIS, et ce n'est pas un détail de confort : une référence anglaise
// passée à un moteur qui parle français rend un accent anglais. Les noms de fichiers sont
// identiques d'une langue à l'autre, donc seuls les hachages changeraient pour une autre langue.
//
// Les voice ne sont pas dans ce paquet et ne peuvent pas y être : ce sont des enregistrements
// d'acteurs. Ce fichier ne dit qu'où les trouver dans la copie du joueur.

const UNIVERS = "Cyberpunk 2077";

if product() != "cp77" {
    throw "cette recette est celle de Cyberpunk 2077, et ce stockage annonce « " + product() + " »";
}

// Les takes ont été choisies dans une fenêtre de 2 à 8 s, mesurée après rognage des silences.
// L'hôte rogne à -42 dBFS et non -48, donc un peu plus large : la fenêtre est desserrée d'autant
// pour ne pas écarter une prise déjà retenue à cause d'un seuil qui n'est pas le même.
const DUREE = 32.0;
const MINI = 1.5;
const MAXI = 9.0;

const CASTING = [
    #{
        id: "goro_takemura",
        name: "Goro Takemura",
        lines: [
            "Il y a une manière correcte de faire les choses.",
            "Ma loyauté ne s'achète pas.",
            "Vous jugez trop vite. Attendez.",
        ],
        // 7 takes, 34.4 s a la source
        takes: [
            "0088ea45ce6d59cb", // takemura_q201_f_1bd00cd58c5f9020
            "019520590085d4a9", // takemura_q101_f_16c356afc328d000
            "019a557b662a0490", // takemura_q112_f_17dd73f08b4e1000
            "01cb41ffa10f200d", // takemura_q112_f_1826008bde2d7000
            "0211c542201c8495", // takemura_q112_f_17ee4061484e1000
            "025ba359c5bc977b", // takemura_q112_f_17ee3afe7c4e1000
            "02c1e6910dfccccb", // takemura_q112_f_17c39257934e1000
        ],
    },
    #{
        id: "jackie_welles",
        name: "Jackie Welles",
        lines: [
            "Hé, mon pote ! Ça faisait un bail.",
            "T'inquiète, je gère. On fait ça propre.",
            "Un verre au Coyote, et on en reparle.",
        ],
        // 7 takes, 31.0 s a la source
        takes: [
            "001b84026b1557dd", // jackie_q005_f_1ad874f9e7405000
            "00ae29c5ca94c2fa", // jackie_q003_f_17b5cedaaa4b6000
            "0103b91f684c510d", // jackie_q003_f_185adee7622ef000
            "019d31a27af43149", // jackie_q005_f_168a2a88272cd000
            "024ef6481bcf60fc", // jackie_q000_f_18e53c3f552ef000
            "031e112eb119cf53", // jackie_q001_f_16663a5ec85b6000
            "032de654e8f61f97", // jackie_q001_m_19e64b4c3f627000
        ],
    },
    #{
        id: "judy_alvarez",
        name: "Judy Alvarez",
        lines: [
            "Passe-moi ça, je vais voir ce que ça donne.",
            "J'ai pas le temps pour les conneries des corpos.",
            "T'es sûr de toi ? Parce que moi, non.",
        ],
        // 6 takes, 33.0 s a la source
        takes: [
            "019291c8e47b9861", // judy_q105_f_17a687de8d2b6000
            "01ccfd60fb417b6f", // judy_q004_f_16a54a98e829f000
            "01cfc261dbddac64", // judy_sq030_f_18795a9ae52fc000
            "02430c7a6269b22d", // judy_q004_m_10d3dd9c5f29f000
            "028bf7d3e8d34620", // judy_sq030_f_1994ab416a2b6000
            "030d2eb71a600ed8", // judy_sq030_m_18a6e78c9e2b6000
        ],
    },
    #{
        id: "kerry_eurodyne",
        name: "Kerry Eurodyne",
        lines: [
            "Tu sais qui je suis, ou tu fais semblant ?",
            "C'était pas mieux avant. C'était juste plus fort.",
            "Fais pas cette tête, c'est que du bruit.",
        ],
        // 8 takes, 32.5 s a la source
        takes: [
            "00453b3d14fb9600", // kerry_sq017_f_1f0c200b5142f000
            "0063c2e315446443", // kerry_scene_kerry_default_f_1af678770c29f000
            "0091ab24682ef991", // kerry_sq017_f_1921eb403c29f000
            "0106c5ab0138e176", // kerry_q201_m_181828b8ef386010
            "0112e3678b9da84c", // kerry_sq017_f_167bb9d4035f9000
            "01c929681081a9fe", // kerry_sq017_f_197648eeb629f000
            "02f580654165b9ca", // kerry_q203_m_18dfe7dae529f000
            "0399fdb66a3f96e0", // kerry_sq017_f_129854bb20401000
        ],
    },
    #{
        id: "panam_palmer",
        name: "Panam Palmer",
        lines: [
            "On fait ça à ma façon, ou on le fait pas.",
            "T'as intérêt à pas me décevoir.",
            "Monte. On roule.",
        ],
        // 9 takes, 30.2 s a la source
        takes: [
            "002f08b7693d0f2e", // panam_q103_f_18cb45e56e29f000
            "0030790e7fefaa01", // panam_q103_f_1724a87b1829f000
            "012ae65fc3e1aa47", // panam_sq004_f_13196ee75b2d7000
            "01e8ff7625b41301", // panam_sq027_f_1a4c94c1022c5000
            "029522228c8b598f", // panam_q202_m_19506428d129f000
            "02d58697cc042385", // panam_q115_m_185efbe0fb502030
            "030d897cd9c4ae9f", // panam_q202_m_179b7e6e1a2cd000
            "037d6eaf5cadf664", // panam_sq027_f_1909d114dc351000
            "03812c0839b52014", // panam_sq004_f_197035a3d0623000
        ],
    },
    #{
        id: "river_ward",
        name: "River Ward",
        lines: [
            "Je suis pas là en tant que flic. Pas ce soir.",
            "Raconte-moi tout, depuis le début.",
            "Y a des choses qu'on peut pas laisser passer.",
        ],
        // 8 takes, 31.2 s a la source
        takes: [
            "0072563218891b77", // sobchak_sq012_f_1b2fb1312a386000
            "00fe294ee928ac75", // sobchak_sq012_f_17960f877d49d000
            "010ce97be1dbd7a3", // sobchak_scene_sobchak_default_f_1ae8654be6521000
            "01ca7a4be6ad0259", // sobchak_sq021_f_1aba9b3777521000
            "02400aafaecf02cd", // sobchak_sq021_f_1936ad1e5a2d2000
            "0289e121ad5ef594", // sobchak_sq029_f_1b3581888d623004
            "04350e5b43362675", // sobchak_sq021_f_1a4dbf212c5f9000
            "04cea01692067e4d", // sobchak_sq029_f_19988f2777521000
        ],
    },
    #{
        id: "rogue_amendiares",
        name: "Rogue Amendiares",
        lines: [
            "Le prix a changé. C'est comme ça.",
            "J'ai pas besoin de connaître tes raisons.",
            "Assieds-toi. Et parle vite.",
        ],
        // 9 takes, 33.1 s a la source
        takes: [
            "0019d26cd71e8b02", // rogue_q115_f_179c35d2614e1000
            "004e13e31ea807fc", // rogue_sq031_f_1a439d874b2c5000
            "00e880efe5d9ce19", // rogue_sq031_f_1a66b4cc062c5000
            "0144e37b441cf6a8", // rogue_q203_f_1f4486072a42f000
            "01dad90cbffa2951", // rogue_q115_f_1a24f2c0c3502000
            "0219501bf4847bf6", // rogue_sts_wbr_hil_06_f_2083b044a5559000
            "0344cad4f787887d", // rogue_q103_f_173550082b29f00c
            "03514c0db4fac56f", // rogue_q115_f_1734b245864e1000
            "0385c08a61a84fd9", // rogue_q115_f_17a503a7d94e1000
        ],
    },
    #{
        id: "songbird",
        name: "Songbird",
        lines: [
            "Je sais déjà ce que tu vas demander.",
            "Rien de ce que tu vois ici n'est vraiment là.",
            "Fais-moi confiance. Une dernière fois.",
        ],
        // 8 takes, 35.5 s a la source
        takes: [
            "003c9d9b1a0feb78", // songbird_q306_f_2c54266a4f46a000
            "00a1f9f1d0715f10", // songbird_q304_f_2d6f58d12b40500c
            "0135f29a38ee3a93", // songbird_q301_f_2ac7310a587f6000
            "027631353c483aba", // songbird_q301_f_2db84c0e00571000
            "03910b452d87ff80", // songbird_q302_f_330ff6ab4b45a010
            "045d5d54d037d031", // songbird_q306_f_2de60ca3538ae018
            "049a90e6a809598a", // songbird_q304_f_2d6f454fb240501c
            "04e34ee8b041957a", // songbird_q303_f_3083882298521000
        ],
    },
    #{
        id: "viktor_vector",
        name: "Viktor Vector",
        lines: [
            "Allonge-toi. Ça va piquer un peu.",
            "T'as encore repoussé la limite, hein ?",
            "Paye quand tu peux. Mais paye.",
        ],
        // 6 takes, 32.8 s a la source
        takes: [
            "0250f8cef7698e73", // victor_vector_scene_victor_vector_default_f_16ec5f396728d000
            "025c21e260c303ce", // victor_vector_mq025_f_1af6e425e0404000
            "02827c0eb71a1ec8", // victor_vector_scene_victor_vector_default_f_1bb1e21bcf386000
            "02d86d927ec31c63", // victor_vector_q101_f_1898da10b5386000
            "03b6bb026e00aaf4", // victor_vector_q001_f_16a37e2ad72ef000
            "03ef78f2e904e859", // victor_vector_q001_m_104dd8dd17335000
        ],
    },];

let faites = 0;
for role in CASTING {
    // Le préfixe n'est pas décoratif : le moteur met en cache l'état de clonage sous le seul
    // radical du fichier, et un `judy.wav` d'un autre universe hériterait de celui-ci.
    let fichier = "cp77_" + role.id;

    let monte = voice(#{ name: fichier, takes: role.takes, target: DUREE, min: MINI, max: MAXI });
    if monte.takes == 0 {
        log(role.name + " : aucune prise utilisable, écarté");
        continue;
    }

    // Les répliques sont écrites pour la table, pas reprises du jeu. Cyberpunk ne permet pas la
    // jointure text-audio de StarCraft II : ses sous-titres vivent dans des ressources binaires
    // à part, sans numéro de ligne commun avec le name du `.wem`. Trois phrases par personnage,
    // choisies pour servir à peu près n'importe quelle scène — celles qu'on redit vingt fois
    // dans une soirée, donc celles qui gagnent à être à un clic.
    character(#{
        id: role.id,
        name: role.name,
        universe: UNIVERS,
        voice: fichier + ".wav",
        lines: role.lines,
    });
    faites += 1;
}

log(faites + " personnages sur " + CASTING.len());
