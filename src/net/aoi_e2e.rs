//! BANC AoI BOUT-EN-BOUT — la PERTINENCE TRANSITIVE passe-t-elle VRAIMENT par le RÉSEAU ? (D29, étapes 1+2)
//!
//! Le banc `jeu aoi` prouve la LOGIQUE (sélection + allocation) en local. Les tests unitaires prouvent chaque
//! PIÈCE isolément (le wire `KIND_ENGAGED`, `refresh_focus`). Mais on n'avait pas encore prouvé la CHAÎNE COMPLÈTE :
//! un nœud déclare un partenaire → le message VOYAGE sur le réseau → un AUTRE nœud le reçoit, VÉRIFIE le sceau, et
//! son focus change pour de vrai. C'est ce que fait ce banc, sur le **bus mémoire** (transport réel `Socket`,
//! déterministe), de bout en bout : `encode_engaged` (signé) → `Socket::send_to` → `Socket::poll` → `decode_engaged`
//! (vérif) → `note_engaged` → `refresh_focus`. Rien n'est injecté à la main du côté de la pertinence : B l'APPREND
//! du paquet de F.
//!
//! Scénario (le même que `jeu aoi`, mais la pertinence TRANSITE par le réseau) : B (observateur) à l'origine, son
//! ami PROCHE F (1 m), un partenaire LOIN P (40 m), au milieu d'une foule dense (anneau 9–11 m). Sous la seule
//! distance, P tombe en conscience (flou) ; quand F ÉMET « je suis engagé avec P » et que B le reçoit, B garde P
//! au plein débit. ⚠ BUS_DOUTE — le bus est un réseau PARFAIT : ce banc prouve la MÉCANIQUE bout-en-bout, pas les
//! conditions réelles (pour ça : `sim`/`tc netem`).

use super::aoi::{allocate_tiers, dist2, relevance_weight, SEND_BUDGET_HZ};
use super::crypto::{Identity, PeerId};
use super::link::NetLink;
use super::message::{decode_engaged, encode_engaged};
use super::transport::{new_bus, Socket};
use super::wire::{kind, KIND_ENGAGED};
use std::net::SocketAddr;

const SEND_HZ: f32 = 20.0; // débit plein (focus) = la cadence d'émission

fn addr(port: u16) -> SocketAddr {
    SocketAddr::from(([127, 0, 0, 1], port))
}

/// Un id synthétique non nul et distinct (le partenaire et la foule ne sont pas des nœuds vivants —
/// juste des entrées de table chez B). `b[2]=1` → jamais l'identité nulle.
fn pid(i: usize) -> PeerId {
    let mut b = [0u8; 32];
    b[0] = (i & 0xff) as u8;
    b[1] = ((i >> 8) & 0xff) as u8;
    b[2] = 1;
    PeerId::from_bytes(b)
}

/// Remplit la table d'un observateur : ami PROCHE (1 m), partenaire LOIN (40 m), foule dense (9–11 m).
fn garnir_table(b: &mut NetLink, friend: PeerId, partner: PeerId, n_foule: usize) {
    b.peers.insert(friend, addr(9001));
    b.peer_pos.insert(friend, (1.0, 0.0));
    b.peers.insert(partner, addr(9002));
    b.peer_pos.insert(partner, (40.0, 0.0));
    for c in 0..n_foule {
        let id = pid(1000 + c);
        b.peers.insert(id, addr(9100 + c as u16));
        let a = c as f32 * 2.399_963_2; // angle d'or → répartition déterministe
        let r = 9.0 + (c % 3) as f32; // 9, 10, 11 m
        b.peer_pos.insert(id, (r * a.cos(), r * a.sin()));
    }
}

/// Débit (Hz) que B livrerait à `target`, vu son focus courant (allocation par tiers, comme en prod).
fn hz_to(link: &NetLink, me: (f32, f32), target: &PeerId) -> f32 {
    let ids: Vec<PeerId> = link.peers.keys().copied().collect();
    let weights: Vec<f32> = ids
        .iter()
        .map(|id| link.peer_pos.get(id).map(|&p| relevance_weight(dist2(me, p))).unwrap_or(0.0))
        .collect();
    let foc: Vec<bool> = ids.iter().map(|id| link.is_focus(id)).collect();
    let rates = allocate_tiers(&weights, &foc, SEND_BUDGET_HZ, SEND_HZ);
    ids.iter().position(|id| id == target).map(|i| rates[i]).unwrap_or(0.0)
}

/// Mesure BOUT-EN-BOUT pour une foule de taille `n_foule`. Renvoie :
/// (P au focus en proximité, Hz livré à P en proximité, P au focus en pertinence, Hz en pertinence,
///  taille du paquet `KIND_ENGAGED` qui a transité). « Proximité » = F n'émet rien ; « pertinence » =
/// F émet sa déclaration sur le bus, B la reçoit/vérifie/consomme.
fn mesurer_e2e(n_foule: usize) -> (bool, f32, bool, f32, usize) {
    let me = (0.0, 0.0);
    let bus = new_bus();

    // F = ami PROCHE et ÉMETTEUR de la déclaration d'engagement.
    let mut f = NetLink::new_on_with_identity(Socket::bus(addr(7001), bus.clone()), addr(7000), (1.0, 1.0, 1.0), Identity::generate_pow(0));
    let f_id = f.identity.id();
    let partner = pid(200);

    // PHASE PROXIMITÉ — F NE déclare RIEN : aucun paquet n'est émis, B ne voit que la distance.
    let mut b_prox = NetLink::new_on_with_identity(Socket::bus(addr(7002), bus.clone()), addr(7000), (1.0, 1.0, 1.0), Identity::generate_pow(0));
    garnir_table(&mut b_prox, f_id, partner, n_foule);
    b_prox.refresh_focus(me);
    let prox_focus = b_prox.is_focus(&partner);
    let prox_hz = hz_to(&b_prox, me, &partner);

    // PHASE PERTINENCE — F déclare P et l'ÉMET sur le réseau ; B le REÇOIT, le VÉRIFIE et le consomme.
    let mut b_pert = NetLink::new_on_with_identity(Socket::bus(addr(7003), bus.clone()), addr(7000), (1.0, 1.0, 1.0), Identity::generate_pow(0));
    garnir_table(&mut b_pert, f_id, partner, n_foule);

    // F : exactement comme `bot.step` — `set_engaged` puis `encode_engaged` signé → `send_to` chaque pair.
    f.set_engaged(vec![partner]);
    let pkt = encode_engaged(&f.identity, &f.my_engaged);
    let pkt_len = pkt.len();
    f.socket.send_to(addr(7003), &pkt).unwrap(); // → la boîte mémoire de B

    // B : exactement comme le dispatch de réception de `bot.rs` — `kind` → `decode_engaged` (vérif sceau)
    //     → `note_engaged`. Le paquet a réellement transité par le transport `Socket`.
    for (_from, bytes) in b_pert.socket.poll() {
        if kind(&bytes) == Some(KIND_ENGAGED) {
            if let Some((id, partners)) = decode_engaged(&bytes) {
                b_pert.note_engaged(id, partners);
            }
        }
    }
    b_pert.refresh_focus(me);
    let pert_focus = b_pert.is_focus(&partner);
    let pert_hz = hz_to(&b_pert, me, &partner);

    (prox_focus, prox_hz, pert_focus, pert_hz, pkt_len)
}

fn oui_non(b: bool) -> &'static str {
    if b {
        "OUI"
    } else {
        "non"
    }
}

/// Point d'entrée `jeu aoi-live`.
pub fn run_aoi_e2e() {
    println!("🔌  BANC AoI BOUT-EN-BOUT — la pertinence transitive PASSE-T-ELLE par le RÉSEAU ? (D29, étapes 1+2)");
    println!("    F (ami proche, 1 m) déclare 1 partenaire LOIN (40 m) et l'ÉMET en KIND_ENGAGED SIGNÉ sur le bus ;");
    println!("    B (observateur) le REÇOIT, VÉRIFIE le sceau, et recalcule son focus. Foule dense (anneau 9–11 m).\n");
    println!("   {:>9} │ {:^23} │ {:^27}", "foule N", "P (partenaire) au FOCUS", "débit livré à P");
    println!("   {:>9} │ {:>11} {:>11} │ {:>12} {:>13}", "", "proximité", "PERTINENCE", "proximité", "PERTINENCE");
    let mut pkt_len = 0;
    for &n in &[10usize, 30, 60, 120] {
        let (pf, ph, qf, qh, pl) = mesurer_e2e(n);
        pkt_len = pl;
        println!(
            "   {:>9} │ {:>11} {:>11} │ {:>9.1} Hz {:>10.1} Hz",
            n,
            oui_non(pf),
            oui_non(qf),
            ph,
            qh
        );
    }
    println!("\n📌 Lecture : SANS paquet réseau, B ne voit que la distance → P (loin) tombe en CONSCIENCE (débit qui");
    println!("   s'effondre quand la foule grossit). Le paquet KIND_ENGAGED de F ({} o, signé) ARRIVE, est VÉRIFIÉ,", pkt_len);
    println!("   et B garde P au PLEIN débit, quelle que soit la foule. La pertinence n'est plus un calcul LOCAL :");
    println!("   elle a VOYAGÉ sur le réseau et changé le focus du RÉCEPTEUR. (Bus = réseau parfait → prouve la");
    println!("   MÉCANIQUE bout-en-bout, pas les conditions réelles — cf. BUS_DOUTE / `sim`.)");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// D29 BOUT-EN-BOUT : la déclaration de F voyage sur le bus, B la consomme, et son focus garde le
    /// partenaire LOIN au plein débit — alors que SANS le paquet, B le laisse tomber en conscience.
    #[test]
    fn la_pertinence_voyage_sur_le_reseau_et_change_le_focus() {
        for n in [10usize, 60, 120] {
            let (prox_focus, prox_hz, pert_focus, pert_hz, pkt_len) = mesurer_e2e(n);
            assert!(!prox_focus, "sans engagement reçu, P (loin) n'est PAS au focus (N={})", n);
            assert!(pert_focus, "après réception du KIND_ENGAGED, P est REHAUSSÉ au focus (N={})", n);
            assert!(pert_hz > 10.0, "P au plein débit après réception (N={}) : {} Hz", n, pert_hz);
            assert!(
                pert_hz > prox_hz * 3.0,
                "la pertinence REÇUE livre ≫ la proximité (N={}) : {} vs {}",
                n,
                pert_hz,
                prox_hz
            );
            assert!(pkt_len > 0 && pkt_len < 200, "le paquet engaged est petit (N={}) : {} o", n, pkt_len);
        }
    }

    /// Le paquet est bien REÇU et VÉRIFIÉ : après le `poll`, B connaît l'engagement signé de F (et un
    /// sceau invalide aurait été rejeté par `decode_engaged` → rien stocké).
    #[test]
    fn le_paquet_engaged_est_recu_et_verifie() {
        let bus = new_bus();
        let mut f = NetLink::new_on_with_identity(Socket::bus(addr(7011), bus.clone()), addr(7000), (1.0, 1.0, 1.0), Identity::generate_pow(0));
        let f_id = f.identity.id();
        let partner = pid(200);
        let mut b = NetLink::new_on_with_identity(Socket::bus(addr(7012), bus.clone()), addr(7000), (1.0, 1.0, 1.0), Identity::generate_pow(0));
        b.peers.insert(f_id, addr(7011));
        b.peer_pos.insert(f_id, (1.0, 0.0));
        b.peers.insert(partner, addr(9002));
        b.peer_pos.insert(partner, (40.0, 0.0));

        f.set_engaged(vec![partner]);
        let pkt = encode_engaged(&f.identity, &f.my_engaged);
        f.socket.send_to(addr(7012), &pkt).unwrap();
        for (_from, bytes) in b.socket.poll() {
            if kind(&bytes) == Some(KIND_ENGAGED) {
                if let Some((id, partners)) = decode_engaged(&bytes) {
                    b.note_engaged(id, partners);
                }
            }
        }
        assert_eq!(b.peer_engaged.get(&f_id), Some(&vec![partner]), "B a reçu et stocké l'engagement signé de F");
    }
}
