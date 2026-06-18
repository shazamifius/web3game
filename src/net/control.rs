//! L'ANNUAIRE : les messages échangés avec le serveur de rendez-vous.
//!
//!   - HELLO   (client → serveur) : « je suis là ». Le serveur lit l'adresse
//!     source du paquet, donc pas besoin de la mettre dedans.
//!   - WELCOME (serveur → client) : « voici les autres ». Depuis le chapitre 6.1,
//!     le serveur ne nous attribue PLUS de numéro : notre identité est notre clé
//!     publique, qu'on connaît déjà. Le serveur n'est qu'un carnet d'adresses.

use super::crypto::{PeerId, PUBKEY_LEN};
use super::wire::{KIND_HELLO, KIND_WELCOME, PROTO_VERSION};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

/// Taille d'un HELLO : type + version + x(4) + z(4) + clé publique (32) = 42.
const HELLO_SIZE: usize = 2 + 4 + 4 + PUBKEY_LEN;

/// Fabrique un paquet HELLO : type + version + POSITION (x, z) + notre IDENTITÉ
/// (clé publique). La position sert à l'AoI ; l'identité permet au rendez-vous de
/// nous lister auprès des autres (et à eux de nous joindre).
pub(crate) fn encode_hello(x: f32, z: f32, id: PeerId) -> [u8; HELLO_SIZE] {
    let mut b = [0u8; HELLO_SIZE];
    b[0] = KIND_HELLO;
    b[1] = PROTO_VERSION;
    b[2..6].copy_from_slice(&x.to_le_bytes());
    b[6..10].copy_from_slice(&z.to_le_bytes());
    b[10..10 + PUBKEY_LEN].copy_from_slice(id.bytes());
    b
}

/// Lit un paquet HELLO : renvoie la position (x, z) et l'identité (clé) annoncées.
pub(crate) fn decode_hello(buf: &[u8]) -> Option<(f32, f32, PeerId)> {
    if buf.len() < HELLO_SIZE || buf[0] != KIND_HELLO || buf[1] != PROTO_VERSION {
        return None;
    }
    let x = f32::from_le_bytes(buf[2..6].try_into().ok()?);
    let z = f32::from_le_bytes(buf[6..10].try_into().ok()?);
    let mut pk = [0u8; PUBKEY_LEN];
    pk.copy_from_slice(&buf[10..10 + PUBKEY_LEN]);
    Some((x, z, PeerId::from_bytes(pk)))
}

/// Une entrée d'annuaire : id (clé, 32) + ip (4) + port (2) = 38 octets.
const ROSTER_ENTRY_SIZE: usize = PUBKEY_LEN + 4 + 2;

/// Type d'une fiche de pair dans le roster : son identité (clé) et son adresse.
pub(crate) type RosterEntry = (PeerId, SocketAddr);

/// Fabrique un paquet WELCOME : type + version + teinte_du_monde (2o) + nombre +
/// [ (clé, ip, port) … ]. La teinte (0–359) est la couleur de salle PARTAGÉE par
/// tous les joueurs. (IPv4 uniquement — tout est sur 127.0.0.1 pour l'instant.)
pub(crate) fn encode_welcome(world_hue: u16, roster: &[RosterEntry]) -> Vec<u8> {
    let v4: Vec<(PeerId, SocketAddrV4)> = roster
        .iter()
        .filter_map(|(id, addr)| match addr {
            SocketAddr::V4(a) => Some((*id, *a)),
            SocketAddr::V6(_) => None,
        })
        .collect();

    let mut buf = vec![KIND_WELCOME, PROTO_VERSION];
    buf.extend_from_slice(&world_hue.to_le_bytes()); // teinte du monde (2 octets)
    buf.push(v4.len() as u8);
    for (id, addr) in v4 {
        buf.extend_from_slice(id.bytes()); // 32 octets : l'identité (clé) du pair
        buf.extend_from_slice(&addr.ip().octets()); // 4 octets
        buf.extend_from_slice(&addr.port().to_le_bytes()); // 2 octets
    }
    buf
}

/// Lit un paquet WELCOME : renvoie (teinte_du_monde, liste des autres avec leur clé).
pub(crate) fn decode_welcome(buf: &[u8]) -> Option<(u16, Vec<RosterEntry>)> {
    if buf.len() < 5 || buf[0] != KIND_WELCOME || buf[1] != PROTO_VERSION {
        return None;
    }
    let world_hue = u16::from_le_bytes([buf[2], buf[3]]);
    let count = buf[4] as usize;
    let mut roster = Vec::with_capacity(count);
    let mut o = 5; // type + version + teinte (2o) + count
    for _ in 0..count {
        if o + ROSTER_ENTRY_SIZE > buf.len() {
            break; // paquet tronqué : on s'arrête là, sans planter
        }
        let mut pk = [0u8; PUBKEY_LEN];
        pk.copy_from_slice(&buf[o..o + PUBKEY_LEN]);
        let ip = Ipv4Addr::new(
            buf[o + PUBKEY_LEN],
            buf[o + PUBKEY_LEN + 1],
            buf[o + PUBKEY_LEN + 2],
            buf[o + PUBKEY_LEN + 3],
        );
        let port = u16::from_le_bytes([buf[o + PUBKEY_LEN + 4], buf[o + PUBKEY_LEN + 5]]);
        roster.push((PeerId::from_bytes(pk), SocketAddr::from(SocketAddrV4::new(ip, port))));
        o += ROSTER_ENTRY_SIZE;
    }
    Some((world_hue, roster))
}
