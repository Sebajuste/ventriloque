// StarCraft II — la recette du paquet de voice.
//
// Ce script tourne dans `fabriquer.exe`, pas dans Ventriloque. Il n'a aucune entrée-sortie : il
// nomme des entrées d'archive et des personnages, l'hôte lit, décode, monte et écrit.
//
// CE QUI REND CE PAQUET POSSIBLE. Le doublage porte un numéro de ligne que la table de
// sous-titres porte aussi :
//
//     campaigns/…/vo/zbriefing_char03_kerrigan_007.ogg
//     Conversation/zBriefing_Char03/Line00007=Soldats, l'Essaim de Kerrigan est sur le plateau.
//
// Une jointure sur un name de fichier, et l'on sait ce que dit chaque prise. C'est ce qui permet
// de ne retenir que des phrases — pas un cri, pas un râle, pas un « oui, commandant » — et de
// remplir les répliques des fiches avec le text exact du jeu.

const UNIVERS = "StarCraft II";

// Le jeton du locuteur est celui qui précède le numéro dans le name du fichier. Plusieurs jetons
// désignent parfois le même personnage, selon le mode de jeu.
const CASTING = [
    #{ id: "sarah_kerrigan",  name: "Sarah Kerrigan",   jetons: ["kerrigan", "kerrigancommander"] },
    #{ id: "jim_raynor",      name: "Jim Raynor",       jetons: ["raynor", "raynorcommander"] },
    #{ id: "artanis",         name: "Artanis",          jetons: ["artanis", "artaniscommander"] },
    #{ id: "nova",            name: "Nova",             jetons: ["nova", "novacommander"] },
    #{ id: "alexei_stukov",   name: "Alexei Stukov",    jetons: ["stukov", "stukovcommander"] },
    #{ id: "zeratul",         name: "Zeratul",          jetons: ["zeratul", "zeratulac", "zeratulcommander"] },
    #{ id: "alarak",          name: "Alarak",           jetons: ["alarak", "alarakcommander"] },
    #{ id: "dehaka",          name: "Dehaka",           jetons: ["dehaka", "dehakacommander"] },
    #{ id: "abathur",         name: "Abathur",          jetons: ["abathur", "abathurcommander"] },
    #{ id: "valerian_mengsk", name: "Valerian Mengsk",  jetons: ["valerian"] },
    #{ id: "arcturus_mengsk", name: "Arcturus Mengsk",  jetons: ["mengsk", "mengskcommander"] },
    #{ id: "zagara",          name: "Zagara",           jetons: ["zagara", "zagaracommander"] },
    #{ id: "rory_swann",      name: "Rory Swann",       jetons: ["swann", "swanncommander"] },
    #{ id: "tychus_findlay",  name: "Tychus Findlay",   jetons: ["tychus", "tychuscommander"] },
    #{ id: "matt_horner",     name: "Matt Horner",      jetons: ["horner", "hornerhan"] },
    #{ id: "amon",            name: "Amon",             jetons: ["amon"] },
    #{ id: "izsha",           name: "Izsha",            jetons: ["izsha"] },
    #{ id: "vorazun",         name: "Vorazun",          jetons: ["vorazun", "vorazuncommander"] },
    #{ id: "karax",           name: "Karax",            jetons: ["karax", "karaxcommander"] },
    #{ id: "egon_stetmann",   name: "Egon Stetmann",    jetons: ["stetmann", "stetmanncommander"] },
];

// Fenêtre de size du .ogg, en octets. Le doublage tourne autour de 18 ko la seconde : sous
// 20 ko c'est une interjection, au-dessus de 70 ko un monologue qu'il faudrait couper. Filtrer
// ici évite de décoder des milliers de fichiers pour n'en garder que quarante.
const TAILLE_MIN = 20000;
const TAILLE_MAX = 70000;
// Le repli des personnages qui ne parlent jamais court : Amon n'a pas une réplique sous les
// quatre secondes. Mieux vaut une référence en phrases longues que pas de voice du tout.
const TAILLE_LARGE = 200000;
const PRISES_MINIMUM = 6;
const CANDIDATS = 45;

if product() != "s2" {
    throw "cette recette est celle de StarCraft II, et ce stockage annonce « " + product() + " »";
}

// ---------------------------------------------------------------- les sous-titres

// `Conversation/<scène>/Line<numéro>` → le text français. Une même clé vue avec deux textes
// différents veut log que deux mods partagent un name de scène : on la retire plutôt que de
// risquer de coller la mauvaise phrase sur une prise.
let textes = #{};
let ambigues = #{};
let tables = list("*frfr.sc2data/localizeddata/conversationstrings.txt");
log(tables.len() + " tables de sous-titres");

for table in tables {
    for ligne in text(table).split("\n") {
        if !ligne.starts_with("Conversation/") { continue; }
        let egal = ligne.index_of("=");
        if egal < 0 { continue; }

        let cle = ligne.sub_string(0, egal);
        let morceaux = cle.split("/");
        if morceaux.len() != 3 { continue; }
        let dernier = morceaux[2];
        if !dernier.starts_with("Line") { continue; }

        // « Line00007 » → 7. La prise, elle, écrit 007 ou 0007 : c'est le nombre qui joint.
        let numero = nombre(dernier.sub_string(4, dernier.len() - 4));
        if numero == () { continue; }
        let identite = morceaux[1].to_lower() + "|" + numero;

        // La table porte « français /// anglais » sur une seule ligne.
        let valeur = ligne.sub_string(egal + 1, ligne.len() - egal - 1);
        let separateur = valeur.index_of(" /// ");
        if separateur >= 0 { valeur = valeur.sub_string(0, separateur); }
        // `trim` modifie sur place et ne rend rien : l'appeler dans une affectation mettrait
        // l'unite dans la table, et la jointure echouerait sans log un mot.
        valeur.trim();

        if textes[identite] != () && textes[identite] != valeur {
            ambigues[identite] = true;
        }
        textes[identite] = valeur;
    }
}
for identite in ambigues.keys() { textes.remove(identite); }
log(textes.len() + " sous-titres (" + ambigues.len() + " clés ambiguës écartées)");

// ---------------------------------------------------------------- les takes

// `parse_int` lève sur ce qui n'est pas un nombre, et beaucoup de noms de takes finissent par
// un mot — `vespenegiven01` n'est pas un numéro de ligne. On regarde avant de convertir.
fn nombre(s) {
    if s.is_empty() { return (); }
    for c in s.chars() {
        if "0123456789".index_of(c) < 0 { return (); }
    }
    return parse_int(s);
}

// Une réplique sert si c'est une phrase, pas un fragment de gabarit d'interface.
fn utilisable(t) {
    if t.len() < 30 || t.len() > 160 { return false; }
    if t.contains("~") || t.contains("<") || t.contains(">") || t.contains("|") { return false; }
    let fin = t.sub_string(t.len() - 1, 1);
    return fin == "." || fin == "!" || fin == "?" || fin == "…";
}

// Chaque prise dont on connaît le text, avec son locuteur. Un seul parcours des 28 000 .ogg
// français ; les personnages piochent ensuite dans cette liste.
let takes = [];
for name in list("*frfr.sc2assets*.ogg") {
    let octets = size(name);
    if octets < TAILLE_MIN || octets > TAILLE_LARGE { continue; }

    let fichier = name.split("/");
    let radical = fichier[fichier.len() - 1];
    radical = radical.sub_string(0, radical.len() - 4);

    let bouts = radical.split("_");
    if bouts.len() < 3 { continue; }

    let numero = nombre(bouts[bouts.len() - 1]);
    if numero == () { continue; }
    let jeton = bouts[bouts.len() - 2];

    // La scène est tout ce qui précède le locuteur et le numéro.
    let scene = "";
    for i in 0..(bouts.len() - 2) {
        if i > 0 { scene += "_"; }
        scene += bouts[i];
    }

    let dit = textes[scene + "|" + numero];
    if dit == () || !utilisable(dit) { continue; }

    takes.push(#{ jeton: jeton, name: name, octets: octets, dit: dit });
}
log(takes.len() + " takes dont le text est connu");

// ---------------------------------------------------------------- les voice

let faites = 0;
for role in CASTING {
    // Deux passes : la fenêtre étroite d'abord, la large seulement si le personnage n'a pas
    // assez de répliques courtes.
    let retenus = [];
    let phrases = [];
    let duree_max = 4.0;

    for large in [false, true] {
        if retenus.len() >= PRISES_MINIMUM { break; }
        retenus = [];
        phrases = [];
        let vus = #{};
        let plafond = if large { TAILLE_LARGE } else { TAILLE_MAX };
        if large { duree_max = 9.0; }

        for prise in takes {
            if retenus.len() >= CANDIDATS { break; }
            if prise.octets > plafond { continue; }
            if !role.jetons.contains(prise.jeton) { continue; }
            if vus[prise.dit] != () { continue; }
            vus[prise.dit] = true;
            retenus.push(prise.name);
            phrases.push(prise.dit);
        }
    }

    if retenus.is_empty() {
        log(role.name + " : aucune prise, écarté");
        continue;
    }

    // Le préfixe n'est pas décoratif : le moteur met en cache l'état de clonage sous le seul
    // radical du fichier, et un `kerrigan.wav` d'un autre universe hériterait de celui-ci.
    let fichier = "sc2_" + role.id;
    voice(#{ name: fichier, takes: retenus, target: 32.0, min: 1.0, max: duree_max });

    // Trois répliques pour la table. Sous trente-cinq signes c'est un ordre de combat, au-dessus
    // de cent un discours : le milieu est ce qui se rejoue.
    let courtes = [];
    for p in phrases {
        if courtes.len() >= 3 { break; }
        if p.len() >= 35 && p.len() <= 100 { courtes.push(p); }
    }
    if courtes.is_empty() {
        for p in phrases { if courtes.len() < 3 { courtes.push(p); } }
    }

    character(#{
        id: role.id,
        name: role.name,
        universe: UNIVERS,
        voice: fichier + ".wav",
        lines: courtes,
    });
    faites += 1;
}

log(faites + " personnages sur " + CASTING.len());
