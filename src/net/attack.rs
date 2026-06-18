//! L'ATTAQUANT : un VRAI programme malveillant, sur de VRAIES sockets, pour PROUVER
//! la robustesse du protocole — pas un test « en jeu ». Il se connecte au rendez-vous
//! exactement comme un client, récupère la liste des victimes (id, adresse, clé
//! publique), puis lance l'attaque demandée et envoie de vrais paquets forgés.
//!
//! Lancement (après `rendezvous` + au moins un client `a`/`b`) :
//!   cargo run -- attack forge        # usurpation d'identité (sceau qui ne colle pas)
//!   cargo run -- attack replay       # rejeu d'un vieux paquet
//!   cargo run -- attack flood        # inondation (déni de service)
//!   cargo run -- attack orb-steal    # vol de l'orbe à distance (saut de version)
//!   cargo run -- attack orb-freeze   # gel de l'orbe (version = 65535)
//!
//! Pour VOIR la défense, regarde la console des CLIENTS-victimes : selon l'attaque,
//! soit ils ignorent en silence (sceau invalide), soit ils affichent « 🛡 Faute… »
//! puis « 🛡 … SOURDINE » (l'attaquant est banni localement), soit « 🛡 Inondation… ».

use super::control::{decode_welcome, encode_hello};
use super::crypto::Identity;
use super::link::rendezvous_addr;
use super::message::{encode_signed, PlayerState};
use super::orb::{encode_orb_signed, OrbWire};
use super::punch::encode_punch;
use super::transport::Socket;
use super::wire::{kind, KIND_WELCOME};
use std::net::SocketAddr;
use std::thread::sleep;
use std::time::Duration;

/// Une victime découverte via le rendez-vous : son id, son adresse, sa clé publique.
type Victim = (u8, SocketAddr, [u8; 32]);

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
    println!("[attaquant] inscrit (id {my_id}). {} victime(s) : {:?}", victims.len(),
        victims.iter().map(|(id, _, _)| *id).collect::<Vec<_>>());
    // On « perce » les victimes comme un vrai client (sur localhost c'est inutile,
    // mais on imite fidèlement le comportement d'un client honnête).
    for (_, addr, _) in &victims {
        let _ = socket.send_to(*addr, &encode_punch(my_id));
    }

    match attack {
        "forge" | "usurp" => attack_forge(&socket, &identity, my_id, &victims),
        "replay" => attack_replay(&socket, &identity, my_id, &victims),
        "flood" => attack_flood(&socket, &victims),
        "orb-steal" | "orb" => attack_orb(&socket, &identity, my_id, &victims, 60_000),
        "orb-freeze" => attack_orb(&socket, &identity, my_id, &victims, 65_535),
        other => println!(
            "[attaquant] attaque inconnue « {other} ». Choix : forge | replay | flood | orb-steal | orb-freeze"
        ),
    }
}

/// Envoie des HELLO au rendez-vous jusqu'à recevoir notre id et la liste des pairs.
/// Renvoie (notre id, victimes). Patiente quelques secondes pour laisser les autres
/// clients s'inscrire ET pour que le rendez-vous nous diffuse leurs clés publiques.
fn join_and_discover(socket: &Socket, identity: &Identity, rv: SocketAddr) -> (u8, Vec<Victim>) {
    let pubkey = identity.public();
    let mut my_id = 0u8;
    let mut victims: Vec<Victim> = Vec::new();

    for _ in 0..12 {
        let _ = socket.send_to(rv, &encode_hello(0.0, 0.0, &pubkey));
        sleep(Duration::from_millis(300));
        for (_, bytes) in socket.poll() {
            if kind(&bytes) == Some(KIND_WELCOME) {
                if let Some((id, _hue, roster)) = decode_welcome(&bytes) {
                    my_id = id;
                    victims = roster;
                }
            }
        }
        if !victims.is_empty() {
            // On insiste encore un peu pour que les victimes aient AUSSI reçu notre
            // clé (sinon elles ignoreraient nos paquets « inconnus » sans nous juger).
            sleep(Duration::from_millis(700));
        }
    }
    (my_id, victims)
}

/// Un état de joueur de base (positions bidon : on ne teste pas le mouvement ici).
fn etat(id: u8, seq: u64) -> PlayerState {
    PlayerState {
        id,
        x: 0.0, y: 0.7, z: 0.0,
        vx: 0.0, vy: 0.0, vz: 0.0,
        yaw: 0.0, pitch: 0.0,
        r: 1.0, g: 0.0, b: 0.0,
        parent: 0,
        seq,
    }
}

/// ATTAQUE 1 — USURPATION : on envoie un état qui REVENDIQUE l'id d'une victime,
/// mais signé avec NOTRE clé. Le sceau ne correspond pas à la clé publique de la
/// victime → les autres le rejettent, et SANS accuser la victime (le framing est
/// impossible). Défense visible : … rien. C'est justement la preuve : l'imposteur
/// n'a STRICTEMENT aucun effet.
fn attack_forge(socket: &Socket, identity: &Identity, _my_id: u8, victims: &[Victim]) {
    let cible = victims[0].0;
    println!("[attaquant] USURPATION : je me fais passer pour le joueur {cible} (signé avec MA clé).");
    let forged = encode_signed(&etat(cible, 1), identity); // id = victime, sceau = attaquant
    for _ in 0..20 {
        for (_, addr, _) in victims {
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
fn attack_replay(socket: &Socket, identity: &Identity, my_id: u8, victims: &[Victim]) {
    println!("[attaquant] REJEU : j'émets un état valide (seq=100), puis je le rejoue (seq=100, puis 50).");
    let frais = encode_signed(&etat(my_id, 100), identity);
    let rejeu_meme = encode_signed(&etat(my_id, 100), identity);
    let rejeu_vieux = encode_signed(&etat(my_id, 50), identity);
    for (_, addr, _) in victims {
        let _ = socket.send_to(*addr, &frais); // accepté (seq neuf)
    }
    sleep(Duration::from_millis(200));
    for _ in 0..10 {
        for (_, addr, _) in victims {
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
    let (_, addr, _) = victims[0];
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
fn attack_orb(socket: &Socket, identity: &Identity, my_id: u8, victims: &[Victim], version: u16) {
    let quoi = if version == 65_535 { "GEL (version 65535)" } else { "VOL (saut de version)" };
    println!("[attaquant] ORBE — {quoi} : je me proclame maître {my_id} avec version {version}.");
    let w = OrbWire {
        owner: my_id, version,
        x: 0.0, y: 1.5, z: 0.0, vx: 0.0, vy: 0.0, vz: 0.0,
        r: 1.0, g: 0.0, b: 0.0,
    };
    let bytes = encode_orb_signed(&w, identity);
    for _ in 0..10 {
        for (_, addr, _) in victims {
            let _ = socket.send_to(*addr, &bytes);
        }
        sleep(Duration::from_millis(150));
    }
    println!("[attaquant] 10 tentatives envoyées. Côté victimes : saut refusé + « 🛡 Faute… » → « SOURDINE ».");
    println!("            → l'orbe ne peut être ni volée à distance ni gelée (chap. 5.3).");
}
