//! L'ORBE PARTAGÉE : le premier objet du monde qui n'appartient à PERSONNE par
//! naissance — et c'est tout l'intérêt.
//!
//! # La règle d'or du P2P sans serveur
//! Pour tout objet du monde, à chaque instant, UN SEUL pair décide de sa vérité :
//! on l'appelle le **maître** (l'autorité). Les autres ne font que recopier.
//!   - ton avatar  → tu en es le maître (toi seul décides où il est) ;
//!   - l'orbe      → personne au départ ; le DERNIER à l'avoir touchée en devient
//!                   le maître. La propriété saute donc de main en main.
//!
//! # Identité auto-certifiante (chap. 6.1)
//! Le maître n'est plus un numéro `u8` mais une **clé publique** (`PeerId`), portée
//! dans le paquet et qui sert à vérifier le sceau. Se proclamer maître exige donc
//! de POSSÉDER la clé privée correspondante : on ne peut pas usurper un autre maître.
//!
//! # Le protocole (paquet KIND_ORB)
//!   maître → pairs : id (clé) du maître, n° de version, position, vitesse, couleur.
//! Règle d'émission : SEUL le maître émet. Règle de réception : on accepte l'état
//! entrant s'il SUPPLANTE le nôtre (version plus haute, ou égale mais id plus petit).

use super::crypto::{verify, Identity, PeerId, PUBKEY_LEN, SIG_LEN};
use super::wire::{KIND_ORB, PROTO_VERSION};
use crate::math::Vec3;

/// Position de repos de l'orbe (au centre, à hauteur des yeux), avant toute prise.
const ORB_START: Vec3 = Vec3::new(0.0, 1.5, 0.0);
/// Délai sans nouvelle du maître au-delà duquel on le présume parti (s). Généreux
/// exprès (règle des vrais systèmes type Raft) : sinon une micro-coupure ferait
/// basculer l'orbe à tort.
const MASTER_TIMEOUT: f32 = 2.0;
/// Couleur de l'orbe tant que personne ne l'a touchée (blanc bleuté néon).
const NEUTRAL_COLOR: (f32, f32, f32) = (0.85, 0.85, 1.0);
/// Saut de version maximal toléré entre deux états d'orbe acceptés (chap. 5.3).
/// Un bond énorme (ex. vers 65535 pour verrouiller l'orbe à vie) est ABERRANT → on
/// le refuse et on inscrit une faute. 16 laisse de la marge pour des transferts manqués.
const MAX_ORB_VERSION_JUMP: u16 = 16;
/// Distance max (m) entre un nouveau maître et l'orbe au moment où il la revendique
/// (chap. 6.4). Pour devenir maître il faut avoir ÉTÉ près d'elle (preuve de contact)
/// — sinon on la « vole » à distance par incréments. Généreux (rayon orbe + joueur +
/// marge d'interpolation) pour ne jamais refuser une vraie frappe.
const CONTACT_DIST: f32 = 3.0;

/// Le paquet « état de l'orbe » tel qu'il voyage sur le réseau (avant/après octets).
pub(crate) struct OrbWire {
    pub owner: PeerId, // identité (clé) du maître ; portée dans le paquet
    pub version: u16,  // compteur de propriété : +1 à chaque transfert
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub r: f32, // couleur (celle du maître)
    pub g: f32,
    pub b: f32,
}

// Décalages, calculés à la main : [0] type | [1] version proto | [2..34] owner (clé,
//   32 o) | [34..36] version d'orbe (u16) | [36..72] 9 f32 (x,y,z, vx,vy,vz, r,g,b).
const OFF_OWNER: usize = 2;
const OFF_OVERSION: usize = OFF_OWNER + PUBKEY_LEN; // 34
const OFF_FLOATS: usize = OFF_OVERSION + 2; // 36
const ORB_SIZE: usize = OFF_FLOATS + 4 * 9; // 72

/// Sérialiser l'état de l'orbe en octets bruts (même sens little-endian que `message`).
pub(crate) fn encode_orb(o: &OrbWire) -> [u8; ORB_SIZE] {
    let mut buf = [0u8; ORB_SIZE];
    buf[0] = KIND_ORB;
    buf[1] = PROTO_VERSION;
    buf[OFF_OWNER..OFF_OWNER + PUBKEY_LEN].copy_from_slice(o.owner.bytes());
    buf[OFF_OVERSION..OFF_OVERSION + 2].copy_from_slice(&o.version.to_le_bytes());
    let f = OFF_FLOATS;
    buf[f..f + 4].copy_from_slice(&o.x.to_le_bytes());
    buf[f + 4..f + 8].copy_from_slice(&o.y.to_le_bytes());
    buf[f + 8..f + 12].copy_from_slice(&o.z.to_le_bytes());
    buf[f + 12..f + 16].copy_from_slice(&o.vx.to_le_bytes());
    buf[f + 16..f + 20].copy_from_slice(&o.vy.to_le_bytes());
    buf[f + 20..f + 24].copy_from_slice(&o.vz.to_le_bytes());
    buf[f + 24..f + 28].copy_from_slice(&o.r.to_le_bytes());
    buf[f + 28..f + 32].copy_from_slice(&o.g.to_le_bytes());
    buf[f + 32..f + 36].copy_from_slice(&o.b.to_le_bytes());
    buf
}

/// Reconstruire un `OrbWire` depuis les octets reçus. `None` si trop court, pas du
/// bon type/version, ou porteur d'un NaN/Inf — on ne fait jamais confiance au réseau.
pub(crate) fn decode_orb(buf: &[u8]) -> Option<OrbWire> {
    if buf.len() < ORB_SIZE || buf[0] != KIND_ORB || buf[1] != PROTO_VERSION {
        return None;
    }
    let mut ob = [0u8; PUBKEY_LEN];
    ob.copy_from_slice(&buf[OFF_OWNER..OFF_OWNER + PUBKEY_LEN]);
    let owner = PeerId::from_bytes(ob);
    let version = u16::from_le_bytes(buf[OFF_OVERSION..OFF_OVERSION + 2].try_into().ok()?);
    let f = OFF_FLOATS;
    let x = f32::from_le_bytes(buf[f..f + 4].try_into().ok()?);
    let y = f32::from_le_bytes(buf[f + 4..f + 8].try_into().ok()?);
    let z = f32::from_le_bytes(buf[f + 8..f + 12].try_into().ok()?);
    let vx = f32::from_le_bytes(buf[f + 12..f + 16].try_into().ok()?);
    let vy = f32::from_le_bytes(buf[f + 16..f + 20].try_into().ok()?);
    let vz = f32::from_le_bytes(buf[f + 20..f + 24].try_into().ok()?);
    let r = f32::from_le_bytes(buf[f + 24..f + 28].try_into().ok()?);
    let g = f32::from_le_bytes(buf[f + 28..f + 32].try_into().ok()?);
    let b = f32::from_le_bytes(buf[f + 32..f + 36].try_into().ok()?);
    if ![x, y, z, vx, vy, vz, r, g, b].iter().all(|f| f.is_finite()) {
        return None;
    }
    Some(OrbWire { owner, version, x, y, z, vx, vy, vz, r, g, b })
}

/// Lit l'id (clé) du maître revendiqué dans un paquet d'orbe, sans rien vérifier.
pub(crate) fn claimed_owner(buf: &[u8]) -> Option<PeerId> {
    if buf.len() < OFF_OWNER + PUBKEY_LEN {
        return None;
    }
    let mut ob = [0u8; PUBKEY_LEN];
    ob.copy_from_slice(&buf[OFF_OWNER..OFF_OWNER + PUBKEY_LEN]);
    Some(PeerId::from_bytes(ob))
}

/// Taille d'un état d'orbe SIGNÉ : corps (72 o) + sceau Ed25519 (64 o) = 136.
pub(crate) const SIGNED_ORB_SIZE: usize = ORB_SIZE + SIG_LEN;

/// Scelle l'état de l'orbe : le maître SIGNE le corps avec sa clé privée. Le corps
/// embarque déjà sa clé publique (`owner`) : personne d'autre ne peut produire un
/// sceau valide pour cette clé (chap. 5.3 + 6.1).
pub(crate) fn encode_orb_signed(o: &OrbWire, identity: &Identity) -> [u8; SIGNED_ORB_SIZE] {
    let body = encode_orb(o);
    let sig = identity.sign(&body);
    let mut out = [0u8; SIGNED_ORB_SIZE];
    out[..ORB_SIZE].copy_from_slice(&body);
    out[ORB_SIZE..].copy_from_slice(&sig);
    out
}

/// Le sceau d'un état d'orbe est-il valide ? On lit la clé du maître DIRECTEMENT
/// dans le paquet (`owner`, octets 2..34) et on vérifie contre elle (auto-certifié).
pub(crate) fn orb_sig_ok(buf: &[u8]) -> bool {
    if buf.len() < SIGNED_ORB_SIZE {
        return false;
    }
    let mut pubkey = [0u8; PUBKEY_LEN];
    pubkey.copy_from_slice(&buf[OFF_OWNER..OFF_OWNER + PUBKEY_LEN]);
    let mut sig = [0u8; SIG_LEN];
    sig.copy_from_slice(&buf[ORB_SIZE..SIGNED_ORB_SIZE]);
    verify(&buf[..ORB_SIZE], &sig, &pubkey)
}

/// Vérifie le sceau PUIS décode (combo pratique pour les tests).
#[cfg(test)]
pub(crate) fn decode_orb_verified(buf: &[u8]) -> Option<OrbWire> {
    if !orb_sig_ok(buf) {
        return None;
    }
    decode_orb(buf)
}

/// Verdict de l'application d'un état d'orbe reçu (pour la réputation).
pub(crate) enum OrbApply {
    Applied,     // accepté : il supplante notre état
    Ignored,     // ignoré : ne supplante pas (plus ancien) — bénin
    Implausible, // refusé : saut de version aberrant (tentative de vol/gel) → faute
    NoContact,   // refusé : se proclame maître sans avoir été près de l'orbe → faute (6.4)
}

/// L'état logique de l'orbe — PUR (aucun type moteur). La présentation 3D (sphère,
/// couleur, rebonds visuels) est reconstruite côté client (Unreal) à partir de cet état.
pub struct Orb {
    pub(crate) pos: Vec3,
    pub(crate) vel: Vec3,
    pub(crate) owner: Option<PeerId>, // None = personne ne l'a encore touchée
    pub(crate) version: u16,
    pub(crate) color: (f32, f32, f32),
    last_heard: f32, // instant du dernier paquet reçu du maître (migration)
}

impl Orb {
    /// Construit une `Orb` au repos (état initial : sans maître). Utilisée par le nœud
    /// headless (`Bot`) et le sidecar — même logique d'autorité que partout.
    pub(crate) fn headless() -> Orb {
        Orb {
            pos: ORB_START,
            vel: Vec3::ZERO,
            owner: None,
            version: 0,
            color: NEUTRAL_COLOR,
            last_heard: 0.0,
        }
    }
}

/// Décide si un état entrant doit SUPPLANTER l'état courant. Toute la logique
/// d'autorité en une fonction :
///   - version plus haute        → touche plus récente, il l'emporte ;
///   - version égale + id ≤       → flux du maître courant (==) OU départage d'une
///                                  course en faveur du plus petit id (<).
fn supersedes(in_ver: u16, in_owner: PeerId, cur_ver: u16, cur_owner: Option<PeerId>) -> bool {
    match cur_owner {
        None => true, // pas encore de maître : le premier paquet fait foi
        Some(cur) => in_ver > cur_ver || (in_ver == cur_ver && in_owner <= cur),
    }
}

/// Appliquer un paquet d'orbe reçu du réseau. On n'écrase notre état QUE s'il est
/// supplanté. `now` date ce battement : il prouve que le maître est vivant.
/// `claimer_pos` = dernière position connue du maître revendiqué (None si on ne le
/// « voit » pas). Sert à la PREUVE DE CONTACT (chap. 6.4) lors d'un transfert.
pub(crate) fn apply_incoming(orb: &mut Orb, w: OrbWire, now: f32, claimer_pos: Option<Vec3>) -> OrbApply {
    if !supersedes(w.version, w.owner, orb.version, orb.owner) {
        return OrbApply::Ignored; // état plus ancien : on garde le nôtre (bénin)
    }
    // SHIELD (chap. 5.3) : même signé, on refuse un SAUT de version aberrant. Un
    // bond vers 65535 (verrou) ou très loin devant est impossible légitimement.
    if orb.owner.is_some() {
        let jump = w.version.wrapping_sub(orb.version);
        if jump > MAX_ORB_VERSION_JUMP {
            return OrbApply::Implausible;
        }
    }
    // PREUVE DE CONTACT (chap. 6.4) : pour devenir maître, il faut avoir été PRÈS de
    // l'orbe. Exception : la MIGRATION (l'ancien maître s'est tu depuis MASTER_TIMEOUT)
    // — là, le remplaçant élu peut être n'importe où, on tolère sans contact. Un
    // maître INCONNU (qu'on ne voit pas bouger) n'est accepté QUE dans ce cas.
    if orb.owner != Some(w.owner) {
        let migration = orb.owner.is_some() && (now - orb.last_heard >= MASTER_TIMEOUT);
        let claim_pos = Vec3::new(w.x, w.y, w.z);
        let near = match claimer_pos {
            Some(p) => p.distance_squared(claim_pos) <= CONTACT_DIST * CONTACT_DIST,
            None => migration,
        };
        if !near {
            return OrbApply::NoContact; // vol à distance / par incréments → faute
        }
        println!("Orbe : j'adopte le maître {} (v{}) reçu du réseau.", w.owner.short(), w.version);
    }
    orb.owner = Some(w.owner);
    orb.version = w.version;
    orb.pos = Vec3::new(w.x, w.y, w.z);
    orb.vel = Vec3::new(w.vx, w.vy, w.vz);
    orb.color = (w.r, w.g, w.b);
    orb.last_heard = now;
    OrbApply::Applied
}

// NOTE — MIGRATION D'HÔTE. La RÉCEPTION d'une migration (adopter un nouveau maître après
// que l'ancien s'est tu > `MASTER_TIMEOUT`) vit dans `apply_incoming` ci-dessus. L'AUTO-
// ÉLECTION (se proclamer soi-même remplaçant : plus petit id, version +1) était portée par
// un système client ; elle reviendra ici, en fonction pure, quand le jeu (Unreal) câblera
// l'autorité d'objet — aucun nœud headless ne s'auto-élit aujourd'hui.

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(seed: u8) -> PeerId {
        PeerId::from_bytes([seed; PUBKEY_LEN])
    }

    /// `supersedes` est le CŒUR de l'autorité : on fige ses règles par des tests.
    #[test]
    fn supersedes_regles_d_autorite() {
        assert!(supersedes(0, pid(5), 0, None));
        assert!(supersedes(10, pid(9), 5, Some(pid(2))));
        assert!(!supersedes(4, pid(1), 5, Some(pid(2))));
        // Égalité de version : le PLUS PETIT id gagne (départage déterministe).
        assert!(supersedes(7, pid(1), 7, Some(pid(3)))); // 1 < 3 → l'entrant gagne
        assert!(!supersedes(7, pid(8), 7, Some(pid(3)))); // 8 > 3 → on garde le nôtre
        assert!(supersedes(7, pid(3), 7, Some(pid(3)))); // flux normal du maître
    }

    /// L'état de l'orbe doit survivre à l'aller-retour encode→decode (offsets sûrs).
    #[test]
    fn orbe_survit_a_l_aller_retour() {
        let w = OrbWire {
            owner: pid(4), version: 42,
            x: 1.0, y: 1.5, z: -2.0,
            vx: 3.0, vy: -1.0, vz: 0.5,
            r: 0.9, g: 0.1, b: 0.8,
        };
        let d = decode_orb(&encode_orb(&w)).expect("doit se décoder");
        assert_eq!(d.owner, pid(4));
        assert_eq!(d.version, 42);
        assert_eq!(d.x, 1.0);
        assert_eq!(d.vz, 0.5);
        assert_eq!(d.b, 0.8);
    }

    /// Un paquet d'orbe porteur d'un NaN/Inf est rejeté (sinon la balle part à l'∞).
    #[test]
    fn orbe_nan_est_rejete() {
        let w = OrbWire {
            owner: pid(1), version: 1,
            x: f32::NAN, y: 0.0, z: 0.0, vx: 0.0, vy: 0.0, vz: 0.0,
            r: 0.0, g: 0.0, b: 0.0,
        };
        assert!(decode_orb(&encode_orb(&w)).is_none());
    }

    /// Un paquet d'orbe d'une autre version protocole est rejeté, pas lu de travers.
    #[test]
    fn orbe_version_differente_rejetee() {
        let w = orbe_exemple(pid(1), 1);
        let mut bytes = encode_orb(&w);
        bytes[1] = PROTO_VERSION.wrapping_add(1);
        assert!(decode_orb(&bytes).is_none());
    }

    fn orbe_exemple(owner: PeerId, version: u16) -> OrbWire {
        OrbWire {
            owner, version,
            x: 0.0, y: 1.5, z: 0.0, vx: 1.0, vy: 0.0, vz: 0.0,
            r: 1.0, g: 0.0, b: 0.0,
        }
    }

    /// Une orbe SIGNÉE se vérifie ; un paquet revendiquant un autre maître mais signé
    /// par l'imposteur est rejeté (le sceau ne colle pas à la clé embarquée).
    #[test]
    fn orbe_signee_se_verifie() {
        let maitre = Identity::generate();
        let signed = encode_orb_signed(&orbe_exemple(maitre.id(), 7), &maitre);
        assert!(decode_orb_verified(&signed).is_some());
        // L'imposteur signe un paquet qui prétend « maître = la vraie clé du maître ».
        let imposteur = Identity::generate();
        let forge = encode_orb_signed(&orbe_exemple(maitre.id(), 7), &imposteur);
        assert!(decode_orb_verified(&forge).is_none());
    }

    fn orb_test(owner: Option<PeerId>, version: u16) -> Orb {
        Orb {
            pos: Vec3::ZERO, vel: Vec3::ZERO, owner, version,
            color: NEUTRAL_COLOR, last_heard: 0.0,
        }
    }

    /// Position « au contact » de l'orbe d'exemple (qui est à (0, 1.5, 0)).
    fn pres() -> Option<Vec3> {
        Some(Vec3::new(0.0, 1.5, 0.0))
    }

    /// Un transfert normal (+1) AVEC contact est appliqué ; un SAUT vers 65535
    /// (verrou) est refusé avant même le test de contact.
    #[test]
    fn apply_borne_le_saut_de_version() {
        let mut orb = orb_test(Some(pid(2)), 5);
        assert!(matches!(apply_incoming(&mut orb, orbe_exemple(pid(9), 6), 1.0, pres()), OrbApply::Applied));

        let mut orb = orb_test(Some(pid(2)), 5);
        assert!(matches!(
            apply_incoming(&mut orb, orbe_exemple(pid(9), 65535), 1.0, pres()),
            OrbApply::Implausible
        ));
        assert_eq!(orb.owner, Some(pid(2)));
        assert_eq!(orb.version, 5);
    }

    /// Un état plus ancien est simplement ignoré (bénin, pas une faute).
    #[test]
    fn apply_ignore_un_etat_plus_ancien() {
        let mut orb = orb_test(Some(pid(2)), 10);
        assert!(matches!(apply_incoming(&mut orb, orbe_exemple(pid(9), 4), 1.0, pres()), OrbApply::Ignored));
    }

    /// ORB-CREEP (chap. 6.4) : se proclamer maître par +1 SANS être près de l'orbe
    /// (claimer inconnu, orbe fraîche) → NoContact. Avec contact → accepté.
    #[test]
    fn apply_exige_un_contact() {
        // Maître courant frais (pas une migration), revendiqueur INCONNU (None) → refus.
        let mut orb = orb_test(Some(pid(2)), 5);
        orb.last_heard = 1.0;
        assert!(matches!(
            apply_incoming(&mut orb, orbe_exemple(pid(9), 6), 1.0, None),
            OrbApply::NoContact
        ));
        assert_eq!(orb.owner, Some(pid(2))); // rien volé

        // Même chose mais le revendiqueur est VU près de l'orbe → accepté.
        let mut orb = orb_test(Some(pid(2)), 5);
        orb.last_heard = 1.0;
        assert!(matches!(
            apply_incoming(&mut orb, orbe_exemple(pid(9), 6), 1.0, pres()),
            OrbApply::Applied
        ));

        // MIGRATION : l'ancien maître s'est tu (> MASTER_TIMEOUT) → un inconnu est toléré.
        let mut orb = orb_test(Some(pid(2)), 5);
        orb.last_heard = 0.0;
        assert!(matches!(
            apply_incoming(&mut orb, orbe_exemple(pid(9), 6), 10.0, None),
            OrbApply::Applied
        ));
    }
}
