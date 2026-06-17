//! LE MESSAGE : comment l'état d'un joueur devient une suite d'octets, et inversement.
//!
//! C'est le « protocole » : l'émetteur et le récepteur doivent s'accorder sur
//! l'ordre exact des octets. On le fait à la main pour tout comprendre.
//!
//! Le 1er octet est le TYPE de paquet (`KIND_STATE`) : il distingue un état de
//! joueur des messages d'annuaire (voir `wire`/`control`).

use super::crypto::{verify, Identity, PUBKEY_LEN, SIG_LEN};
use super::wire::{KIND_RELAY, KIND_STATE, PROTO_VERSION};

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
    pub parent: u8, // rôle : id de notre tuteur (relais) si on est sous tutelle, sinon 0
}

// Taille exacte d'un paquet d'état, calculée à la main pour bien comprendre :
//   1 (type) + 1 (version) + 1 (id) + 11 nombres f32 de 4 octets
//   (x,y,z, vx,vy,vz, yaw,pitch, r,g,b) + 1 (parent) = 3 + 44 + 1 = 48 octets.
const STATE_SIZE: usize = 3 + 4 * 11 + 1;

/// « Sérialiser » : transformer la fiche `PlayerState` en octets bruts à envoyer.
/// `to_le_bytes` découpe chaque nombre en 4 octets (sens « little-endian »).
/// L'émetteur et le récepteur doivent juste utiliser le même sens — on choisit LE.
/// En-tête commun : octet 0 = type, octet 1 = version du protocole.
pub(crate) fn encode(p: &PlayerState) -> [u8; STATE_SIZE] {
    let mut buf = [0u8; STATE_SIZE];
    buf[0] = KIND_STATE; // type de paquet
    buf[1] = PROTO_VERSION; // version du protocole
    buf[2] = p.id;
    buf[3..7].copy_from_slice(&p.x.to_le_bytes());
    buf[7..11].copy_from_slice(&p.y.to_le_bytes());
    buf[11..15].copy_from_slice(&p.z.to_le_bytes());
    buf[15..19].copy_from_slice(&p.vx.to_le_bytes());
    buf[19..23].copy_from_slice(&p.vy.to_le_bytes());
    buf[23..27].copy_from_slice(&p.vz.to_le_bytes());
    buf[27..31].copy_from_slice(&p.yaw.to_le_bytes());
    buf[31..35].copy_from_slice(&p.pitch.to_le_bytes());
    buf[35..39].copy_from_slice(&p.r.to_le_bytes());
    buf[39..43].copy_from_slice(&p.g.to_le_bytes());
    buf[43..47].copy_from_slice(&p.b.to_le_bytes());
    buf[47] = p.parent; // rôle (tuteur) sur le tout dernier octet
    buf
}


/// L'inverse : reconstruire un `PlayerState` à partir des octets reçus.
/// Renvoie `None` si le paquet est trop court ou n'est pas un état — on ne fait
/// jamais confiance aveuglément à ce qui vient du réseau.
pub(crate) fn decode(buf: &[u8]) -> Option<PlayerState> {
    if buf.len() < STATE_SIZE || buf[0] != KIND_STATE || buf[1] != PROTO_VERSION {
        return None;
    }
    let id = buf[2];
    // `?` = « si la conversion rate, renvoie None tout de suite ».
    let x = f32::from_le_bytes(buf[3..7].try_into().ok()?);
    let y = f32::from_le_bytes(buf[7..11].try_into().ok()?);
    let z = f32::from_le_bytes(buf[11..15].try_into().ok()?);
    let vx = f32::from_le_bytes(buf[15..19].try_into().ok()?);
    let vy = f32::from_le_bytes(buf[19..23].try_into().ok()?);
    let vz = f32::from_le_bytes(buf[23..27].try_into().ok()?);
    let yaw = f32::from_le_bytes(buf[27..31].try_into().ok()?);
    let pitch = f32::from_le_bytes(buf[31..35].try_into().ok()?);
    let r = f32::from_le_bytes(buf[35..39].try_into().ok()?);
    let g = f32::from_le_bytes(buf[39..43].try_into().ok()?);
    let b = f32::from_le_bytes(buf[43..47].try_into().ok()?);
    let parent = buf[47];
    // ON NE FAIT JAMAIS CONFIANCE AU RÉSEAU, suite : on rejette tout NaN/Inf. Un
    // seul flottant non fini corromprait DÉFINITIVEMENT le lissage (`smooth_damp`
    // garde un état interne qui reste NaN). Mieux vaut jeter le paquet entier.
    if ![x, y, z, vx, vy, vz, yaw, pitch, r, g, b]
        .iter()
        .all(|f| f.is_finite())
    {
        return None;
    }
    Some(PlayerState { id, x, y, z, vx, vy, vz, yaw, pitch, r, g, b, parent })
}

// ----------------------------------------------------------------------------
// ENVELOPPE SIGNÉE (chapitre 5.1) : corps + sceau cryptographique.
// ----------------------------------------------------------------------------
/// Taille d'un état SIGNÉ : le corps (48 o) suivi de la signature (64 o) = 112.
pub(crate) const SIGNED_STATE_SIZE: usize = STATE_SIZE + SIG_LEN;

/// Scelle un état : on encode le corps (forme canonique `KIND_STATE`), on le SIGNE
/// avec notre clé privée, et on colle la signature derrière. Le récepteur pourra
/// prouver que CES octets viennent bien de nous et n'ont pas bougé.
///
/// Astuce du relais : la signature couvre TOUJOURS le corps en forme `KIND_STATE`.
/// Un client à faible upload n'a qu'à basculer le 1er octet en `KIND_RELAY` pour le
/// transport (cf. `mark_as_relay`) ; le parent le rebascule en `KIND_STATE` avant
/// de recopier → les octets signés sont identiques, le sceau tient. Le parent porte
/// l'enveloppe mais ne peut pas en changer le contenu.
pub(crate) fn encode_signed(p: &PlayerState, identity: &Identity) -> [u8; SIGNED_STATE_SIZE] {
    let body = encode(p); // 48 octets, 1er octet = KIND_STATE
    let sig = identity.sign(&body); // 64 octets : le sceau
    let mut out = [0u8; SIGNED_STATE_SIZE];
    out[..STATE_SIZE].copy_from_slice(&body);
    out[STATE_SIZE..].copy_from_slice(&sig);
    out
}

/// Bascule un paquet signé en variante RELAY (juste le 1er octet). Le corps SIGNÉ
/// ne change pas : on ne touche pas aux octets couverts par le sceau (on a signé la
/// forme `KIND_STATE`, on ne fait que marquer le transport).
pub(crate) fn mark_as_relay(signed: &mut [u8; SIGNED_STATE_SIZE]) {
    signed[0] = KIND_RELAY;
}

/// Vérifie le sceau d'un état signé avec la clé publique de l'émetteur, PUIS le
/// décode. Renvoie `None` au moindre problème : trop court, signature qui ne colle
/// pas (forgé ou altéré), ou corps invalide. Accepte les deux types en tête
/// (`KIND_STATE` direct ou `KIND_RELAY` recopié) : on ramène à la forme canonique
/// `KIND_STATE` — celle qui a été signée — avant de vérifier.
pub(crate) fn decode_verified(buf: &[u8], pubkey: &[u8; PUBKEY_LEN]) -> Option<PlayerState> {
    if buf.len() < SIGNED_STATE_SIZE {
        return None;
    }
    // Corps canonique : copie des 48 octets, 1er octet forcé à KIND_STATE (la forme
    // qui a été signée, que le paquet arrive en direct ou via un relais).
    let mut body = [0u8; STATE_SIZE];
    body.copy_from_slice(&buf[..STATE_SIZE]);
    body[0] = KIND_STATE;

    let mut sig = [0u8; SIG_LEN];
    sig.copy_from_slice(&buf[STATE_SIZE..SIGNED_STATE_SIZE]);

    if !verify(&body, &sig, pubkey) {
        return None; // sceau invalide : forgé, altéré, ou mauvaise identité
    }
    decode(&body) // re-valide type + version + finitude
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Un état qui fait l'aller-retour encode→decode doit revenir identique :
    /// c'est la garantie que les offsets d'octets sont cohérents des deux côtés.
    #[test]
    fn etat_survit_a_l_aller_retour() {
        let p = PlayerState {
            id: 7,
            x: 1.5, y: 0.7, z: -3.25,
            vx: -2.0, vy: 0.1, vz: 4.0,
            yaw: 1.2, pitch: -0.3,
            r: 0.8, g: 0.2, b: 1.1,
            parent: 3,
        };
        let d = decode(&encode(&p)).expect("doit se décoder");
        assert_eq!(d.id, p.id);
        assert_eq!(d.parent, p.parent);
        assert_eq!(d.x, p.x);
        assert_eq!(d.z, p.z);
        assert_eq!(d.vz, p.vz);
        assert_eq!(d.b, p.b);
    }

    /// Un paquet d'une AUTRE version est rejeté (None), pas lu de travers.
    #[test]
    fn version_differente_est_rejetee() {
        let p = PlayerState {
            id: 1, x: 0.0, y: 0.0, z: 0.0, vx: 0.0, vy: 0.0, vz: 0.0,
            yaw: 0.0, pitch: 0.0, r: 0.0, g: 0.0, b: 0.0, parent: 0,
        };
        let mut bytes = encode(&p);
        bytes[1] = PROTO_VERSION.wrapping_add(1); // on falsifie la version
        assert!(decode(&bytes).is_none());
    }

    /// Un paquet porteur d'un NaN/Inf est rejeté (jamais laissé entrer).
    #[test]
    fn nan_est_rejete() {
        let mut p = PlayerState {
            id: 1, x: 0.0, y: 0.0, z: 0.0, vx: 0.0, vy: 0.0, vz: 0.0,
            yaw: 0.0, pitch: 0.0, r: 0.0, g: 0.0, b: 0.0, parent: 0,
        };
        p.x = f32::NAN;
        assert!(decode(&encode(&p)).is_none());
        p.x = 0.0;
        p.vz = f32::INFINITY;
        assert!(decode(&encode(&p)).is_none());
    }

    fn etat_exemple() -> PlayerState {
        PlayerState {
            id: 5, x: 1.0, y: 0.7, z: -2.0, vx: 0.5, vy: 0.0, vz: -1.0,
            yaw: 0.3, pitch: 0.1, r: 0.9, g: 0.4, b: 0.2, parent: 0,
        }
    }

    /// Un état SIGNÉ se vérifie avec la bonne clé publique puis se décode.
    #[test]
    fn etat_signe_se_verifie_et_se_decode() {
        let id = Identity::generate();
        let p = etat_exemple();
        let signed = encode_signed(&p, &id);
        let d = decode_verified(&signed, &id.public()).expect("sceau valide");
        assert_eq!(d.id, 5);
        assert_eq!(d.z, -2.0);
    }

    /// Le moindre octet du CORPS modifié casse le sceau → rejet (anti-falsification).
    #[test]
    fn etat_signe_altere_est_rejete() {
        let id = Identity::generate();
        let mut signed = encode_signed(&etat_exemple(), &id);
        signed[3] ^= 0xFF; // on triture un octet de la position
        assert!(decode_verified(&signed, &id.public()).is_none());
    }

    /// Vérifié contre la clé publique de QUELQU'UN D'AUTRE → rejet (anti-usurpation).
    #[test]
    fn etat_signe_mauvaise_cle_est_rejete() {
        let moi = Identity::generate();
        let autre = Identity::generate();
        let signed = encode_signed(&etat_exemple(), &moi);
        assert!(decode_verified(&signed, &autre.public()).is_none());
    }

    /// L'enveloppe scellée résiste au RELAIS : basculer en KIND_RELAY (ce que fait
    /// le client faible) puis rebascule implicite par decode_verified ne casse pas
    /// le sceau — mais changer le contenu, si. C'est la garantie « parent porteur,
    /// pas faussaire ».
    #[test]
    fn enveloppe_scellee_survit_au_relais_mais_pas_a_la_falsification() {
        let id = Identity::generate();
        let mut signed = encode_signed(&etat_exemple(), &id);
        mark_as_relay(&mut signed); // le client faible marque le transport
        // Le parent (et les voisins) vérifient : le sceau tient malgré KIND_RELAY.
        assert!(decode_verified(&signed, &id.public()).is_some());
        // Mais si un parent malveillant change la position, le sceau casse.
        signed[5] ^= 0x7F;
        assert!(decode_verified(&signed, &id.public()).is_none());
    }
}
