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

use super::aoi::MAX_ENGAGED;
use super::crypto::{verify, Identity, PeerId, PUBKEY_LEN, SIG_LEN};
use super::wire::{
    KIND_ENGAGED, KIND_RECV_BUDGET, KIND_RELAY, KIND_RELAY_FWD, KIND_STATE, KIND_STATE_BUNDLE,
    PROTO_VERSION,
};

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
// ENVELOPPE DE RELAIS NAT (chap. 12.3 / 12.3-G / D17) — UNICAST A → rendez-vous → B.
// ----------------------------------------------------------------------------
// Quand A ne peut PAS percer le NAT de B (perçage abandonné), il demande au seul point public
// commun (le rendez-vous, v1) de router un paquet SCELLÉ vers B. C'est une simple enveloppe de
// ROUTAGE autour d'un paquet déjà signé — on NE re-signe RIEN (le sceau bout-en-bout tient).
//   [0] KIND_RELAY_FWD | [1] version | [2..34] dest_id (clé de B, ROUTAGE seul, NON signé)
//   | [34..] le PAYLOAD scellé, VERBATIM, de longueur LIBRE.
// 12.3-G — GÉNÉRALISATION : le payload n'est plus figé à l'état joueur (182 o). C'est n'importe quel
// paquet déjà scellé (état KIND_STATE 182 o, orbe KIND_ORB 136 o, plus tard gossip…) → l'orbe et les
// objets partagés du monde traversent eux aussi le relais. ⭐ Pour un état joueur, l'enveloppe fait
// 34 + 182 = 216 o : STRICTEMENT le format 12.3 d'origine (octet pour octet) → le relais avatar PROUVÉ
// ne bouge pas d'un octet. Le destinataire n'est PAS signé : au pire, un dest falsifié fait router le
// paquet (toujours scellé, infalsifiable) vers le mauvais pair, qui vérifie le sceau — aucune forge.
/// En-tête d'une enveloppe de relais = type (1) + version (1) + dest (32) = 34 o. Le PAYLOAD scellé
/// qui suit est de longueur LIBRE (12.3-G).
pub(crate) const RELAY_FWD_HEADER: usize = 2 + PUBKEY_LEN;

/// Décode une enveloppe `KIND_RELAY_FWD` → `(dest, payload_scellé_verbatim)`, ou `None` si malformée
/// (mauvais type/version, ou aucun payload). Le payload est rendu VERBATIM (déjà scellé : état, orbe,
/// …) : le rendez-vous ne fait que le PORTER ; c'est le DESTINATAIRE qui vérifie le sceau à la
/// réception (états ET orbes s'auto-vérifient — cf. PAPIER-WIRE 12.3-G).
pub(crate) fn decode_relay_fwd(buf: &[u8]) -> Option<(PeerId, &[u8])> {
    if buf.len() <= RELAY_FWD_HEADER || buf[0] != KIND_RELAY_FWD || buf[1] != PROTO_VERSION {
        return None;
    }
    let mut db = [0u8; PUBKEY_LEN];
    db.copy_from_slice(&buf[2..2 + PUBKEY_LEN]);
    let dest = PeerId::from_bytes(db);
    Some((dest, &buf[RELAY_FWD_HEADER..]))
}

/// Construit une enveloppe `KIND_RELAY_FWD` autour d'un paquet DÉJÀ scellé (état, orbe, …), à
/// destination de `dest`. Émise par le client au perçage ABANDONNÉ (derrière le drapeau
/// `RELAY_FALLBACK`) vers le rendez-vous, qui la route vers `dest`. Réciproque de `decode_relay_fwd`.
/// Pour un état joueur de 182 o, le résultat est byte-pour-byte le format 12.3 d'origine.
pub(crate) fn encode_relay_fwd(dest: PeerId, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(RELAY_FWD_HEADER + payload.len());
    out.push(KIND_RELAY_FWD);
    out.push(PROTO_VERSION);
    out.extend_from_slice(dest.bytes());
    out.extend_from_slice(payload);
    out
}

// ----------------------------------------------------------------------------
// LOT D'ÉTATS (KIND_STATE_BUNDLE) — redondance TEMPORELLE budget-free (12.3 / D17).
// ----------------------------------------------------------------------------
// Sur un relais lossy (4G/CGNAT, ~88 % de perte), au lieu d'émettre k COPIES séparées du même état
// (qui chacune coûtent au budget anti-amplification du rendez-vous), on émet UN SEUL paquet portant
// les K DERNIERS états signés. Un seq donné réapparaît dans les K lots consécutifs → il n'est perdu
// que si les K lots sont perdus (p^K), MAIS à 1 envoi/tick (aucune charge budget en plus) et réparti
// sur K ticks (bat aussi la perte EN RAFALE). Le récepteur dédoublonne nativement (accept_seq).
//   [0] KIND_STATE_BUNDLE | [1] version | [2] count(u8) | [3..] count × SIGNED_STATE_SIZE (182 o).
// Chaque élément est un état signé COMPLET (forme KIND_STATE) qui s'auto-vérifie indépendamment.
/// En-tête d'un lot = type (1) + version (1) + nombre d'états (1) = 3 o.
pub(crate) const STATE_BUNDLE_HEADER: usize = 3;

/// Construit un LOT à partir d'états DÉJÀ scellés (chacun de `SIGNED_STATE_SIZE` octets, forme
/// KIND_STATE), du plus ANCIEN au plus récent. `states` est tronqué à 255 (compteur u8). Réciproque
/// de `decode_state_bundle`. Émis UNIQUEMENT sur le chemin relais sous `RELAY_REDUNDANCY ≥ 2`.
pub(crate) fn encode_state_bundle(states: &[&[u8]]) -> Vec<u8> {
    let n = states.len().min(255);
    let mut out = Vec::with_capacity(STATE_BUNDLE_HEADER + n * SIGNED_STATE_SIZE);
    out.push(KIND_STATE_BUNDLE);
    out.push(PROTO_VERSION);
    out.push(n as u8);
    for s in states.iter().take(n) {
        out.extend_from_slice(s);
    }
    out
}

/// Décode un LOT → la liste des tranches d'états signés (chacune `SIGNED_STATE_SIZE`), du plus ancien
/// au plus récent, ou `None` si malformé (mauvais type/version, ou longueur incohérente avec `count`).
/// Chaque tranche est rendue VERBATIM : c'est le destinataire qui vérifie chaque sceau.
pub(crate) fn decode_state_bundle(buf: &[u8]) -> Option<Vec<&[u8]>> {
    if buf.len() < STATE_BUNDLE_HEADER || buf[0] != KIND_STATE_BUNDLE || buf[1] != PROTO_VERSION {
        return None;
    }
    let count = buf[2] as usize;
    if buf.len() != STATE_BUNDLE_HEADER + count * SIGNED_STATE_SIZE {
        return None; // longueur déclarée ≠ longueur réelle → on jette (jamais de lecture partielle)
    }
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        let off = STATE_BUNDLE_HEADER + i * SIGNED_STATE_SIZE;
        out.push(&buf[off..off + SIGNED_STATE_SIZE]);
    }
    Some(out)
}

// ----------------------------------------------------------------------------
// ANNONCE DE BUDGET DE RÉCEPTION (KIND_RECV_BUDGET) — couche 2, AoI BILATÉRALE.
//   [0] KIND_RECV_BUDGET | [1] version | [2..34] id de l'émetteur (à qui le plafond appartient)
//   | [34..38] cap (f32 Hz, LE).
// Un nœud annonce à ses émetteurs « ne m'envoie pas plus vite que `cap` Hz ». ADVISORY (non signé
// v1) : au pire un attaquant fait RÉDUIRE le trafic vers une victime (jamais l'augmenter) ; on
// n'accepte d'ailleurs un cap QUE depuis l'adresse du pair concerné (côté réception, bot.rs). Le
// `cap` est TOUJOURS fini sur le wire : `∞` (pas de surcharge) = on N'ÉMET PAS d'annonce, et le
// stock côté émetteur expire → retour à « pas de bride ». Gaté `AOI_BILATERAL` (défaut OFF).
// ----------------------------------------------------------------------------
const RECV_BUDGET_SIZE: usize = 2 + PUBKEY_LEN + 4; // 38 o

/// Encode une annonce de budget de réception : mon id + le plafond (Hz) que mes émetteurs
/// doivent respecter. `cap_hz` doit être fini et ≥ 0 (on n'annonce jamais `∞` : on se tait).
pub(crate) fn encode_recv_budget(id: &PeerId, cap_hz: f32) -> [u8; RECV_BUDGET_SIZE] {
    let mut buf = [0u8; RECV_BUDGET_SIZE];
    buf[0] = KIND_RECV_BUDGET;
    buf[1] = PROTO_VERSION;
    buf[2..2 + PUBKEY_LEN].copy_from_slice(id.bytes());
    buf[2 + PUBKEY_LEN..].copy_from_slice(&cap_hz.to_le_bytes());
    buf
}

/// Décode une annonce de budget → `(id du pair, cap Hz)`, ou `None` si malformée. On REJETTE
/// tout cap non fini ou négatif (on ne fait jamais confiance aveuglément au réseau ; un cap
/// pourri ne doit pas pouvoir devenir un plafond aberrant).
pub(crate) fn decode_recv_budget(buf: &[u8]) -> Option<(PeerId, f32)> {
    if buf.len() < RECV_BUDGET_SIZE || buf[0] != KIND_RECV_BUDGET || buf[1] != PROTO_VERSION {
        return None;
    }
    let mut idb = [0u8; PUBKEY_LEN];
    idb.copy_from_slice(&buf[2..2 + PUBKEY_LEN]);
    let id = PeerId::from_bytes(idb);
    let cap = f32::from_le_bytes(buf[2 + PUBKEY_LEN..RECV_BUDGET_SIZE].try_into().ok()?);
    if !cap.is_finite() || cap < 0.0 {
        return None;
    }
    Some((id, cap))
}

// ----------------------------------------------------------------------------
// DÉCLARATION D'ENGAGEMENT (KIND_ENGAGED) — pertinence TRANSITIVE (D29, T1).
//   [0] KIND_ENGAGED | [1] version | [2..34] id de l'émetteur (clé, AUTO-CERTIFIANTE)
//   | [34] count(u8 ≤ MAX_ENGAGED) | [35..35+count*32] les ids des partenaires
//   | [..+64] SIGNATURE du corps (tout ce qui précède le sceau).
// « Voici les quelques pairs avec qui je suis en interaction. » Sert à RÉHAUSSER ces tiers chez
// ceux qui M'ONT au focus (un pair LOIN mais engagé avec mon ami proche redevient pertinent).
// SIGNÉ comme un état : la clé est embarquée (champ id) et vérifie le sceau → un pair ne peut
// déclarer QUE SES PROPRES engagements (A ne peut pas prétendre que B est engagé avec C), et son
// « pouvoir de recommandation » reste borné par SA position (`TRANSITIVE_FRACTION × base[A]` ;
// un émetteur lointain a une base ~socle → il ne promeut personne). Variable mais MINUSCULE, et
// basse cadence (l'engagement change lentement) → on N'alourdit PAS le paquet d'état à 20 Hz.
// `engaged` vide → on N'ÉMET RIEN : aucun paquet, comportement byte-intact (comme l'AoI bilatérale).
// ----------------------------------------------------------------------------
/// En-tête d'une déclaration = type (1) + version (1) + id émetteur (32) + count (1) = 35 o.
#[allow(dead_code)] // EN ATTENTE : wire prouvé (tests) ; émis/reçu à l'étape 2 (refresh_focus).
const ENGAGED_HEADER: usize = 2 + PUBKEY_LEN + 1;

/// Scelle une déclaration d'engagement : mon id (clé) + mes (≤ `MAX_ENGAGED`) partenaires, SIGNÉE
/// avec ma clé privée. `engaged` est tronqué à `MAX_ENGAGED` (la borne du protocole). À n'émettre
/// QUE si `engaged` est non vide (sinon il n'y a rien à dire → on se tait, défaut byte-intact).
#[allow(dead_code)] // EN ATTENTE : prouvé par test ; appelé à l'étape 2 (émission basse cadence).
pub(crate) fn encode_engaged(identity: &Identity, engaged: &[PeerId]) -> Vec<u8> {
    let n = engaged.len().min(MAX_ENGAGED);
    let body_len = ENGAGED_HEADER + n * PUBKEY_LEN;
    let mut out = Vec::with_capacity(body_len + SIG_LEN);
    out.push(KIND_ENGAGED);
    out.push(PROTO_VERSION);
    out.extend_from_slice(identity.id().bytes());
    out.push(n as u8);
    for id in engaged.iter().take(n) {
        out.extend_from_slice(id.bytes());
    }
    let sig = identity.sign(&out); // signe le CORPS (en-tête + ids)
    out.extend_from_slice(&sig);
    out
}

/// Décode ET VÉRIFIE une déclaration d'engagement → `(émetteur, partenaires)`, ou `None` si
/// malformée (mauvais type/version, longueur incohérente, `count > MAX_ENGAGED`) ou si le SCEAU
/// ne colle pas à la clé embarquée. Contrairement à l'état, on vérifie le sceau ICI (la
/// déclaration ne passe pas par le chemin de réputation) : un appelant qui reçoit `Some` tient
/// une déclaration AUTHENTIQUE de l'émetteur lui-même. On REJETTE un id nul comme partenaire.
#[allow(dead_code)] // EN ATTENTE : prouvé par test ; appelé à l'étape 2 (réception, bot.rs).
pub(crate) fn decode_engaged(buf: &[u8]) -> Option<(PeerId, Vec<PeerId>)> {
    if buf.len() < ENGAGED_HEADER || buf[0] != KIND_ENGAGED || buf[1] != PROTO_VERSION {
        return None;
    }
    let count = buf[ENGAGED_HEADER - 1] as usize;
    if count > MAX_ENGAGED {
        return None; // un pair ne peut pas se déclarer engagé avec plus que la borne du protocole
    }
    let body_len = ENGAGED_HEADER + count * PUBKEY_LEN;
    if buf.len() != body_len + SIG_LEN {
        return None; // longueur déclarée ≠ longueur réelle → on jette (jamais de lecture partielle)
    }

    // Clé publique embarquée (champ id) → c'est ELLE qui vérifie le sceau (auto-certification).
    let mut pubkey = [0u8; PUBKEY_LEN];
    pubkey.copy_from_slice(&buf[OFF_ID..OFF_ID + PUBKEY_LEN]);
    let mut sig = [0u8; SIG_LEN];
    sig.copy_from_slice(&buf[body_len..body_len + SIG_LEN]);
    if !verify(&buf[..body_len], &sig, &pubkey) {
        return None; // sceau invalide → ni le bon émetteur, ni un corps intact
    }

    let sender = PeerId::from_bytes(pubkey);
    let mut engaged = Vec::with_capacity(count);
    for i in 0..count {
        let off = ENGAGED_HEADER + i * PUBKEY_LEN;
        let mut idb = [0u8; PUBKEY_LEN];
        idb.copy_from_slice(&buf[off..off + PUBKEY_LEN]);
        let id = PeerId::from_bytes(idb);
        if !id.is_none() {
            engaged.push(id); // on ignore les ids nuls (bourrage), on garde les vrais partenaires
        }
    }
    Some((sender, engaged))
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

    /// LOT d'états : round-trip exact (les tranches ressortent VERBATIM, dans l'ordre), chaque
    /// élément reste un état signé valide, et une longueur trafiquée est REJETÉE (jamais de lecture
    /// partielle). C'est la redondance temporelle budget-free du relais (12.3 / D17).
    #[test]
    fn lot_d_etats_round_trip_et_rejette_la_longueur_fausse() {
        let a = Identity::generate();
        let mut s1 = etat_exemple();
        s1.id = a.id();
        s1.seq = 7;
        let mut s2 = s1.clone();
        s2.seq = 8;
        let b1 = encode_signed(&s1, &a);
        let b2 = encode_signed(&s2, &a);

        let lot = encode_state_bundle(&[&b1[..], &b2[..]]);
        assert_eq!(lot[0], KIND_STATE_BUNDLE);
        assert_eq!(lot.len(), STATE_BUNDLE_HEADER + 2 * SIGNED_STATE_SIZE);

        let parts = decode_state_bundle(&lot).expect("lot valide");
        assert_eq!(parts.len(), 2);
        assert_eq!(parts[0], &b1[..]); // ordre préservé : ancien d'abord
        assert_eq!(parts[1], &b2[..]);
        // chaque tranche s'auto-vérifie indépendamment
        assert_eq!(decode_verified(parts[0]).unwrap().seq, 7);
        assert_eq!(decode_verified(parts[1]).unwrap().seq, 8);

        // longueur déclarée incohérente (un octet en trop) → rejet, pas de lecture partielle.
        let mut trafique = lot.clone();
        trafique.push(0);
        assert!(decode_state_bundle(&trafique).is_none());
    }

    /// COUCHE 2 — l'annonce de budget de réception fait l'aller-retour (id + cap préservés), et
    /// un cap non fini (NaN/∞) ou négatif est REJETÉ (on ne laisse jamais un plafond aberrant
    /// entrer depuis le réseau).
    #[test]
    fn annonce_budget_round_trip_et_rejette_cap_aberrant() {
        let id = Identity::generate().id();
        let buf = encode_recv_budget(&id, 12.5);
        assert_eq!(buf[0], KIND_RECV_BUDGET);
        let (got_id, cap) = decode_recv_budget(&buf).expect("annonce valide");
        assert_eq!(got_id, id);
        assert!((cap - 12.5).abs() < 1.0e-6);

        // cap = +∞ injecté à la main sur le wire → rejeté (on n'annonce jamais ∞, et on s'en protège).
        let mut pourri = buf;
        pourri[34..38].copy_from_slice(&f32::INFINITY.to_le_bytes());
        assert!(decode_recv_budget(&pourri).is_none());
        // trop court → rejeté.
        assert!(decode_recv_budget(&buf[..10]).is_none());
    }

    /// D29 — la déclaration d'ENGAGEMENT fait l'aller-retour : l'émetteur (sa clé) et la liste
    /// de partenaires ressortent intacts, et le sceau se vérifie (auto-certification).
    #[test]
    fn engaged_round_trip_signe() {
        let moi = Identity::generate();
        let p1 = pid(11);
        let p2 = pid(22);
        let buf = encode_engaged(&moi, &[p1, p2]);
        assert_eq!(buf[0], KIND_ENGAGED);
        let (sender, partners) = decode_engaged(&buf).expect("déclaration valide");
        assert_eq!(sender, moi.id()); // l'émetteur est bien MA clé
        assert_eq!(partners, vec![p1, p2]); // les partenaires, dans l'ordre
    }

    /// Liste vide : on n'émet rien d'utile, mais SI on encode quand même, ça reste cohérent
    /// (en-tête + sceau, zéro partenaire). Le défaut du protocole reste « on se tait ».
    #[test]
    fn engaged_vide_est_coherent() {
        let moi = Identity::generate();
        let buf = encode_engaged(&moi, &[]);
        let (sender, partners) = decode_engaged(&buf).expect("déclaration vide valide");
        assert_eq!(sender, moi.id());
        assert!(partners.is_empty());
    }

    /// La liste est TRONQUÉE à `MAX_ENGAGED` à l'émission, et une `count` trafiquée AU-DELÀ
    /// de la borne est REJETÉE au décodage (un pair ne peut pas rehausser des centaines de tiers).
    #[test]
    fn engaged_borne_a_max_engaged() {
        let moi = Identity::generate();
        let trop: Vec<PeerId> = (1..=(MAX_ENGAGED as u8 + 3)).map(pid).collect();
        let buf = encode_engaged(&moi, &trop);
        let (_, partners) = decode_engaged(&buf).expect("valide");
        assert_eq!(partners.len(), MAX_ENGAGED); // tronqué à la borne, pas plus

        // count trafiqué à MAX_ENGAGED+1 (longueur incohérente) → rejet net.
        let mut pourri = encode_engaged(&moi, &[pid(1)]);
        pourri[2 + PUBKEY_LEN] = (MAX_ENGAGED + 1) as u8;
        assert!(decode_engaged(&pourri).is_none());
    }

    /// Le moindre octet du corps modifié casse le sceau → rejet (anti-falsification), et un
    /// paquet trop court est rejeté nettement.
    #[test]
    fn engaged_altere_ou_court_est_rejete() {
        let moi = Identity::generate();
        let mut buf = encode_engaged(&moi, &[pid(7)]);
        buf[2 + PUBKEY_LEN + 1] ^= 0xFF; // on triture le 1er octet d'un id partenaire
        assert!(decode_engaged(&buf).is_none());
        let buf = encode_engaged(&moi, &[pid(7)]);
        assert!(decode_engaged(&buf[..10]).is_none());
    }

    /// USURPATION : un attaquant met la clé de la VICTIME dans `id` mais signe avec SA clé →
    /// le sceau (vérifié contre la clé embarquée) ne colle pas → rejet. On ne peut déclarer
    /// QUE SES PROPRES engagements.
    #[test]
    fn engaged_usurpation_est_rejetee() {
        let victime = Identity::generate();
        let attaquant = Identity::generate();
        // On fabrique à la main un corps qui PRÉTEND venir de la victime, signé par l'attaquant.
        let mut body = Vec::new();
        body.push(KIND_ENGAGED);
        body.push(PROTO_VERSION);
        body.extend_from_slice(victime.id().bytes()); // je PRÉTENDS être la victime…
        body.push(1);
        body.extend_from_slice(pid(9).bytes());
        let sig = attaquant.sign(&body); // … mais je signe avec MA clé
        body.extend_from_slice(&sig);
        assert!(decode_engaged(&body).is_none());
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
        let (recu_dest, recu_inner) = decode_relay_fwd(&env).expect("doit se décoder");
        assert_eq!(recu_dest, dest); // le routage a survécu
        assert_eq!(recu_inner, &sealed[..]); // l'état interne est rendu VERBATIM
        assert!(sig_ok(recu_inner)); // et son sceau tient toujours → le relais ne forge rien
        assert_eq!(decode_verified(recu_inner).unwrap().id, a.id()); // l'émetteur reste A
    }

    /// 12.3-G : généraliser le payload NE CHANGE PAS l'enveloppe d'avatar. Pour un état joueur de
    /// 182 o, le format reste byte-pour-byte celui du 12.3 PROUVÉ (en-tête 34 o + état verbatim =
    /// 216 o). C'est le garde-fou de non-régression : la base relais ne bouge pas d'un octet.
    #[test]
    fn relay_fwd_avatar_reste_byte_pour_byte_le_format_12_3() {
        let a = Identity::generate();
        let mut p = etat_exemple();
        p.id = a.id();
        let sealed = encode_signed(&p, &a);
        let dest = pid(42);
        let env = encode_relay_fwd(dest, &sealed);
        assert_eq!(env.len(), RELAY_FWD_HEADER + SIGNED_STATE_SIZE); // 34 + 182 = 216 (taille 12.3)
        assert_eq!(env[0], KIND_RELAY_FWD);
        assert_eq!(env[1], PROTO_VERSION);
        assert_eq!(&env[2..2 + PUBKEY_LEN], dest.bytes()); // dest verbatim
        assert_eq!(&env[RELAY_FWD_HEADER..], &sealed[..]); // état verbatim, à l'octet près
    }

    /// 12.3-G : un payload de longueur LIBRE (≠ 182 o, ici 136 o = la taille d'une orbe scellée) fait
    /// l'aller-retour VERBATIM. C'est ce qui permettra à l'orbe (et plus tard au gossip) d'emprunter
    /// le relais — le format porte n'importe quel objet partagé, pas seulement l'avatar.
    #[test]
    fn relay_fwd_payload_variable_aller_retour() {
        let dest = pid(9);
        let payload: Vec<u8> = (0..136u32).map(|i| i as u8).collect(); // 136 o ≠ 182
        let env = encode_relay_fwd(dest, &payload);
        let (recu_dest, recu_payload) = decode_relay_fwd(&env).expect("doit se décoder");
        assert_eq!(recu_dest, dest);
        assert_eq!(recu_payload, payload.as_slice()); // payload rendu à l'octet près
    }

    /// RELAY_FWD : une enveloppe malformée (mauvais type, en-tête sans payload) est rejetée nettement,
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
        // en-tête seul, aucun payload à porter → rejet (12.3-G : payload variable mais NON vide)
        assert!(decode_relay_fwd(&env[..RELAY_FWD_HEADER]).is_none());
        // un état signé normal (type 1) n'est pas une enveloppe (type 11) → rejet par le type
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
