//! SONDE DE LIEN — partie 1 : le TYPE de NAT (cône PERÇABLE vs symétrique/CGNAT).
//!
//! # Pourquoi (Phase 2 du PLAN_TEST_RESEAU, doute D36)
//! La Phase 1 a tranché : la redondance relais ne sert QUE sur de la perte *aléatoire avec
//! marge*, jamais sur un lien saturé. Pour décider ça machine par machine, chaque agent doit
//! d'abord CONNAÎTRE la nature de son lien. Le premier trait — le plus structurant — c'est :
//! mon NAT est-il **perçable** (cône) ou non (**symétrique / CGNAT**, relais obligatoire) ?
//!
//! # La méthode (RFC 5389, dep-free)
//! On envoie une « binding request » STUN à DEUX serveurs publics d'IP DIFFÉRENTE, depuis la
//! MÊME prise UDP, et on lit l'adresse PUBLIQUE qu'ils nous renvoient (« reflexive address ») :
//!   - même port public vu par les deux → le NAT garde un mapping STABLE quel que soit le
//!     destinataire (« endpoint-independent ») = **cône** → perçable par hole-punching ;
//!   - port public DIFFÉRENT → le NAT refait un mapping par destinataire (« endpoint-dependent »)
//!     = **symétrique** (typique du CGNAT 4G) → le perçage direct échoue → **relais obligatoire**.
//! Un NAT « ouvert » (aucune traduction) se comporte comme un cône (perçable trivialement) → on
//! le replie honnêtement dans `Cone`. Aucune réponse (UDP bloqué, hors-ligne) → `Unknown`.
//!
//! # Découpage testable
//! L'encodage de la requête, le décodage de l'adresse réfléchie et la CLASSIFICATION sont des
//! fonctions PURES (zéro réseau) → couvertes par des tests déterministes. Seul `probe_nat` /
//! `run_natcheck` touchent le réseau (un aller-retour au démarrage, hors boucle chaude).

use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::Duration;

/// Le « magic cookie » STUN (RFC 5389) : constante fixe en tête de chaque message (octets 4..8).
/// Sert AUSSI de masque XOR pour l'adresse réfléchie (attribut XOR-MAPPED-ADDRESS).
const MAGIC_COOKIE: u32 = 0x2112_A442;

/// Type de message STUN : « binding request » — ce qu'on ÉMET vers le serveur.
const MSG_BINDING_REQUEST: u16 = 0x0001;
/// Type de message STUN : « binding success response » — ce qu'on ATTEND en retour.
const MSG_BINDING_SUCCESS: u16 = 0x0101;

/// Attribut moderne (RFC 5389) : adresse réfléchie MASQUÉE (XOR avec le magic cookie). Préféré.
const ATTR_XOR_MAPPED_ADDRESS: u16 = 0x0020;
/// Attribut historique (RFC 3489) : adresse réfléchie NUE. Repli si pas de XOR-MAPPED-ADDRESS.
const ATTR_MAPPED_ADDRESS: u16 = 0x0001;

/// Famille d'adresse IPv4 dans un attribut d'adresse STUN.
const FAMILY_IPV4: u8 = 0x01;

/// Serveurs STUN publics et bien connus, de FOURNISSEURS DIFFÉRENTS (donc d'IP différentes) →
/// indispensable pour distinguer cône de symétrique (un symétrique remappe par IP de destination).
/// On les essaie dans l'ordre jusqu'à obtenir DEUX réponses depuis deux IP distinctes. Ce ne sont
/// pas des secrets, juste des adresses publiques — l'agent ne porte toujours QUE des adresses, jamais de clé.
const STUN_SERVERS: &[&str] = &[
    "stun.l.google.com:19302",
    "stun.cloudflare.com:3478",
    "stun.nextcloud.com:443",
    "stun.stunprotocol.org:3478",
];

/// Le verdict de NAT. Volontairement à TROIS états honnêtes : on sait dire perçable, non-perçable,
/// ou « je n'ai pas pu trancher ». (« Open NAT » = perçable trivialement → replié dans `Cone`.)
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NatType {
    /// Mapping endpoint-indépendant (même port public vers 2 serveurs) → PERÇABLE par hole-punching.
    Cone,
    /// Mapping endpoint-dépendant (le port public change) → CGNAT/symétrique → RELAIS obligatoire.
    Symmetric,
    /// Pas de réponse STUN (UDP bloqué, hors-ligne, ou une seule observation) → indéterminé.
    Unknown,
}

/// Étiquette courte et stable pour le heartbeat / l'affichage (`nat:"cone"` etc.).
pub(crate) fn nat_str(t: NatType) -> &'static str {
    match t {
        NatType::Cone => "cone",
        NatType::Symmetric => "sym",
        NatType::Unknown => "?",
    }
}

/// Fabrique une « binding request » STUN : en-tête de 20 octets, aucun attribut. PUR (testable).
/// `txid` = identifiant de transaction (12 octets) qu'on retrouvera dans la réponse pour l'apparier.
pub(crate) fn encode_binding_request(txid: [u8; 12]) -> [u8; 20] {
    let mut b = [0u8; 20];
    b[0..2].copy_from_slice(&MSG_BINDING_REQUEST.to_be_bytes());
    // octets 2..4 = longueur des attributs = 0 (déjà à zéro).
    b[4..8].copy_from_slice(&MAGIC_COOKIE.to_be_bytes());
    b[8..20].copy_from_slice(&txid);
    b
}

/// Décode l'adresse PUBLIQUE réfléchie d'une réponse STUN. PUR (testable sans réseau). Rejette tout
/// paquet qui n'est pas une réponse de succès, dont le magic cookie ou l'identifiant de transaction
/// ne correspondent pas (anti-paquet-étranger). Cherche XOR-MAPPED-ADDRESS, sinon MAPPED-ADDRESS.
/// IPv4 uniquement (notre cas P2P) → une réponse purement IPv6 donne `None`.
pub(crate) fn decode_mapped_address(resp: &[u8], txid: [u8; 12]) -> Option<SocketAddr> {
    if resp.len() < 20 {
        return None;
    }
    if u16::from_be_bytes([resp[0], resp[1]]) != MSG_BINDING_SUCCESS {
        return None;
    }
    if u32::from_be_bytes([resp[4], resp[5], resp[6], resp[7]]) != MAGIC_COOKIE {
        return None;
    }
    if resp[8..20] != txid {
        return None;
    }
    let attr_len = u16::from_be_bytes([resp[2], resp[3]]) as usize;
    let end = (20 + attr_len).min(resp.len());
    let mut i = 20;
    // On peut rencontrer les deux attributs : on PRÉFÈRE la version XOR (renvoyée immédiatement),
    // en gardant la version nue comme repli si seule elle est présente.
    let mut fallback: Option<SocketAddr> = None;
    while i + 4 <= end {
        let atype = u16::from_be_bytes([resp[i], resp[i + 1]]);
        let alen = u16::from_be_bytes([resp[i + 2], resp[i + 3]]) as usize;
        let vstart = i + 4;
        let vend = vstart + alen;
        if vend > resp.len() {
            break;
        }
        let val = &resp[vstart..vend];
        match atype {
            ATTR_XOR_MAPPED_ADDRESS => {
                if let Some(a) = parse_addr(val, true) {
                    return Some(a);
                }
            }
            ATTR_MAPPED_ADDRESS => {
                if fallback.is_none() {
                    fallback = parse_addr(val, false);
                }
            }
            _ => {}
        }
        // Chaque attribut est aligné sur 4 octets (padding).
        i = vstart + ((alen + 3) & !3);
    }
    fallback
}

/// Lit un attribut d'adresse STUN IPv4 : `[réservé(1)][famille(1)][port(2)][adresse(4)]`. Si `xor`,
/// dé-masque port et adresse avec le magic cookie (XOR-MAPPED-ADDRESS). PUR.
fn parse_addr(val: &[u8], xor: bool) -> Option<SocketAddr> {
    if val.len() < 8 || val[1] != FAMILY_IPV4 {
        return None; // tronqué ou IPv6 → on ne gère pas
    }
    let mut port = u16::from_be_bytes([val[2], val[3]]);
    let mut octets = [val[4], val[5], val[6], val[7]];
    if xor {
        port ^= (MAGIC_COOKIE >> 16) as u16; // les 16 bits de poids fort du cookie
        let cookie = MAGIC_COOKIE.to_be_bytes();
        for k in 0..4 {
            octets[k] ^= cookie[k];
        }
    }
    Some(SocketAddr::from((Ipv4Addr::from(octets), port)))
}

/// Classe le NAT à partir des DEUX adresses réfléchies (une par serveur). PUR (testable). Même
/// adresse publique (IP + port) vue par les deux → cône (perçable) ; port différent → symétrique ;
/// moins de deux observations → indéterminé.
pub(crate) fn classify_nat(a: Option<SocketAddr>, b: Option<SocketAddr>) -> NatType {
    match (a, b) {
        (Some(x), Some(y)) => {
            if x.ip() == y.ip() && x.port() == y.port() {
                NatType::Cone
            } else {
                NatType::Symmetric
            }
        }
        _ => NatType::Unknown,
    }
}

/// Un identifiant de transaction « assez unique » pour apparier requête/réponse. Pas besoin de
/// crypto-aléa ici (c'est juste un tag d'appariement) : horloge nanoseconde + adresse pile (ASLR).
fn random_txid() -> [u8; 12] {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let mut t = [0u8; 12];
    t[0..8].copy_from_slice(&nanos.to_be_bytes());
    let salt = (&nanos as *const u64 as u64).to_be_bytes();
    t[8..12].copy_from_slice(&salt[0..4]);
    t
}

/// Interroge UN serveur STUN et renvoie l'adresse publique réfléchie. Best-effort : 3 tentatives
/// (UDP peut perdre), on ignore les paquets venus d'ailleurs. `None` = pas de réponse exploitable.
fn stun_query(socket: &UdpSocket, server: SocketAddr) -> Option<SocketAddr> {
    let txid = random_txid();
    let req = encode_binding_request(txid);
    let mut buf = [0u8; 512];
    for _ in 0..3 {
        if socket.send_to(&req, server).is_err() {
            return None;
        }
        match socket.recv_from(&mut buf) {
            Ok((n, from)) if from.ip() == server.ip() => {
                if let Some(addr) = decode_mapped_address(&buf[..n], txid) {
                    return Some(addr);
                }
            }
            Ok(_) => continue,  // paquet d'un autre expéditeur → on retente
            Err(_) => continue, // timeout → on retente
        }
    }
    None
}

/// LA SONDE NAT : interroge des serveurs STUN jusqu'à 2 réponses d'IP distinctes, puis classe.
/// Renvoie le verdict + les observations (serveur → adresse réfléchie) pour l'affichage/diagnostic.
/// Touche le réseau (au démarrage), borné par un timeout court → ne bloque jamais durablement.
pub(crate) fn probe_nat() -> (NatType, Vec<(IpAddr, SocketAddr)>) {
    let Ok(socket) = UdpSocket::bind(("0.0.0.0", 0)) else {
        return (NatType::Unknown, Vec::new());
    };
    let _ = socket.set_read_timeout(Some(Duration::from_millis(700)));
    let mut obs: Vec<(IpAddr, SocketAddr)> = Vec::new();
    for host in STUN_SERVERS {
        let Ok(addrs) = host.to_socket_addrs() else {
            continue;
        };
        let Some(server) = addrs.into_iter().find(|a| a.is_ipv4()) else {
            continue;
        };
        if obs.iter().any(|(ip, _)| *ip == server.ip()) {
            continue; // déjà une observation depuis cette IP → on en veut une AUTRE
        }
        if let Some(reflexive) = stun_query(&socket, server) {
            obs.push((server.ip(), reflexive));
            if obs.len() >= 2 {
                break;
            }
        }
    }
    let a = obs.first().map(|(_, r)| *r);
    let b = obs.get(1).map(|(_, r)| *r);
    (classify_nat(a, b), obs)
}

/// `jeu natcheck` : sonde le NAT de CETTE machine et imprime un verdict clair (perçable ou non).
/// Outil de diagnostic à la main, jumeau de `jeu phase1` : zéro popup, pour comprendre son lien.
pub fn run_natcheck() {
    println!("[natcheck] Sonde STUN du type de NAT (cône perçable vs symétrique/CGNAT)…");
    let (nat, obs) = probe_nat();
    if obs.is_empty() {
        println!("[natcheck] Aucune réponse STUN — UDP bloqué, hors-ligne, ou serveurs injoignables.");
    } else {
        for (i, (server, reflexive)) in obs.iter().enumerate() {
            println!("[natcheck]   serveur {} ({server}) → adresse publique vue : {reflexive}", i + 1);
        }
    }
    let verdict = match nat {
        NatType::Cone => "CÔNE → perçable par hole-punching (connexion directe possible)",
        NatType::Symmetric => "SYMÉTRIQUE/CGNAT → perçage direct impossible → RELAIS obligatoire",
        NatType::Unknown => "INDÉTERMINÉ → besoin d'au moins 2 réponses STUN d'IP distinctes",
    };
    println!("[natcheck] Verdict : nat={} — {verdict}", nat_str(nat));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Une réponse STUN forgée à la main, avec un attribut au choix, pour tester le décodage SANS réseau.
    /// `attr_type` + valeur d'adresse IPv4 (`[réservé, famille, port_hi, port_lo, a, b, c, d]`).
    fn forge_response(txid: [u8; 12], attr_type: u16, addr_val: &[u8]) -> Vec<u8> {
        let mut r = Vec::new();
        r.extend_from_slice(&MSG_BINDING_SUCCESS.to_be_bytes());
        let attr_total = 4 + addr_val.len(); // en-tête d'attribut (4) + valeur
        r.extend_from_slice(&(attr_total as u16).to_be_bytes());
        r.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
        r.extend_from_slice(&txid);
        r.extend_from_slice(&attr_type.to_be_bytes());
        r.extend_from_slice(&(addr_val.len() as u16).to_be_bytes());
        r.extend_from_slice(addr_val);
        r
    }

    /// La requête : 20 octets, type binding-request, longueur 0, magic cookie, et notre txid intact.
    #[test]
    fn requete_binding_bien_formee() {
        let txid = [7u8; 12];
        let req = encode_binding_request(txid);
        assert_eq!(req.len(), 20);
        assert_eq!(u16::from_be_bytes([req[0], req[1]]), MSG_BINDING_REQUEST);
        assert_eq!(u16::from_be_bytes([req[2], req[3]]), 0); // aucun attribut
        assert_eq!(u32::from_be_bytes([req[4], req[5], req[6], req[7]]), MAGIC_COOKIE);
        assert_eq!(&req[8..20], &txid);
    }

    /// XOR-MAPPED-ADDRESS : on masque une adresse connue, on vérifie que le décodage la retrouve.
    #[test]
    fn decode_xor_mapped_address() {
        let txid = [1u8; 12];
        // On veut décoder 203.0.113.5:50000 → on FABRIQUE la valeur masquée (XOR cookie).
        let ip = [203u8, 0, 113, 5];
        let port: u16 = 50000;
        let cookie = MAGIC_COOKIE.to_be_bytes();
        let xport = port ^ (MAGIC_COOKIE >> 16) as u16;
        let xip = [ip[0] ^ cookie[0], ip[1] ^ cookie[1], ip[2] ^ cookie[2], ip[3] ^ cookie[3]];
        let val = [0x00, FAMILY_IPV4, (xport >> 8) as u8, xport as u8, xip[0], xip[1], xip[2], xip[3]];
        let resp = forge_response(txid, ATTR_XOR_MAPPED_ADDRESS, &val);
        assert_eq!(
            decode_mapped_address(&resp, txid),
            Some(SocketAddr::from((Ipv4Addr::new(203, 0, 113, 5), 50000)))
        );
    }

    /// MAPPED-ADDRESS (nue, sans XOR) : repli quand le serveur n'envoie que la vieille version.
    #[test]
    fn decode_mapped_address_nue() {
        let txid = [2u8; 12];
        let port: u16 = 4000;
        let val = [0x00, FAMILY_IPV4, (port >> 8) as u8, port as u8, 88, 167, 242, 251];
        let resp = forge_response(txid, ATTR_MAPPED_ADDRESS, &val);
        assert_eq!(
            decode_mapped_address(&resp, txid),
            Some(SocketAddr::from((Ipv4Addr::new(88, 167, 242, 251), 4000)))
        );
    }

    /// Un paquet dont le txid ne correspond PAS est rejeté (anti-paquet-étranger), idem mauvais cookie.
    #[test]
    fn decode_rejette_les_imposteurs() {
        let txid = [3u8; 12];
        let val = [0x00, FAMILY_IPV4, 0x0f, 0xa0, 1, 2, 3, 4];
        let bon = forge_response(txid, ATTR_MAPPED_ADDRESS, &val);
        // Mauvais txid attendu → None.
        assert_eq!(decode_mapped_address(&bon, [9u8; 12]), None);
        // Mauvais cookie → None.
        let mut cookie_casse = bon.clone();
        cookie_casse[4] ^= 0xff;
        assert_eq!(decode_mapped_address(&cookie_casse, txid), None);
        // Trop court → None.
        assert_eq!(decode_mapped_address(&bon[..10], txid), None);
    }

    /// La classification : même adresse publique vue par 2 serveurs = cône ; port différent =
    /// symétrique ; moins de 2 observations = indéterminé.
    #[test]
    fn classification_cone_sym_unknown() {
        let pub1 = SocketAddr::from((Ipv4Addr::new(81, 2, 3, 4), 40000));
        let pub_meme = SocketAddr::from((Ipv4Addr::new(81, 2, 3, 4), 40000));
        let pub_autre_port = SocketAddr::from((Ipv4Addr::new(81, 2, 3, 4), 51234));
        assert_eq!(classify_nat(Some(pub1), Some(pub_meme)), NatType::Cone);
        assert_eq!(classify_nat(Some(pub1), Some(pub_autre_port)), NatType::Symmetric);
        assert_eq!(classify_nat(Some(pub1), None), NatType::Unknown);
        assert_eq!(classify_nat(None, None), NatType::Unknown);
        assert_eq!(nat_str(NatType::Cone), "cone");
        assert_eq!(nat_str(NatType::Symmetric), "sym");
        assert_eq!(nat_str(NatType::Unknown), "?");
    }
}
