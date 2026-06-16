//! L'ANNUAIRE : les messages échangés avec le serveur de rendez-vous.
//!
//!   - HELLO   (client → serveur) : « je suis là ». Le serveur lit l'adresse
//!     source du paquet, donc pas besoin de la mettre dedans (1 octet suffit).
//!   - WELCOME (serveur → client) : « ton identifiant, et voici les autres ».

use super::wire::{KIND_HELLO, KIND_WELCOME};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

/// Fabrique un paquet HELLO (juste le type : 1 octet).
pub(crate) fn encode_hello() -> [u8; 1] {
    [KIND_HELLO]
}

/// Fabrique un paquet WELCOME : type + ton_id + nombre + [ (id, ip, port) … ].
/// (IPv4 uniquement pour l'instant — tout est sur 127.0.0.1.)
pub(crate) fn encode_welcome(your_id: u8, roster: &[(u8, SocketAddr)]) -> Vec<u8> {
    // On ne garde que les adresses IPv4 (les seules qu'on sait encoder).
    let v4: Vec<(u8, SocketAddrV4)> = roster
        .iter()
        .filter_map(|(id, addr)| match addr {
            SocketAddr::V4(a) => Some((*id, *a)),
            SocketAddr::V6(_) => None,
        })
        .collect();

    let mut buf = vec![KIND_WELCOME, your_id, v4.len() as u8];
    for (id, addr) in v4 {
        buf.push(id);
        buf.extend_from_slice(&addr.ip().octets()); // 4 octets
        buf.extend_from_slice(&addr.port().to_le_bytes()); // 2 octets
    }
    buf
}

/// Lit un paquet WELCOME : renvoie (ton_id, liste des autres joueurs).
pub(crate) fn decode_welcome(buf: &[u8]) -> Option<(u8, Vec<(u8, SocketAddr)>)> {
    if buf.len() < 3 || buf[0] != KIND_WELCOME {
        return None;
    }
    let your_id = buf[1];
    let count = buf[2] as usize;
    let mut roster = Vec::with_capacity(count);
    let mut o = 3; // on saute type + your_id + count
    for _ in 0..count {
        if o + 7 > buf.len() {
            break; // paquet tronqué : on s'arrête là, sans planter
        }
        let id = buf[o];
        let ip = Ipv4Addr::new(buf[o + 1], buf[o + 2], buf[o + 3], buf[o + 4]);
        let port = u16::from_le_bytes([buf[o + 5], buf[o + 6]]);
        roster.push((id, SocketAddr::from(SocketAddrV4::new(ip, port))));
        o += 7;
    }
    Some((your_id, roster))
}
