// StarCraft II — la recette du paquet de voix.
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
// Une jointure sur un nom de fichier, et l'on sait ce que dit chaque prise. C'est ce qui permet
// de ne retenir que des phrases — pas un cri, pas un râle, pas un « oui, commandant » — et de
// remplir les répliques des fiches avec le texte exact du jeu.

const UNIVERS = "StarCraft II";

// Le jeton du locuteur est celui qui précède le numéro dans le nom du fichier. Plusieurs jetons
// désignent parfois le même personnage, selon le mode de jeu.
const CASTING = [
    #{ id: "sarah_kerrigan",  nom: "Sarah Kerrigan",   jetons: ["kerrigan", "kerrigancommander"] },
    #{ id: "jim_raynor",      nom: "Jim Raynor",       jetons: ["raynor", "raynorcommander"] },
    #{ id: "artanis",         nom: "Artanis",          jetons: ["artanis", "artaniscommander"] },
    #{ id: "nova",            nom: "Nova",             jetons: ["nova", "novacommander"] },
    #{ id: "alexei_stukov",   nom: "Alexei Stukov",    jetons: ["stukov", "stukovcommander"] },
    #{ id: "zeratul",         nom: "Zeratul",          jetons: ["zeratul", "zeratulac", "zeratulcommander"] },
    #{ id: "alarak",          nom: "Alarak",           jetons: ["alarak", "alarakcommander"] },
    #{ id: "dehaka",          nom: "Dehaka",           jetons: ["dehaka", "dehakacommander"] },
    #{ id: "abathur",         nom: "Abathur",          jetons: ["abathur", "abathurcommander"] },
    #{ id: "valerian_mengsk", nom: "Valerian Mengsk",  jetons: ["valerian"] },
    #{ id: "arcturus_mengsk", nom: "Arcturus Mengsk",  jetons: ["mengsk", "mengskcommander"] },
    #{ id: "zagara",          nom: "Zagara",           jetons: ["zagara", "zagaracommander"] },
    #{ id: "rory_swann",      nom: "Rory Swann",       jetons: ["swann", "swanncommander"] },
    #{ id: "tychus_findlay",  nom: "Tychus Findlay",   jetons: ["tychus", "tychuscommander"] },
    #{ id: "matt_horner",     nom: "Matt Horner",      jetons: ["horner", "hornerhan"] },
    #{ id: "amon",            nom: "Amon",             jetons: ["amon"] },
    #{ id: "izsha",           nom: "Izsha",            jetons: ["izsha"] },
    #{ id: "vorazun",         nom: "Vorazun",          jetons: ["vorazun", "vorazuncommander"] },
    #{ id: "karax",           nom: "Karax",            jetons: ["karax", "karaxcommander"] },
    #{ id: "egon_stetmann",   nom: "Egon Stetmann",    jetons: ["stetmann", "stetmanncommander"] },
];

// Fenêtre de taille du .ogg, en octets. Le doublage tourne autour de 18 ko la seconde : sous
// 20 ko c'est une interjection, au-dessus de 70 ko un monologue qu'il faudrait couper. Filtrer
// ici évite de décoder des milliers de fichiers pour n'en garder que quarante.
const TAILLE_MIN = 20000;
const TAILLE_MAX = 70000;
// Le repli des personnages qui ne parlent jamais court : Amon n'a pas une réplique sous les
// quatre secondes. Mieux vaut une référence en phrases longues que pas de voix du tout.
const TAILLE_LARGE = 200000;
const PRISES_MINIMUM = 6;
const CANDIDATS = 45;

if produit() != "s2" {
    throw "cette recette est celle de StarCraft II, et ce stockage annonce « " + produit() + " »";
}

// ---------------------------------------------------------------- les sous-titres

// `Conversation/<scène>/Line<numéro>` → le texte français. Une même clé vue avec deux textes
// différents veut dire que deux mods partagent un nom de scène : on la retire plutôt que de
// risquer de coller la mauvaise phrase sur une prise.
let textes = #{};
let ambigues = #{};
let tables = lister("*frfr.sc2data/localizeddata/conversationstrings.txt");
dire(tables.len() + " tables de sous-titres");

for table in tables {
    for ligne in texte(table).split("\n") {
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
        // l'unite dans la table, et la jointure echouerait sans dire un mot.
        valeur.trim();

        if textes[identite] != () && textes[identite] != valeur {
            ambigues[identite] = true;
        }
        textes[identite] = valeur;
    }
}
for identite in ambigues.keys() { textes.remove(identite); }
dire(textes.len() + " sous-titres (" + ambigues.len() + " clés ambiguës écartées)");

// ---------------------------------------------------------------- les prises

// `parse_int` lève sur ce qui n'est pas un nombre, et beaucoup de noms de prises finissent par
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

// Chaque prise dont on connaît le texte, avec son locuteur. Un seul parcours des 28 000 .ogg
// français ; les personnages piochent ensuite dans cette liste.
let prises = [];
for nom in lister("*frfr.sc2assets*.ogg") {
    let octets = taille(nom);
    if octets < TAILLE_MIN || octets > TAILLE_LARGE { continue; }

    let fichier = nom.split("/");
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

    prises.push(#{ jeton: jeton, nom: nom, octets: octets, dit: dit });
}
dire(prises.len() + " prises dont le texte est connu");

// ---------------------------------------------------------------- les voix

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

        for prise in prises {
            if retenus.len() >= CANDIDATS { break; }
            if prise.octets > plafond { continue; }
            if !role.jetons.contains(prise.jeton) { continue; }
            if vus[prise.dit] != () { continue; }
            vus[prise.dit] = true;
            retenus.push(prise.nom);
            phrases.push(prise.dit);
        }
    }

    if retenus.is_empty() {
        dire(role.nom + " : aucune prise, écarté");
        continue;
    }

    // Le préfixe n'est pas décoratif : le moteur met en cache l'état de clonage sous le seul
    // radical du fichier, et un `kerrigan.wav` d'un autre univers hériterait de celui-ci.
    let fichier = "sc2_" + role.id;
    voix(#{ nom: fichier, prises: retenus, duree: 32.0, min: 1.0, max: duree_max });

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

    fiche(#{
        id: role.id,
        nom: role.nom,
        univers: UNIVERS,
        voix: fichier + ".wav",
        repliques: courtes,
    });
    faites += 1;
}

dire(faites + " personnages sur " + CASTING.len());
