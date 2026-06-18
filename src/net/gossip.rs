//! LE GOSSIP : la « carte de visite » qui casse le plafond de 32 (chapitre 8.1, D22).
//!
//! # Pourquoi ce fichier existe
//! Jusqu'ici, la découverte des autres joueurs passait UNIQUEMENT par le rendez-vous,
//! qui ne présentait que les `MAX_NEIGHBORS = 32` plus proches ([rendezvous.rs]) — et
//! le client ÉCRASAIT sa table de pairs avec ces 32. Résultat : le 33e joueur n'existe
//! pas et ne peut JAMAIS être appris (D22 : en foule, aveugle au-delà de 32).
//!
//! Le gossip décentralise la découverte : chaque nœud annonce périodiquement, à
//! quelques voisins, un PETIT lot de « cartes de visite » d'AUTRES pairs qu'il connaît
//! (identité + adresse + dernière position connue). De proche en proche, chacun finit
//! par apprendre l'existence de toute la foule — SANS plafond dur, et SANS serveur
//! central qui énumère (le rendez-vous est démoté à l'amorçage). La table reste bornée
//! en MÉMOIRE (`MAX_KNOWN`, côté client), mais plus en VISION.
//!
//! # Anti-éclipse (amorce D9)
//! La carte n'est qu'un INDICE (« ce pair existerait là »). Elle ne prouve rien par
//! elle-même : pour vraiment « voir » le pair, il faudra recevoir SES paquets signés
//! (auto-certifiés). Un menteur peut donc inventer des cartes bidon, mais il ne peut
//! pas SIGNER à la place d'un autre → au pire il nous fait percer des trous dans le
//! vide (borné par `MAX_KNOWN` + le rate-limit). La diversité des informateurs (tirage
//! tournant côté émetteur) et la corroboration (8.8) durcissent ça plus tard.

use super::crypto::{PeerId, PUBKEY_LEN};
use super::wire::{KIND_GOSSIP, PROTO_VERSION};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

/// Nombre MAX de cartes par paquet de gossip. 16 × 46 o + 3 o d'en-tête = 739 o, bien
/// sous la taille d'un datagramme UDP sans fragmentation. Borne le coût d'un paquet.
pub(crate) const MAX_CARDS: usize = 16;

/// Taille d'une carte : id (clé, 32) + ip (4) + port (2) + x (4) + z (4) = 46 octets.
const CARD_SIZE: usize = PUBKEY_LEN + 4 + 2 + 4 + 4;

/// Une « carte de visite » : qui (`id`), où le joindre (`addr`), et sa dernière
/// position connue (`x, z`) — la position sert à l'AoI (pondérer sa pertinence avant
/// même d'avoir reçu un de ses états).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Card {
    pub(crate) id: PeerId,
    pub(crate) addr: SocketAddr,
    pub(crate) x: f32,
    pub(crate) z: f32,
}

/// Fabrique un paquet GOSSIP : type + version + nombre + [ carte… ]. Les cartes IPv6
/// sont ignorées (tout est en IPv4 pour l'instant, comme le WELCOME). On tronque à
/// `MAX_CARDS` : l'appelant choisit QUELLES cartes (un sous-ensemble divers).
pub(crate) fn encode_gossip(cards: &[Card]) -> Vec<u8> {
    let v4: Vec<(&Card, SocketAddrV4)> = cards
        .iter()
        .filter_map(|c| match c.addr {
            SocketAddr::V4(a) => Some((c, a)),
            SocketAddr::V6(_) => None,
        })
        .take(MAX_CARDS)
        .collect();

    let mut buf = vec![KIND_GOSSIP, PROTO_VERSION, v4.len() as u8];
    for (c, a) in v4 {
        buf.extend_from_slice(c.id.bytes()); // 32
        buf.extend_from_slice(&a.ip().octets()); // 4
        buf.extend_from_slice(&a.port().to_le_bytes()); // 2
        buf.extend_from_slice(&c.x.to_le_bytes()); // 4
        buf.extend_from_slice(&c.z.to_le_bytes()); // 4
    }
    buf
}

/// Lit un paquet GOSSIP : renvoie les cartes qu'il porte. Tronqué/corrompu → on
/// s'arrête à la dernière carte complète, sans planter (comme `decode_welcome`).
/// Une position non finie (NaN/Inf) fait rejeter la carte (jamais de poison numérique).
pub(crate) fn decode_gossip(buf: &[u8]) -> Option<Vec<Card>> {
    if buf.len() < 3 || buf[0] != KIND_GOSSIP || buf[1] != PROTO_VERSION {
        return None;
    }
    let count = buf[2] as usize;
    let mut cards = Vec::with_capacity(count.min(MAX_CARDS));
    let mut o = 3;
    for _ in 0..count {
        if o + CARD_SIZE > buf.len() {
            break; // tronqué : on garde ce qu'on a lu
        }
        let mut pk = [0u8; PUBKEY_LEN];
        pk.copy_from_slice(&buf[o..o + PUBKEY_LEN]);
        let p = o + PUBKEY_LEN;
        let ip = Ipv4Addr::new(buf[p], buf[p + 1], buf[p + 2], buf[p + 3]);
        let port = u16::from_le_bytes([buf[p + 4], buf[p + 5]]);
        let x = f32::from_le_bytes([buf[p + 6], buf[p + 7], buf[p + 8], buf[p + 9]]);
        let z = f32::from_le_bytes([buf[p + 10], buf[p + 11], buf[p + 12], buf[p + 13]]);
        o += CARD_SIZE;
        if !x.is_finite() || !z.is_finite() {
            continue; // carte empoisonnée : ignorée (on ne jette pas tout le paquet)
        }
        let addr = SocketAddr::from(SocketAddrV4::new(ip, port));
        cards.push(Card { id: PeerId::from_bytes(pk), addr, x, z });
    }
    Some(cards)
}

/// Choisit un sous-ensemble DIVERS (≤ `MAX_CARDS`) de pairs connus à présenter en
/// gossip (chap. 8.1). On parcourt les pairs dans un ordre STABLE (tri par identité)
/// à partir d'un `cursor` qui TOURNE d'un appel à l'autre : au fil du temps on présente
/// TOUS les pairs, pas toujours les mêmes — c'est ça la diversité des informateurs
/// (amorce anti-éclipse, D9). `exclude` (généralement le destinataire) n'est jamais
/// inclus : inutile de présenter un pair à lui-même.
pub(crate) fn sample_cards(
    peers: &HashMap<PeerId, SocketAddr>,
    pos: &HashMap<PeerId, (f32, f32)>,
    exclude: PeerId,
    cursor: usize,
) -> Vec<Card> {
    // Ordre stable : sans tri, l'itération d'une HashMap est arbitraire → le curseur
    // ne couvrirait pas l'ensemble de façon fiable.
    let mut ids: Vec<PeerId> = peers.keys().copied().filter(|id| *id != exclude).collect();
    if ids.is_empty() {
        return Vec::new();
    }
    ids.sort_unstable_by(|a, b| a.bytes().cmp(b.bytes()));
    let n = ids.len();
    let start = cursor % n;
    (0..n.min(MAX_CARDS))
        .map(|k| {
            let id = ids[(start + k) % n];
            let (x, z) = pos.get(&id).copied().unwrap_or((0.0, 0.0));
            Card { id, addr: peers[&id], x, z }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(seed: u8, port: u16, x: f32, z: f32) -> Card {
        Card {
            id: PeerId::from_bytes([seed; PUBKEY_LEN]),
            addr: SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port)),
            x,
            z,
        }
    }

    /// Aller-retour : un lot de cartes encodé puis décodé revient identique.
    #[test]
    fn gossip_survit_a_l_aller_retour() {
        let cards = vec![card(1, 5001, 1.0, -2.0), card(2, 5002, 3.5, 4.0)];
        let d = decode_gossip(&encode_gossip(&cards)).expect("doit se décoder");
        assert_eq!(d, cards);
    }

    /// On ne dépasse JAMAIS `MAX_CARDS` cartes par paquet (borne de coût).
    #[test]
    fn gossip_tronque_a_max_cards() {
        let many: Vec<Card> = (0..40).map(|i| card(i as u8, 6000 + i, 0.0, 0.0)).collect();
        let d = decode_gossip(&encode_gossip(&many)).expect("doit se décoder");
        assert_eq!(d.len(), MAX_CARDS);
    }

    /// Un paquet tronqué en plein milieu ne plante pas : on garde les cartes complètes.
    #[test]
    fn gossip_tronque_ne_plante_pas() {
        let cards = vec![card(1, 5001, 1.0, 2.0), card(2, 5002, 3.0, 4.0)];
        let mut bytes = encode_gossip(&cards);
        bytes.truncate(bytes.len() - 5); // coupe la 2e carte en deux
        let d = decode_gossip(&bytes).expect("doit se décoder");
        assert_eq!(d, vec![cards[0]]); // seule la 1re carte, complète, survit
    }

    /// Une carte à position NaN/Inf est ignorée (jamais de flottant non fini admis).
    #[test]
    fn gossip_rejette_position_non_finie() {
        let cards = vec![card(1, 5001, f32::NAN, 0.0), card(2, 5002, 1.0, 2.0)];
        let d = decode_gossip(&encode_gossip(&cards)).expect("doit se décoder");
        assert_eq!(d, vec![cards[1]]); // la NaN sautée, la saine gardée
    }

    /// Le curseur qui TOURNE finit par présenter TOUS les pairs (diversité des
    /// informateurs) : sur plus de pairs que `MAX_CARDS`, quelques rounds couvrent tout.
    #[test]
    fn sample_cards_tourne_et_couvre_tout() {
        let mut peers = HashMap::new();
        let pos = HashMap::new();
        let total = MAX_CARDS * 3; // bien plus que ce qu'un paquet porte
        for i in 0..total {
            peers.insert(
                PeerId::from_bytes([i as u8; PUBKEY_LEN]),
                SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 7000 + i as u16)),
            );
        }
        let me = PeerId::from_bytes([200; PUBKEY_LEN]); // pas dans la table
        let mut seen = std::collections::HashSet::new();
        for round in 0..8 {
            for c in sample_cards(&peers, &pos, me, round * MAX_CARDS) {
                seen.insert(c.id);
            }
        }
        assert_eq!(seen.len(), total); // tout le monde a été présenté au moins une fois
    }

    /// On ne se présente jamais soi-même (le destinataire exclu n'apparaît pas).
    #[test]
    fn sample_cards_exclut_le_destinataire() {
        let mut peers = HashMap::new();
        let pos = HashMap::new();
        let dest = PeerId::from_bytes([1; PUBKEY_LEN]);
        peers.insert(dest, SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 7001)));
        peers.insert(
            PeerId::from_bytes([2; PUBKEY_LEN]),
            SocketAddr::from(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 7002)),
        );
        let cards = sample_cards(&peers, &pos, dest, 0);
        assert!(cards.iter().all(|c| c.id != dest));
    }
}
