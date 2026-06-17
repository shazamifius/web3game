//! LE MESSAGE : comment l'état d'un joueur devient une suite d'octets, et inversement.
//!
//! C'est le « protocole » : l'émetteur et le récepteur doivent s'accorder sur
//! l'ordre exact des octets. On le fait à la main pour tout comprendre.
//!
//! Le 1er octet est le TYPE de paquet (`KIND_STATE`) : il distingue un état de
//! joueur des messages d'annuaire (voir `wire`/`control`).

use super::wire::{KIND_RELAY, KIND_STATE};

/// L'état d'un joueur transmis sur le réseau : qui (`id`), où (`x,y,z`), à quelle
/// vitesse il va (`vx,vy,vz`), comment il est orienté (`yaw,pitch`) et de quelle
/// couleur est son skin (`r,g,b`).
#[derive(Clone, Copy, Debug)]
pub struct PlayerState {
    pub id: u8,     // identifiant du joueur (1 octet : jusqu'à 255 joueurs)
    pub x: f32,     // position gauche/droite
    pub y: f32,     // position haut/bas (hauteur)
    pub z: f32,     // position avant/arrière
    pub vx: f32,    // vitesse RÉELLE de l'émetteur : gauche/droite (m/s)
    pub vy: f32,    // vitesse réelle : haut/bas (m/s)
    pub vz: f32,    // vitesse réelle : avant/arrière (m/s)
    pub yaw: f32,   // orientation du corps gauche/droite (radians)
    pub pitch: f32, // inclinaison de la tête haut/bas (radians)
    pub r: f32,     // couleur du skin : rouge
    pub g: f32,     // couleur du skin : vert
    pub b: f32,     // couleur du skin : bleu
}

// Taille exacte d'un paquet d'état, calculée à la main pour bien comprendre :
//   1 octet (type) + 1 octet (id) + 11 nombres f32 de 4 octets
//   (x,y,z, vx,vy,vz, yaw,pitch, r,g,b) = 2 + 44 = 46 octets.
const STATE_SIZE: usize = 2 + 4 * 11;

/// « Sérialiser » : transformer la fiche `PlayerState` en octets bruts à envoyer.
/// `to_le_bytes` découpe chaque nombre en 4 octets (sens « little-endian »).
/// L'émetteur et le récepteur doivent juste utiliser le même sens — on choisit LE.
pub(crate) fn encode(p: &PlayerState) -> [u8; STATE_SIZE] {
    let mut buf = [0u8; STATE_SIZE];
    buf[0] = KIND_STATE; // type de paquet
    buf[1] = p.id;
    buf[2..6].copy_from_slice(&p.x.to_le_bytes());
    buf[6..10].copy_from_slice(&p.y.to_le_bytes());
    buf[10..14].copy_from_slice(&p.z.to_le_bytes());
    buf[14..18].copy_from_slice(&p.vx.to_le_bytes());
    buf[18..22].copy_from_slice(&p.vy.to_le_bytes());
    buf[22..26].copy_from_slice(&p.vz.to_le_bytes());
    buf[26..30].copy_from_slice(&p.yaw.to_le_bytes());
    buf[30..34].copy_from_slice(&p.pitch.to_le_bytes());
    buf[34..38].copy_from_slice(&p.r.to_le_bytes());
    buf[38..42].copy_from_slice(&p.g.to_le_bytes());
    buf[42..46].copy_from_slice(&p.b.to_le_bytes());
    buf
}

/// Variante RELAYÉE : exactement le même état, mais avec l'octet de tête
/// `KIND_RELAY`. Un joueur à faible upload l'envoie à son parent pour dire
/// « recopie ça à mes voisins » (cf. `net/netcode`). Le parent le ré-émet ensuite
/// en `KIND_STATE` ordinaire — l'`id` reste celui de l'auteur, pas du relayeur.
pub(crate) fn encode_relay(p: &PlayerState) -> [u8; STATE_SIZE] {
    let mut buf = encode(p);
    buf[0] = KIND_RELAY; // seul l'octet de type change ; tout le reste est identique
    buf
}

/// Décode un paquet RELAY (même corps qu'un état, type `KIND_RELAY`).
pub(crate) fn decode_relay(buf: &[u8]) -> Option<PlayerState> {
    if buf.len() < STATE_SIZE || buf[0] != KIND_RELAY {
        return None;
    }
    // Le corps est identique à un état : on rebascule l'octet de tête et on réutilise
    // `decode` pour ne pas dupliquer la lecture des 11 nombres.
    let mut tmp = buf[..STATE_SIZE].to_vec();
    tmp[0] = KIND_STATE;
    decode(&tmp)
}

/// L'inverse : reconstruire un `PlayerState` à partir des octets reçus.
/// Renvoie `None` si le paquet est trop court ou n'est pas un état — on ne fait
/// jamais confiance aveuglément à ce qui vient du réseau.
pub(crate) fn decode(buf: &[u8]) -> Option<PlayerState> {
    if buf.len() < STATE_SIZE || buf[0] != KIND_STATE {
        return None;
    }
    let id = buf[1];
    // `?` = « si la conversion rate, renvoie None tout de suite ».
    let x = f32::from_le_bytes(buf[2..6].try_into().ok()?);
    let y = f32::from_le_bytes(buf[6..10].try_into().ok()?);
    let z = f32::from_le_bytes(buf[10..14].try_into().ok()?);
    let vx = f32::from_le_bytes(buf[14..18].try_into().ok()?);
    let vy = f32::from_le_bytes(buf[18..22].try_into().ok()?);
    let vz = f32::from_le_bytes(buf[22..26].try_into().ok()?);
    let yaw = f32::from_le_bytes(buf[26..30].try_into().ok()?);
    let pitch = f32::from_le_bytes(buf[30..34].try_into().ok()?);
    let r = f32::from_le_bytes(buf[34..38].try_into().ok()?);
    let g = f32::from_le_bytes(buf[38..42].try_into().ok()?);
    let b = f32::from_le_bytes(buf[42..46].try_into().ok()?);
    Some(PlayerState { id, x, y, z, vx, vy, vz, yaw, pitch, r, g, b })
}
