//! L'ATTAQUANT : un VRAI programme malveillant, sur de VRAIES sockets, pour PROUVER
//! la robustesse du protocole — pas un test « en jeu ». Il se connecte au rendez-vous
//! exactement comme un client, récupère la liste des victimes (id, adresse, clé
//! publique), puis lance l'attaque demandée et envoie de vrais paquets forgés.
//!
//! Lancement (après `rendezvous` + au moins une victime : `bot`, `a` ou `b`) :
//!   # Chapitre 5 — attaques DÉJÀ neutralisées (la défense est visible) :
//!   cargo run -- attack forge        # usurpation d'identité (sceau qui ne colle pas)
//!   cargo run -- attack replay       # rejeu d'un vieux paquet
//!   cargo run -- attack flood        # inondation (déni de service)
//!   cargo run -- attack orb-steal    # vol de l'orbe à distance (saut de version)
//!   cargo run -- attack orb-freeze   # gel de l'orbe (version = 65535)
//!
//!   # Chapitre 6 — attaques qui RÉUSSISSENT ENCORE (« rouges », trous à fermer) :
//!   cargo run -- attack teleport     # téléport / speed-hack (état signé, position folle) → 6.3
//!   cargo run -- attack sybil        # banni puis reconnecté avec une identité neuve → 6.2
//!   cargo run -- attack orb-creep    # vol d'orbe par incréments +1 (sous le radar)  → 6.4
//!   cargo run -- attack amplify      # 1 RELAY → la victime rediffuse à tous (réflexion) → 6.5
//!
//! Pour VOIR le résultat, regarde la console des VICTIMES (idéalement des `bot`,
//! qui impriment un « ledger »). Attaques chap. 5 : ignorées ou « 🛡 Faute… /
//! SOURDINE / Inondation… ». Attaques chap. 6 : elles PASSENT aujourd'hui — c'est
//! justement ce qu'on veut rendre visible avant de les fermer.

use super::control::{decode_welcome, encode_hello};
use super::crypto::{Identity, PeerId};
use super::link::rendezvous_addr;
use super::message::{encode_signed, mark_as_relay, PlayerState};
use super::orb::{encode_orb_signed, OrbWire};
use super::punch::encode_punch;
use super::transport::Socket;
use super::wire::{kind, KIND_WELCOME};
use std::net::SocketAddr;
use std::thread::sleep;
use std::time::Duration;

/// Une victime découverte via le rendez-vous : son identité (clé) et son adresse.
/// Depuis le chap. 6.1, l'identité EST la clé — plus besoin de la transporter à part.
type Victim = (PeerId, SocketAddr);

pub fn run_attack(attack: &str) {
    let socket = match Socket::bind(0) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[attaquant] impossible d'ouvrir la prise : {e}");
            return;
        }
    };
    let identity = Identity::generate();
    let rv = rendezvous_addr();
    println!(
        "[attaquant] prise {:?}, rendez-vous {rv}. Attaque demandée : « {attack} ».",
        socket.local_addr().ok()
    );

    // 1) S'inscrire comme un client normal et DÉCOUVRIR les victimes.
    let (my_id, victims) = join_and_discover(&socket, &identity, rv);
    if victims.is_empty() {
        println!("[attaquant] aucune victime à portée. Lance d'abord `rendezvous` puis des clients (`a`, `b`…), puis réessaie.");
        return;
    }
    println!("[attaquant] inscrit (id {}). {} victime(s) : {:?}", my_id.short(), victims.len(),
        victims.iter().map(|(id, _)| *id).collect::<Vec<_>>());
    // On « perce » les victimes comme un vrai client (sur localhost c'est inutile,
    // mais on imite fidèlement le comportement d'un client honnête).
    for (_, addr) in &victims {
        let _ = socket.send_to(*addr, &encode_punch(my_id));
    }

    match attack {
        // --- Chapitre 5 : neutralisées ---
        "forge" | "usurp" => attack_forge(&socket, &identity, my_id, &victims),
        "replay" => attack_replay(&socket, &identity, my_id, &victims),
        "flood" => attack_flood(&socket, &victims),
        "orb-steal" | "orb" => attack_orb(&socket, &identity, my_id, &victims, 60_000),
        "orb-freeze" => attack_orb(&socket, &identity, my_id, &victims, 65_535),
        // --- Chapitre 6 : encore RÉUSSIES (« rouges ») ---
        "teleport" => attack_teleport(&socket, &identity, my_id, &victims),
        "sybil" => attack_sybil(&socket, &identity, my_id, &victims, rv),
        "orb-creep" | "creep" => attack_orb_creep(&socket, &identity, my_id, &victims),
        "amplify" | "amp" => attack_amplify(&socket, &identity, my_id, &victims),
        other => println!(
            "[attaquant] attaque inconnue « {other} ». Chap. 5 : forge | replay | flood | \
             orb-steal | orb-freeze. Chap. 6 : teleport | sybil | orb-creep | amplify."
        ),
    }
}

/// Envoie des HELLO au rendez-vous jusqu'à recevoir notre id et la liste des pairs.
/// Renvoie (notre id, victimes). Patiente quelques secondes pour laisser les autres
/// clients s'inscrire ET pour que le rendez-vous nous diffuse leurs clés publiques.
fn join_and_discover(socket: &Socket, identity: &Identity, rv: SocketAddr) -> (PeerId, Vec<Victim>) {
    let my_id = identity.id(); // notre identité = notre clé, connue dès le départ
    let mut victims: Vec<Victim> = Vec::new();

    for _ in 0..12 {
        let _ = socket.send_to(rv, &encode_hello(0.0, 0.0, my_id));
        sleep(Duration::from_millis(300));
        for (_, bytes) in socket.poll() {
            if kind(&bytes) == Some(KIND_WELCOME) {
                if let Some((_hue, roster)) = decode_welcome(&bytes) {
                    victims = roster;
                }
            }
        }
        if !victims.is_empty() {
            // On insiste un peu pour que les victimes aient eu le temps de nous lister.
            sleep(Duration::from_millis(700));
        }
    }
    (my_id, victims)
}

/// Un état de joueur de base (positions bidon : on ne teste pas le mouvement ici).
fn etat(id: PeerId, seq: u64) -> PlayerState {
    PlayerState {
        id,
        x: 0.0, y: 0.7, z: 0.0,
        vx: 0.0, vy: 0.0, vz: 0.0,
        yaw: 0.0, pitch: 0.0,
        r: 1.0, g: 0.0, b: 0.0,
        parent: None,
        seq,
    }
}

/// ATTAQUE 1 — USURPATION : on envoie un état qui REVENDIQUE l'id d'une victime,
/// mais signé avec NOTRE clé. Le sceau ne correspond pas à la clé publique de la
/// victime → les autres le rejettent, et SANS accuser la victime (le framing est
/// impossible). Défense visible : … rien. C'est justement la preuve : l'imposteur
/// n'a STRICTEMENT aucun effet.
fn attack_forge(socket: &Socket, identity: &Identity, _my_id: PeerId, victims: &[Victim]) {
    let cible = victims[0].0;
    println!("[attaquant] USURPATION : je me fais passer pour le joueur {} (signé avec MA clé).", cible.short());
    let forged = encode_signed(&etat(cible, 1), identity); // id = victime, sceau = attaquant
    for _ in 0..20 {
        for (_, addr) in victims {
            let _ = socket.send_to(*addr, &forged);
        }
        sleep(Duration::from_millis(100));
    }
    println!("[attaquant] 20 salves envoyées. Côté victimes : AUCUN effet (sceau invalide → jeté).");
    println!("            → l'usurpation est neutralisée par la signature (chap. 5.1).");
}

/// ATTAQUE 2 — REJEU : on envoie un VRAI paquet signé (seq=100), puis on le REJOUE
/// (même seq, puis seq plus ancien). Les victimes acceptent le 1er, refusent les
/// rejeus (compteur anti-rejeu). On ne peut donc pas « rembobiner » un joueur.
fn attack_replay(socket: &Socket, identity: &Identity, my_id: PeerId, victims: &[Victim]) {
    println!("[attaquant] REJEU : j'émets un état valide (seq=100), puis je le rejoue (seq=100, puis 50).");
    let frais = encode_signed(&etat(my_id, 100), identity);
    let rejeu_meme = encode_signed(&etat(my_id, 100), identity);
    let rejeu_vieux = encode_signed(&etat(my_id, 50), identity);
    for (_, addr) in victims {
        let _ = socket.send_to(*addr, &frais); // accepté (seq neuf)
    }
    sleep(Duration::from_millis(200));
    for _ in 0..10 {
        for (_, addr) in victims {
            let _ = socket.send_to(*addr, &rejeu_meme); // refusé (seq déjà vu)
            let _ = socket.send_to(*addr, &rejeu_vieux); // refusé (seq périmé)
        }
        sleep(Duration::from_millis(100));
    }
    println!("[attaquant] rejeus envoyés. Côté victimes : le 1er passe, les rejeus sont jetés (seq ≤ dernier vu).");
}

/// ATTAQUE 3 — INONDATION : on noie une victime sous des milliers de paquets pour
/// saturer son CPU. Le « seau à jetons » par adresse jette l'excès. Défense visible :
/// la victime affiche « 🛡 Inondation détectée… ».
fn attack_flood(socket: &Socket, victims: &[Victim]) {
    let (_, addr) = victims[0];
    println!("[attaquant] INONDATION : j'envoie 20 000 paquets à {addr} aussi vite que possible.");
    let junk = [0u8; 64];
    for _ in 0..20_000 {
        let _ = socket.send_to(addr, &junk);
    }
    println!("[attaquant] terminé. Côté victime : « 🛡 Inondation détectée… » (excès jeté par le rate-limit).");
}

/// ATTAQUE 4 — VOL / GEL DE L'ORBE : on se proclame maître de l'orbe avec un SAUT
/// de version énorme (vol à distance, ou 65535 pour la verrouiller à vie). Le paquet
/// est VALIDEMENT signé (par notre clé), donc la faute est ATTRIBUABLE : les victimes
/// refusent le saut aberrant ET nous infligent un strike. Au bout de MAX_STRIKES,
/// elles nous mettent en SOURDINE. Défense visible : « 🛡 Faute… » puis « SOURDINE ».
fn attack_orb(socket: &Socket, identity: &Identity, my_id: PeerId, victims: &[Victim], version: u16) {
    let quoi = if version == 65_535 { "GEL (version 65535)" } else { "VOL (saut de version)" };
    println!("[attaquant] ORBE — {quoi} : je me proclame maître {} avec version {version}.", my_id.short());
    let w = OrbWire {
        owner: my_id, version,
        x: 0.0, y: 1.5, z: 0.0, vx: 0.0, vy: 0.0, vz: 0.0,
        r: 1.0, g: 0.0, b: 0.0,
    };
    let bytes = encode_orb_signed(&w, identity);
    for _ in 0..10 {
        for (_, addr) in victims {
            let _ = socket.send_to(*addr, &bytes);
        }
        sleep(Duration::from_millis(150));
    }
    println!("[attaquant] 10 tentatives envoyées. Côté victimes : saut refusé + « 🛡 Faute… » → « SOURDINE ».");
    println!("            → l'orbe ne peut être ni volée à distance ni gelée (chap. 5.3).");
}

// ============================================================================
// CHAPITRE 6 — attaques « ROUGES » : elles RÉUSSISSENT encore aujourd'hui. Chacune
// est une preuve concrète d'un trou de l'audit, à fermer dans une étape dédiée.
// ============================================================================

/// ATTAQUE 5 (ROUGE) — TÉLÉPORT / SPEED-HACK : on émet des états VALIDEMENT signés
/// (par NOTRE clé, donc parfaitement authentiques) mais avec des sauts de position
/// physiquement impossibles. La signature prouve QUI ; elle ne dit RIEN sur la
/// plausibilité du mouvement. Aucune borne de vitesse côté récepteur → tout passe.
/// Trou n°7, à fermer au chapitre 6.3 (validation de mouvement).
fn attack_teleport(socket: &Socket, identity: &Identity, my_id: PeerId, victims: &[Victim]) {
    println!("[attaquant] TÉLÉPORT : états signés par MOI, mais positions folles (0 → 1000 m d'un coup).");
    let sauts = [0.0f32, 50.0, 200.0, 1000.0, -1000.0, 0.0];
    let mut seq = 1u64;
    for (k, px) in sauts.iter().enumerate() {
        let mut p = etat(my_id, seq);
        p.x = *px;
        p.z = *px;
        seq += 1;
        let bytes = encode_signed(&p, identity);
        for (_, addr) in victims {
            let _ = socket.send_to(*addr, &bytes);
        }
        println!("[attaquant]   saut #{k} → x={px} m (téléport instantané).");
        sleep(Duration::from_millis(300));
    }
    println!("[attaquant] Côté victimes (depuis 6.3) : le 1er point passe, puis chaque saut");
    println!("            impossible est REFUSÉ + « 🛡 Faute… » → SOURDINE. Trou n°7 FERMÉ.");
}

/// ATTAQUE 6 (ROUGE) — SYBIL : l'identité est GRATUITE. On triche jusqu'au
/// bannissement, puis on revient avec une clé toute neuve, comme si de rien n'était.
/// La réputation/sourdine ne coûte donc rien à contourner. Trou n°6, à fermer au
/// chapitre 6.2 (coût d'entrée anti-Sybil).
fn attack_sybil(socket: &Socket, identity: &Identity, my_id: PeerId, victims: &[Victim], rv: SocketAddr) {
    println!("[attaquant] SYBIL — phase 1 : je me fais BANNIR (gros saut de version d'orbe).");
    let w = OrbWire {
        owner: my_id, version: 60_000,
        x: 0.0, y: 1.5, z: 0.0, vx: 0.0, vy: 0.0, vz: 0.0,
        r: 1.0, g: 0.0, b: 0.0,
    };
    let bytes = encode_orb_signed(&w, identity);
    for _ in 0..8 {
        for (_, addr) in victims {
            let _ = socket.send_to(*addr, &bytes);
        }
        sleep(Duration::from_millis(150));
    }
    println!("[attaquant] SYBIL — phase 2 : je JETTE cette identité et j'en génère une NEUVE (coût ≈ 0).");
    let socket2 = match Socket::bind(0) {
        Ok(s) => s,
        Err(e) => {
            println!("[attaquant]   (impossible d'ouvrir une 2e prise : {e})");
            return;
        }
    };
    let id2 = Identity::generate();
    let (my_id2, victims2) = join_and_discover(&socket2, &id2, rv);
    if victims2.is_empty() {
        println!("[attaquant]   (pas de victimes à la reconnexion)");
        return;
    }
    println!("[attaquant]   reconnecté sous le NOUVEL id {}. J'émets un état tout propre.", my_id2.short());
    let clean = encode_signed(&etat(my_id2, 1), &id2);
    for _ in 0..10 {
        for (_, addr) in &victims2 {
            let _ = socket2.send_to(*addr, &clean);
        }
        sleep(Duration::from_millis(150));
    }
    println!("[attaquant] Côté victimes : l'ANCIEN id reste en sourdine, mais le NOUVEL id est accepté normalement.");
    println!("            → trou n°6 (bannissement gratuit à contourner) à fermer au chap. 6.2.");
}

/// ATTAQUE 7 (ROUGE) — ORB-CREEP : on vole l'orbe par incréments de +1. Chaque pas
/// est « plausible » (saut ≤ 16 → pas de faute), donc on grimpe tranquillement la
/// version jusqu'à devenir maître, SANS jamais toucher l'orbe et SANS alerter. À
/// comparer avec `orb-steal` (gros saut = faute immédiate). Trou n°8, chap. 6.4.
fn attack_orb_creep(socket: &Socket, identity: &Identity, my_id: PeerId, victims: &[Victim]) {
    println!("[attaquant] ORBE-CREEP : je grimpe la version +1 à la fois (chaque pas ≤16 → AUCUNE faute).");
    for v in 1..=30u16 {
        let w = OrbWire {
            owner: my_id, version: v,
            x: 0.0, y: 1.5, z: 0.0, vx: 0.0, vy: 0.0, vz: 0.0,
            r: 0.0, g: 1.0, b: 0.0,
        };
        let bytes = encode_orb_signed(&w, identity);
        for (_, addr) in victims {
            let _ = socket.send_to(*addr, &bytes);
        }
        sleep(Duration::from_millis(120));
    }
    println!("[attaquant] 30 pas (+1) envoyés. Côté victimes : orbe ADOPTÉE pas à pas, zéro « 🛡 Faute ».");
    println!("            → trou n°8 (vol d'orbe lent, sans preuve de contact) à fermer au chap. 6.4.");
}

/// ATTAQUE 8 (ROUGE) — AMPLIFICATION : on envoie nos paquets sous forme RELAY à UN
/// seul pair (un « parent »). Comme un parent recopie ce qu'il reçoit à TOUS ses
/// voisins, 1 paquet entrant devient N sortants : c'est l'upload de la VICTIME qui
/// amplifie notre attaque (réflexion). Trou n°10, à fermer au chapitre 6.5
/// (consentement du relais + AoI sur la rediffusion).
fn attack_amplify(socket: &Socket, identity: &Identity, my_id: PeerId, victims: &[Victim]) {
    let (_, parent_addr) = victims[0];
    println!("[attaquant] AMPLIFICATION : j'envoie des RELAY à 1 SEUL pair ; il les recopie à TOUS ses voisins.");
    let mut seq = 1u64;
    for _ in 0..10 {
        seq += 1; // seq croissant : chaque RELAY est accepté puis rediffusé
        let mut sealed = encode_signed(&etat(my_id, seq), identity);
        mark_as_relay(&mut sealed);
        let _ = socket.send_to(parent_addr, &sealed);
        sleep(Duration::from_millis(200));
    }
    println!("[attaquant] 10 RELAY envoyés à 1 pair. Côté parent : « ↪ RELAY … recopié à N pairs » (1 entrée → N sorties).");
    println!("            → trou n°10 (amplification réfléchie) à fermer au chap. 6.5.");
}
