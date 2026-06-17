//! L'ANNUAIRE : les messages échangés avec le serveur de rendez-vous.
//!
//!   - HELLO   (client → serveur) : « je suis là ». Le serveur lit l'adresse
//!     source du paquet, donc pas besoin de la mettre dedans (1 octet suffit).
//!   - WELCOME (serveur → client) : « ton identifiant, et voici les autres ».

use super::wire::{KIND_HELLO, KIND_WELCOME};
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

/// Fabrique un paquet HELLO : type + la POSITION (x, z) du joueur. Le rendez-vous
/// s'en sert pour ne renvoyer que les joueurs dans le rayon de perception (AoI).
pub(crate) fn encode_hello(x: f32, z: f32) -> [u8; 9] {
    let mut b = [0u8; 9];
    b[0] = KIND_HELLO;
    b[1..5].copy_from_slice(&x.to_le_bytes());
    b[5..9].copy_from_slice(&z.to_le_bytes());
    b
}

/// Lit un paquet HELLO : renvoie la position (x, z) annoncée.
pub(crate) fn decode_hello(buf: &[u8]) -> Option<(f32, f32)> {
    if buf.len() < 9 || buf[0] != KIND_HELLO {
        return None;
    }
    let x = f32::from_le_bytes(buf[1..5].try_into().ok()?);
    let z = f32::from_le_bytes(buf[5..9].try_into().ok()?);
    Some((x, z))
}

/// Fabrique un paquet WELCOME : type + ton_id + teinte_du_monde (2o) + nombre +
/// [ (id, ip, port) … ]. La teinte (0–359) est la couleur de salle PARTAGÉE par
/// tous les joueurs du même serveur. (IPv4 uniquement — tout est sur 127.0.0.1.)
pub(crate) fn encode_welcome(your_id: u8, world_hue: u16, roster: &[(u8, SocketAddr)]) -> Vec<u8> {
    // On ne garde que les adresses IPv4 (les seules qu'on sait encoder).
    let v4: Vec<(u8, SocketAddrV4)> = roster
        .iter()
        .filter_map(|(id, addr)| match addr {
            SocketAddr::V4(a) => Some((*id, *a)),
            SocketAddr::V6(_) => None,
        })
        .collect();

    let mut buf = vec![KIND_WELCOME, your_id];
    buf.extend_from_slice(&world_hue.to_le_bytes()); // teinte du monde (2 octets)
    buf.push(v4.len() as u8);
    for (id, addr) in v4 {
        buf.push(id);
        buf.extend_from_slice(&addr.ip().octets()); // 4 octets
        buf.extend_from_slice(&addr.port().to_le_bytes()); // 2 octets
    }
    buf
}

/// Lit un paquet WELCOME : renvoie (ton_id, teinte_du_monde, liste des autres).
pub(crate) fn decode_welcome(buf: &[u8]) -> Option<(u8, u16, Vec<(u8, SocketAddr)>)> {
    if buf.len() < 5 || buf[0] != KIND_WELCOME {
        return None;
    }
    let your_id = buf[1];
    let world_hue = u16::from_le_bytes([buf[2], buf[3]]);
    let count = buf[4] as usize;
    let mut roster = Vec::with_capacity(count);
    let mut o = 5; // on saute type + your_id + teinte (2o) + count
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
    Some((your_id, world_hue, roster))
}
