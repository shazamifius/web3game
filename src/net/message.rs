//! LE MESSAGE : comment l'état d'un joueur devient une suite d'octets, et inversement.
//!
//! C'est le « protocole » : l'émetteur et le récepteur doivent s'accorder sur
//! l'ordre exact des octets. On le fait à la main pour tout comprendre.
//!
//! Le 1er octet est le TYPE de paquet (`KIND_STATE`) ; le 2e la version du protocole.
//!
//! # Identité auto-certifiante (chapitre 6.1)
//! Le champ `id` n'est plus un numéro `u8` attribué par le rendez-vous : c'est
//! désormais la **clé publique** de l'émetteur (32 octets), PORTÉE dans le paquet.
//! Le récepteur vérifie le sceau AVEC cette clé embarquée → l'identité s'auto-prouve,
//! sans annuaire de confiance. Personne (pas même le rendez-vous) ne peut usurper
//! une identité : il faudrait posséder la clé privée correspondante.

use super::crypto::{verify, Identity, PeerId, PUBKEY_LEN, SIG_LEN};
use super::wire::{KIND_RELAY, KIND_RELAY_FWD, KIND_STATE, PROTO_VERSION};

/// L'état d'un joueur transmis sur le réseau : qui (`id` = sa clé publique), où
/// (`x,y,z`), à quelle vitesse il va (`vx,vy,vz`), comment il est orienté
/// (`yaw,pitch`), de quelle couleur est son skin (`r,g,b`), son éventuel tuteur
/// (`parent`) et son compteur anti-rejeu (`seq`).
#[derive(Clone, Copy, Debug)]
pub struct PlayerState {
    pub id: PeerId,             // identité = clé publique de l'émetteur (auto-certifiante)
    pub x: f32,                 // position gauche/droite
    pub y: f32,                 // position haut/bas (hauteur)
    pub z: f32,                 // position avant/arrière
    pub vx: f32,                // vitesse RÉELLE : gauche/droite (m/s)
    pub vy: f32,                // vitesse réelle : haut/bas (m/s)
    pub vz: f32,                // vitesse réelle : avant/arrière (m/s)
    pub yaw: f32,               // orientation du corps gauche/droite (radians)
    pub pitch: f32,             // inclinaison de la tête haut/bas (radians)
    pub r: f32,                 // couleur du skin : rouge
    pub g: f32,                 // couleur du skin : vert
    pub b: f32,                 // couleur du skin : bleu
    pub parent: Option<PeerId>, // tuteur (relais) si on est sous tutelle, sinon None
    pub seq: u64,               // compteur monotone ANTI-REJEU (chap. 5.2)
}

// Décalage des champs dans le paquet, calculés à la main pour bien comprendre.
//   [0] type | [1] version | [2..34] id (clé, 32 o) | [34..78] 11 f32 (44 o)
//   | [78..110] parent (clé ou zéros, 32 o) | [110..118] seq (u64, 8 o).
const OFF_ID: usize = 2;
const OFF_FLOATS: usize = OFF_ID + PUBKEY_LEN; // 34
const OFF_PARENT: usize = OFF_FLOATS + 4 * 11; // 78
const OFF_SEQ: usize = OFF_PARENT + PUBKEY_LEN; // 110
const STATE_SIZE: usize = OFF_SEQ + 8; // 118

/// « Sérialiser » : transformer la fiche `PlayerState` en octets bruts à envoyer.
/// `to_le_bytes` découpe chaque nombre en 4 octets (sens « little-endian »).
/// En-tête commun : octet 0 = type, octet 1 = version du protocole.
pub(crate) fn encode(p: &PlayerState) -> [u8; STATE_SIZE] {
    let mut buf = [0u8; STATE_SIZE];
    buf[0] = KIND_STATE;
    buf[1] = PROTO_VERSION;
    buf[OFF_ID..OFF_ID + PUBKEY_LEN].copy_from_slice(p.id.bytes());
    let f = OFF_FLOATS;
    buf[f..f + 4].copy_from_slice(&p.x.to_le_bytes());
    buf[f + 4..f + 8].copy_from_slice(&p.y.to_le_bytes());
    buf[f + 8..f + 12].copy_from_slice(&p.z.to_le_bytes());
    buf[f + 12..f + 16].copy_from_slice(&p.vx.to_le_bytes());
    buf[f + 16..f + 20].copy_from_slice(&p.vy.to_le_bytes());
    buf[f + 20..f + 24].copy_from_slice(&p.vz.to_le_bytes());
    buf[f + 24..f + 28].copy_from_slice(&p.yaw.to_le_bytes());
    buf[f + 28..f + 32].copy_from_slice(&p.pitch.to_le_bytes());
    buf[f + 32..f + 36].copy_from_slice(&p.r.to_le_bytes());
    buf[f + 36..f + 40].copy_from_slice(&p.g.to_le_bytes());
    buf[f + 40..f + 44].copy_from_slice(&p.b.to_le_bytes());
    // parent : la clé du tuteur, ou 32 zéros si autonome.
    let parent = p.parent.unwrap_or_else(PeerId::none);
    buf[OFF_PARENT..OFF_PARENT + PUBKEY_LEN].copy_from_slice(parent.bytes());
    buf[OFF_SEQ..OFF_SEQ + 8].copy_from_slice(&p.seq.to_le_bytes());
    buf
}

/// L'inverse : reconstruire un `PlayerState` à partir des octets reçus.
/// Renvoie `None` si le paquet est trop court, n'est pas un état, ou porte un
/// NaN/Inf — on ne fait jamais confiance aveuglément à ce qui vient du réseau.
pub(crate) fn decode(buf: &[u8]) -> Option<PlayerState> {
    if buf.len() < STATE_SIZE || buf[0] != KIND_STATE || buf[1] != PROTO_VERSION {
        return None;
    }
    let mut idb = [0u8; PUBKEY_LEN];
    idb.copy_from_slice(&buf[OFF_ID..OFF_ID + PUBKEY_LEN]);
    let id = PeerId::from_bytes(idb);

    let f = OFF_FLOATS;
    let x = f32::from_le_bytes(buf[f..f + 4].try_into().ok()?);
    let y = f32::from_le_bytes(buf[f + 4..f + 8].try_into().ok()?);
    let z = f32::from_le_bytes(buf[f + 8..f + 12].try_into().ok()?);
    let vx = f32::from_le_bytes(buf[f + 12..f + 16].try_into().ok()?);
    let vy = f32::from_le_bytes(buf[f + 16..f + 20].try_into().ok()?);
    let vz = f32::from_le_bytes(buf[f + 20..f + 24].try_into().ok()?);
    let yaw = f32::from_le_bytes(buf[f + 24..f + 28].try_into().ok()?);
    let pitch = f32::from_le_bytes(buf[f + 28..f + 32].try_into().ok()?);
    let r = f32::from_le_bytes(buf[f + 32..f + 36].try_into().ok()?);
    let g = f32::from_le_bytes(buf[f + 36..f + 40].try_into().ok()?);
    let b = f32::from_le_bytes(buf[f + 40..f + 44].try_into().ok()?);

    let mut pb = [0u8; PUBKEY_LEN];
    pb.copy_from_slice(&buf[OFF_PARENT..OFF_PARENT + PUBKEY_LEN]);
    let parent_id = PeerId::from_bytes(pb);
    let parent = if parent_id.is_none() { None } else { Some(parent_id) };

    let seq = u64::from_le_bytes(buf[OFF_SEQ..OFF_SEQ + 8].try_into().ok()?);

    // Rejet de tout NaN/Inf : un seul flottant non fini corromprait DÉFINITIVEMENT
    // le lissage (`smooth_damp` garde un état interne qui reste NaN). On jette tout.
    if ![x, y, z, vx, vy, vz, yaw, pitch, r, g, b]
        .iter()
        .all(|f| f.is_finite())
    {
        return None;
    }
    Some(PlayerState { id, x, y, z, vx, vy, vz, yaw, pitch, r, g, b, parent, seq })
}

// ----------------------------------------------------------------------------
// ENVELOPPE SIGNÉE (chap. 5.1) + identité auto-certifiante (chap. 6.1).
// ----------------------------------------------------------------------------
/// Taille d'un état SIGNÉ : le corps (118 o) suivi de la signature (64 o) = 182.
pub(crate) const SIGNED_STATE_SIZE: usize = STATE_SIZE + SIG_LEN;

/// Scelle un état : on encode le corps (forme canonique `KIND_STATE`), on le SIGNE
/// avec notre clé privée, et on colle la signature derrière. Le corps EMBARQUE
/// déjà notre clé publique (le champ `id`) : c'est elle qui servira à vérifier.
/// IMPORTANT : `p.id` doit être `identity.id()`, sinon le sceau ne collera pas à
/// la clé embarquée et le paquet sera rejeté (c'est exactement ce qui rend
/// l'usurpation impossible — on ne peut signer que pour SA propre clé).
pub(crate) fn encode_signed(p: &PlayerState, identity: &Identity) -> [u8; SIGNED_STATE_SIZE] {
    let body = encode(p);
    let sig = identity.sign(&body);
    let mut out = [0u8; SIGNED_STATE_SIZE];
    out[..STATE_SIZE].copy_from_slice(&body);
    out[STATE_SIZE..].copy_from_slice(&sig);
    out
}

/// Bascule un paquet signé en variante RELAY (juste le 1er octet). Le corps SIGNÉ
/// ne change pas : on a signé la forme `KIND_STATE`, on ne fait que marquer le transport.
pub(crate) fn mark_as_relay(signed: &mut [u8; SIGNED_STATE_SIZE]) {
    signed[0] = KIND_RELAY;
}

/// Le sceau d'un paquet signé est-il valide ? On lit la clé publique DIRECTEMENT
/// dans le paquet (champ `id`, octets 2..34) et on vérifie le sceau contre ELLE.
/// C'est tout l'intérêt de l'auto-certification : aucun annuaire, aucun tiers de
/// confiance. On ramène d'abord le 1er octet à la forme `KIND_STATE` (celle qui a
/// été signée, que le paquet arrive en direct ou via un relais).
pub(crate) fn sig_ok(buf: &[u8]) -> bool {
    if buf.len() < SIGNED_STATE_SIZE {
        return false;
    }
    let mut pubkey = [0u8; PUBKEY_LEN];
    pubkey.copy_from_slice(&buf[OFF_ID..OFF_ID + PUBKEY_LEN]);
    let mut body = [0u8; STATE_SIZE];
    body.copy_from_slice(&buf[..STATE_SIZE]);
    body[0] = KIND_STATE;
    let mut sig = [0u8; SIG_LEN];
    sig.copy_from_slice(&buf[STATE_SIZE..SIGNED_STATE_SIZE]);
    verify(&body, &sig, &pubkey)
}

// ----------------------------------------------------------------------------
// ENVELOPPE DE RELAIS NAT (chap. 12.3 / D17) — UNICAST A → rendez-vous → B.
// ----------------------------------------------------------------------------
// Quand A ne peut PAS percer le NAT de B (perçage abandonné), il demande au seul point public
// commun (le rendez-vous, v1) de router son état SCELLÉ vers B. C'est une simple enveloppe de
// ROUTAGE autour de l'état déjà signé — on NE re-signe RIEN (le sceau bout-en-bout tient).
//   [0] KIND_RELAY_FWD | [1] version | [2..34] dest_id (clé de B, ROUTAGE seul, NON signé)
//   | [34..216] l'état KIND_STATE SCELLÉ de A, VERBATIM (182 o = SIGNED_STATE_SIZE).
// Le destinataire n'est PAS signé : au pire, un dest falsifié fait router l'état (toujours scellé,
// infalsifiable) vers le mauvais pair, qui vérifie le sceau et affiche A — aucune forge possible.
/// Taille d'une enveloppe de relais NAT = en-tête (2) + dest (32) + état scellé (182) = 216 o.
pub(crate) const RELAY_FWD_SIZE: usize = 2 + PUBKEY_LEN + SIGNED_STATE_SIZE;

/// Décode une enveloppe `KIND_RELAY_FWD` → `(dest, état_scellé_verbatim)`, ou `None` si malformée.
/// L'état interne est rendu VERBATIM (déjà en forme `KIND_STATE`, déjà signé) : l'appelant (le
/// rendez-vous) vérifie son sceau avec `sig_ok` AVANT de le recopier, et ne re-signe jamais.
pub(crate) fn decode_relay_fwd(buf: &[u8]) -> Option<(PeerId, [u8; SIGNED_STATE_SIZE])> {
    if buf.len() != RELAY_FWD_SIZE || buf[0] != KIND_RELAY_FWD || buf[1] != PROTO_VERSION {
        return None;
    }
    let mut db = [0u8; PUBKEY_LEN];
    db.copy_from_slice(&buf[2..2 + PUBKEY_LEN]);
    let dest = PeerId::from_bytes(db);
    let mut inner = [0u8; SIGNED_STATE_SIZE];
    inner.copy_from_slice(&buf[2 + PUBKEY_LEN..]);
    Some((dest, inner))
}

/// Construit une enveloppe `KIND_RELAY_FWD` autour d'un état déjà scellé, à destination de `dest`.
/// Émise par le client au perçage ABANDONNÉ (derrière le drapeau `RELAY_FALLBACK`) vers le rendez-vous,
/// qui la route vers `dest`. Réciproque exacte de `decode_relay_fwd`.
pub(crate) fn encode_relay_fwd(dest: PeerId, sealed: &[u8; SIGNED_STATE_SIZE]) -> [u8; RELAY_FWD_SIZE] {
    let mut out = [0u8; RELAY_FWD_SIZE];
    out[0] = KIND_RELAY_FWD;
    out[1] = PROTO_VERSION;
    out[2..2 + PUBKEY_LEN].copy_from_slice(dest.bytes());
    out[2 + PUBKEY_LEN..].copy_from_slice(sealed);
    out
}

/// Lit l'id (clé) revendiqué dans un paquet d'état, sans rien vérifier. Sert à la
/// réputation : quand le sceau est valide mais le contenu impossible, on sait QUI accuser.
pub(crate) fn claimed_id(buf: &[u8]) -> Option<PeerId> {
    if buf.len() < OFF_ID + PUBKEY_LEN {
        return None;
    }
    let mut idb = [0u8; PUBKEY_LEN];
    idb.copy_from_slice(&buf[OFF_ID..OFF_ID + PUBKEY_LEN]);
    Some(PeerId::from_bytes(idb))
}

/// Décode le corps d'un paquet signé (ramené en forme `KIND_STATE`), SANS revérifier
/// le sceau — à n'appeler qu'après un `sig_ok` positif. Revalide type/version/finitude.
pub(crate) fn decode_canonical(buf: &[u8]) -> Option<PlayerState> {
    if buf.len() < STATE_SIZE {
        return None;
    }
    let mut body = [0u8; STATE_SIZE];
    body.copy_from_slice(&buf[..STATE_SIZE]);
    body[0] = KIND_STATE;
    decode(&body)
}

/// Vérifie le sceau PUIS décode (combo pratique, utilisé par les tests ; la
/// réception, elle, sépare `sig_ok` et `decode_canonical` pour la réputation).
#[cfg(test)]
pub(crate) fn decode_verified(buf: &[u8]) -> Option<PlayerState> {
    if !sig_ok(buf) {
        return None;
    }
    decode_canonical(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(seed: u8) -> PeerId {
        PeerId::from_bytes([seed; PUBKEY_LEN])
    }

    /// Un état qui fait l'aller-retour encode→decode doit revenir identique :
    /// c'est la garantie que les décalages d'octets sont cohérents des deux côtés.
    #[test]
    fn etat_survit_a_l_aller_retour() {
        let p = PlayerState {
            id: pid(7),
            x: 1.5, y: 0.7, z: -3.25,
            vx: -2.0, vy: 0.1, vz: 4.0,
            yaw: 1.2, pitch: -0.3,
            r: 0.8, g: 0.2, b: 1.1,
            parent: Some(pid(3)), seq: 99,
        };
        let d = decode(&encode(&p)).expect("doit se décoder");
        assert_eq!(d.id, p.id);
        assert_eq!(d.parent, Some(pid(3)));
        assert_eq!(d.seq, 99);
        assert_eq!(d.x, p.x);
        assert_eq!(d.z, p.z);
        assert_eq!(d.vz, p.vz);
        assert_eq!(d.b, p.b);
    }

    /// `parent = None` (autonome) survit à l'aller-retour (32 zéros → None).
    #[test]
    fn parent_absent_survit() {
        let mut p = etat_exemple();
        p.parent = None;
        let d = decode(&encode(&p)).expect("doit se décoder");
        assert_eq!(d.parent, None);
    }

    /// Un paquet d'une AUTRE version est rejeté (None), pas lu de travers.
    #[test]
    fn version_differente_est_rejetee() {
        let mut bytes = encode(&etat_exemple());
        bytes[1] = PROTO_VERSION.wrapping_add(1);
        assert!(decode(&bytes).is_none());
    }

    /// Un paquet porteur d'un NaN/Inf est rejeté (jamais laissé entrer).
    #[test]
    fn nan_est_rejete() {
        let mut p = etat_exemple();
        p.x = f32::NAN;
        assert!(decode(&encode(&p)).is_none());
        p.x = 0.0;
        p.vz = f32::INFINITY;
        assert!(decode(&encode(&p)).is_none());
    }

    fn etat_exemple() -> PlayerState {
        PlayerState {
            id: pid(5), x: 1.0, y: 0.7, z: -2.0, vx: 0.5, vy: 0.0, vz: -1.0,
            yaw: 0.3, pitch: 0.1, r: 0.9, g: 0.4, b: 0.2, parent: None, seq: 1,
        }
    }

    /// Un état signé par une vraie identité se vérifie et se décode ; son `id`
    /// décodé est bien la clé publique de l'émetteur (auto-certification).
    #[test]
    fn etat_signe_se_verifie_et_son_id_est_la_cle() {
        let identity = Identity::generate();
        let mut p = etat_exemple();
        p.id = identity.id(); // on émet sous NOTRE clé
        let signed = encode_signed(&p, &identity);
        let d = decode_verified(&signed).expect("sceau valide");
        assert_eq!(d.id, identity.id());
        assert_eq!(d.z, -2.0);
    }

    /// Le moindre octet du CORPS modifié casse le sceau → rejet (anti-falsification).
    #[test]
    fn etat_signe_altere_est_rejete() {
        let identity = Identity::generate();
        let mut p = etat_exemple();
        p.id = identity.id();
        let mut signed = encode_signed(&p, &identity);
        signed[OFF_FLOATS] ^= 0xFF; // on triture un octet de la position
        assert!(decode_verified(&signed).is_none());
    }

    /// USURPATION : un attaquant embarque la clé de la VICTIME dans `id`, mais signe
    /// avec SA propre clé. Le sceau, vérifié contre la clé embarquée (la victime),
    /// ne colle pas → rejet. Impossible de se faire passer pour un autre.
    #[test]
    fn usurpation_est_rejetee() {
        let victime = Identity::generate();
        let attaquant = Identity::generate();
        let mut p = etat_exemple();
        p.id = victime.id(); // je PRÉTENDS être la victime…
        let signed = encode_signed(&p, &attaquant); // … mais je signe avec MA clé
        assert!(decode_verified(&signed).is_none());
    }

    /// RELAY_FWD (12.3) : l'enveloppe de routage fait l'aller-retour, ET l'état SCELLÉ interne
    /// ressort VERBATIM + se vérifie encore (le relais ne casse pas le sceau bout-en-bout).
    #[test]
    fn relay_fwd_survit_a_l_aller_retour_et_garde_le_sceau() {
        let a = Identity::generate();
        let mut p = etat_exemple();
        p.id = a.id();
        let sealed = encode_signed(&p, &a); // l'état scellé de A (forme KIND_STATE)
        let dest = pid(42); // la clé de B (destinataire)
        let env = encode_relay_fwd(dest, &sealed);
        assert_eq!(env.len(), RELAY_FWD_SIZE); // 216 o
        let (recu_dest, recu_inner) = decode_relay_fwd(&env).expect("doit se décoder");
        assert_eq!(recu_dest, dest); // le routage a survécu
        assert_eq!(recu_inner, sealed); // l'état interne est rendu VERBATIM
        assert!(sig_ok(&recu_inner)); // et son sceau tient toujours → le relais ne forge rien
        assert_eq!(decode_verified(&recu_inner).unwrap().id, a.id()); // l'émetteur reste A
    }

    /// RELAY_FWD : une enveloppe malformée (mauvais type, mauvaise taille) est rejetée nettement,
    /// et un paquet d'état NORMAL n'est PAS lu comme une enveloppe (pas de croisement de format).
    #[test]
    fn relay_fwd_malforme_et_pas_de_croisement() {
        let a = Identity::generate();
        let mut p = etat_exemple();
        p.id = a.id();
        let sealed = encode_signed(&p, &a);
        let mut env = encode_relay_fwd(pid(7), &sealed);
        // mauvais type → rejet
        env[0] = KIND_STATE;
        assert!(decode_relay_fwd(&env).is_none());
        // taille non canonique → rejet
        assert!(decode_relay_fwd(&env[..RELAY_FWD_SIZE - 1]).is_none());
        // un état signé normal (182 o) n'est pas une enveloppe (216 o) → rejet par la taille
        assert!(decode_relay_fwd(&sealed).is_none());
        // et réciproquement, l'enveloppe (type 11) n'est pas décodée comme un état (type 1)
        let bonne = encode_relay_fwd(pid(7), &sealed);
        assert!(decode(&bonne).is_none());
    }

    /// L'enveloppe scellée résiste au RELAIS (bascule KIND_RELAY) mais pas à la
    /// falsification du contenu. C'est la garantie « parent porteur, pas faussaire ».
    #[test]
    fn enveloppe_scellee_survit_au_relais_mais_pas_a_la_falsification() {
        let identity = Identity::generate();
        let mut p = etat_exemple();
        p.id = identity.id();
        let mut signed = encode_signed(&p, &identity);
        mark_as_relay(&mut signed);
        assert!(decode_verified(&signed).is_some());
        signed[OFF_FLOATS + 2] ^= 0x7F; // un parent malveillant change la position
        assert!(decode_verified(&signed).is_none());
    }
}
