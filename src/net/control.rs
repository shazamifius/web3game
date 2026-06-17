//! L'ANNUAIRE : les messages échangés avec le serveur de rendez-vous.
//!
//!   - HELLO   (client → serveur) : « je suis là ». Le serveur lit l'adresse
//!     source du paquet, donc pas besoin de la mettre dedans (1 octet suffit).
//!   - WELCOME (serveur → client) : « ton identifiant, et voici les autres ».

use super::crypto::PUBKEY_LEN;
use super::wire::{KIND_HELLO, KIND_WELCOME, PROTO_VERSION};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

/// Taille d'un HELLO : type + version + x(4) + z(4) + clé publique (32) = 42.
const HELLO_SIZE: usize = 2 + 4 + 4 + PUBKEY_LEN;

/// Fabrique un paquet HELLO : type + version + POSITION (x, z) + notre CLÉ PUBLIQUE.
/// La position sert à l'AoI ; la clé publique fait du rendez-vous un annuaire
/// d'IDENTITÉS (il la redistribue pour que chacun puisse vérifier nos signatures).
pub(crate) fn encode_hello(x: f32, z: f32, pubkey: &[u8; PUBKEY_LEN]) -> [u8; HELLO_SIZE] {
    let mut b = [0u8; HELLO_SIZE];
    b[0] = KIND_HELLO;
    b[1] = PROTO_VERSION;
    b[2..6].copy_from_slice(&x.to_le_bytes());
    b[6..10].copy_from_slice(&z.to_le_bytes());
    b[10..10 + PUBKEY_LEN].copy_from_slice(pubkey);
    b
}

/// Lit un paquet HELLO : renvoie la position (x, z) et la clé publique annoncées.
pub(crate) fn decode_hello(buf: &[u8]) -> Option<(f32, f32, [u8; PUBKEY_LEN])> {
    if buf.len() < HELLO_SIZE || buf[0] != KIND_HELLO || buf[1] != PROTO_VERSION {
        return None;
    }
    let x = f32::from_le_bytes(buf[2..6].try_into().ok()?);
    let z = f32::from_le_bytes(buf[6..10].try_into().ok()?);
    let mut pubkey = [0u8; PUBKEY_LEN];
    pubkey.copy_from_slice(&buf[10..10 + PUBKEY_LEN]);
    Some((x, z, pubkey))
}

/// Une entrée d'annuaire : id (1) + ip (4) + port (2) + clé publique (32) = 39 octets.
const ROSTER_ENTRY_SIZE: usize = 1 + 4 + 2 + PUBKEY_LEN;

/// Type d'une fiche de pair dans le roster : son id, son adresse, et sa CLÉ PUBLIQUE
/// (son identité — c'est elle qui permettra de vérifier ses signatures).
pub(crate) type RosterEntry = (u8, SocketAddr, [u8; PUBKEY_LEN]);

/// Fabrique un paquet WELCOME : type + version + ton_id + teinte_du_monde (2o) +
/// nombre + [ (id, ip, port, clé publique) … ]. La teinte (0–359) est la couleur de
/// salle PARTAGÉE par tous les joueurs. (IPv4 uniquement — tout est sur 127.0.0.1.)
pub(crate) fn encode_welcome(your_id: u8, world_hue: u16, roster: &[RosterEntry]) -> Vec<u8> {
    // On ne garde que les adresses IPv4 (les seules qu'on sait encoder).
    let v4: Vec<(u8, SocketAddrV4, &[u8; PUBKEY_LEN])> = roster
        .iter()
        .filter_map(|(id, addr, pk)| match addr {
            SocketAddr::V4(a) => Some((*id, *a, pk)),
            SocketAddr::V6(_) => None,
        })
        .collect();

    let mut buf = vec![KIND_WELCOME, PROTO_VERSION, your_id];
    buf.extend_from_slice(&world_hue.to_le_bytes()); // teinte du monde (2 octets)
    buf.push(v4.len() as u8);
    for (id, addr, pk) in v4 {
        buf.push(id);
        buf.extend_from_slice(&addr.ip().octets()); // 4 octets
        buf.extend_from_slice(&addr.port().to_le_bytes()); // 2 octets
        buf.extend_from_slice(pk); // 32 octets : la clé publique du pair
    }
    buf
}

/// Lit un paquet WELCOME : renvoie (ton_id, teinte_du_monde, liste des autres
/// avec leur clé publique).
pub(crate) fn decode_welcome(buf: &[u8]) -> Option<(u8, u16, Vec<RosterEntry>)> {
    if buf.len() < 6 || buf[0] != KIND_WELCOME || buf[1] != PROTO_VERSION {
        return None;
    }
    let your_id = buf[2];
    let world_hue = u16::from_le_bytes([buf[3], buf[4]]);
    let count = buf[5] as usize;
    let mut roster = Vec::with_capacity(count);
    let mut o = 6; // on saute type + version + your_id + teinte (2o) + count
    for _ in 0..count {
        if o + ROSTER_ENTRY_SIZE > buf.len() {
            break; // paquet tronqué : on s'arrête là, sans planter
        }
        let id = buf[o];
        let ip = Ipv4Addr::new(buf[o + 1], buf[o + 2], buf[o + 3], buf[o + 4]);
        let port = u16::from_le_bytes([buf[o + 5], buf[o + 6]]);
        let mut pk = [0u8; PUBKEY_LEN];
        pk.copy_from_slice(&buf[o + 7..o + 7 + PUBKEY_LEN]);
        roster.push((id, SocketAddr::from(SocketAddrV4::new(ip, port)), pk));
        o += ROSTER_ENTRY_SIZE;
    }
    Some((your_id, world_hue, roster))
}
