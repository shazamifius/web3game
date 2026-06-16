//! LE MESSAGE : comment l'état d'un joueur devient une suite d'octets, et inversement.
//!
//! C'est le « protocole » : l'émetteur et le récepteur doivent s'accorder sur
//! l'ordre exact des octets. On le fait à la main pour tout comprendre.

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

// Taille exacte d'un paquet, calculée à la main pour bien comprendre :
//   1 octet (id) + 11 nombres f32 de 4 octets (x,y,z, vx,vy,vz, yaw,pitch, r,g,b)
//   = 1 + 44 = 45 octets.
const PACKET_SIZE: usize = 1 + 4 * 11;

/// « Sérialiser » : transformer la fiche `PlayerState` en octets bruts à envoyer.
/// `to_le_bytes` découpe chaque nombre en 4 octets (sens « little-endian »).
/// L'émetteur et le récepteur doivent juste utiliser le même sens — on choisit LE.
pub(crate) fn encode(p: &PlayerState) -> [u8; PACKET_SIZE] {
    let mut buf = [0u8; PACKET_SIZE];
    buf[0] = p.id;
    buf[1..5].copy_from_slice(&p.x.to_le_bytes());
    buf[5..9].copy_from_slice(&p.y.to_le_bytes());
    buf[9..13].copy_from_slice(&p.z.to_le_bytes());
    buf[13..17].copy_from_slice(&p.vx.to_le_bytes());
    buf[17..21].copy_from_slice(&p.vy.to_le_bytes());
    buf[21..25].copy_from_slice(&p.vz.to_le_bytes());
    buf[25..29].copy_from_slice(&p.yaw.to_le_bytes());
    buf[29..33].copy_from_slice(&p.pitch.to_le_bytes());
    buf[33..37].copy_from_slice(&p.r.to_le_bytes());
    buf[37..41].copy_from_slice(&p.g.to_le_bytes());
    buf[41..45].copy_from_slice(&p.b.to_le_bytes());
    buf
}

/// L'inverse : reconstruire un `PlayerState` à partir des octets reçus.
/// Renvoie `None` si le paquet est trop court — on ne fait jamais confiance
/// aveuglément à ce qui vient du réseau.
pub(crate) fn decode(buf: &[u8]) -> Option<PlayerState> {
    if buf.len() < PACKET_SIZE {
        return None;
    }
    let id = buf[0];
    // `?` = « si la conversion rate, renvoie None tout de suite ».
    let x = f32::from_le_bytes(buf[1..5].try_into().ok()?);
    let y = f32::from_le_bytes(buf[5..9].try_into().ok()?);
    let z = f32::from_le_bytes(buf[9..13].try_into().ok()?);
    let vx = f32::from_le_bytes(buf[13..17].try_into().ok()?);
    let vy = f32::from_le_bytes(buf[17..21].try_into().ok()?);
    let vz = f32::from_le_bytes(buf[21..25].try_into().ok()?);
    let yaw = f32::from_le_bytes(buf[25..29].try_into().ok()?);
    let pitch = f32::from_le_bytes(buf[29..33].try_into().ok()?);
    let r = f32::from_le_bytes(buf[33..37].try_into().ok()?);
    let g = f32::from_le_bytes(buf[37..41].try_into().ok()?);
    let b = f32::from_le_bytes(buf[41..45].try_into().ok()?);
    Some(PlayerState { id, x, y, z, vx, vy, vz, yaw, pitch, r, g, b })
}
