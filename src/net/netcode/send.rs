//! ENVOI : deux choses chaque image.
//!   1) un « battement de cœur » HELLO vers le rendez-vous (toutes les ~1 s),
//!      pour rester dans l'annuaire et recevoir la liste à jour des joueurs ;
//!   2) NOTRE état (position, vraie vitesse, orientation, couleur) à TOUS les
//!      pairs connus, à débit limité (SEND_HZ fois/s).

use super::state::{RemoteAvatars, SEND_HZ};
use crate::net::aoi::{allocate_tiers, dist2, relevance_weight, SEND_BUDGET_HZ};
use crate::net::control::encode_hello;
use crate::net::crypto::PeerId;
use crate::net::gossip::{encode_gossip, sample_cards};
use crate::net::link::NetLink;
use crate::net::message::{encode_relay_fwd, encode_signed, mark_as_relay, PlayerState};
use crate::net::punch::Holes;
use bevy::prelude::*;
use std::collections::HashMap;

/// Gossip (chap. 8.1) : période d'émission des cartes de visite et nombre de
/// destinataires par tic. Miroir des réglages du bot ([bot.rs]) — même protocole.
const GOSSIP_PERIOD: f32 = 0.5;
const GOSSIP_FANOUT: usize = 4;

/// REPLI RELAIS côté client (chap. 12.3 / D17), derrière DRAPEAU. Quand le perçage vers un pair est
/// ABANDONNÉ (NAT symétrique / box injoignable), au lieu de l'ignorer on route notre état via le
/// rendez-vous. ÉTEINT par défaut (`RELAY_FALLBACK` absent) → comportement historique exact (on
/// n'émet rien vers un trou fermé). Allumé sur `1`/`true`.
pub(crate) fn relay_fallback_enabled() -> bool {
    relay_fallback_on(std::env::var("RELAY_FALLBACK").ok().as_deref())
}

/// Politique du drapeau (PURE, testable sans toucher l'environnement). Défaut sûr = OFF.
fn relay_fallback_on(v: Option<&str>) -> bool {
    matches!(v, Some("1") | Some("true"))
}

pub fn net_send(
    time: Res<Time>,
    mut send_acc: Local<f32>,
    mut hello_acc: Local<f32>,
    mut last_pos: Local<Option<Vec3>>,
    mut credits: Local<HashMap<PeerId, f32>>,
    mut seq: Local<u64>, // compteur anti-rejeu : +1 à chaque état émis (chap. 5.2)
    mut gossip_acc: Local<f32>,
    mut gossip_cursor: Local<usize>,
    mut relay_fallback: Local<Option<bool>>, // 12.3 : drapeau lu UNE fois (cache)
    mut link: ResMut<NetLink>,
    avatars: Res<RemoteAvatars>,
    holes: Res<Holes>,
    player: Query<&Transform, With<crate::player::Player>>,
    camera: Query<&Transform, With<crate::player::PlayerCamera>>,
) {
    let dt = time.delta_secs();

    // On a besoin de notre position tout de suite (pour la case AoI du HELLO).
    let Ok(transform) = player.single() else {
        return;
    };
    let pos = transform.translation;

    // 1) Battement de cœur vers le rendez-vous : « je suis toujours là, dans
    //    cette case ». Le rendez-vous s'en sert pour ne nous donner que les voisins.
    *hello_acc += dt;
    if *hello_acc >= 1.0 {
        *hello_acc = 0.0;
        // Notre HELLO porte notre clé publique : le rendez-vous la redistribue pour
        // que chacun puisse vérifier nos signatures.
        let hello = encode_hello(pos.x, pos.z, link.identity.id());
        let _ = link.socket.send_to(link.rendezvous, &hello);
    }

    // 1bis) GOSSIP (chap. 8.1) : présenter un lot de cartes de visite (sous-ensemble
    //       DIVERS, curseur tournant) à quelques voisins au trou ouvert. C'est la
    //       découverte décentralisée qui lève le plafond de 32 (D22). Avant l'éventuel
    //       retour anticipé ci-dessous : le gossip a son propre rythme, indépendant de SEND_HZ.
    *gossip_acc += dt;
    if *gossip_acc >= GOSSIP_PERIOD {
        *gossip_acc = 0.0;
        if let Some(my_id) = link.my_id {
            let open: Vec<std::net::SocketAddr> = link
                .peers
                .iter()
                .filter(|(id, _)| holes.map.get(id).map_or(false, |h| h.open))
                .map(|(_, a)| *a)
                .take(GOSSIP_FANOUT)
                .collect();
            if !open.is_empty() {
                let cards = sample_cards(&link.peers, &link.peer_pos, my_id, *gossip_cursor);
                *gossip_cursor = gossip_cursor.wrapping_add(cards.len());
                if !cards.is_empty() {
                    let pkt = encode_gossip(&cards);
                    for addr in open {
                        let _ = link.socket.send_to(addr, &pkt);
                    }
                }
            }
        }
    }

    // 2) Notre état vers tous les pairs VOISINS (SEND_HZ/s). On accumule le temps
    //    et on n'envoie que quand l'intervalle est atteint.
    *send_acc += dt;
    let interval = 1.0 / SEND_HZ;
    if *send_acc < interval {
        return;
    }
    let dt_send = *send_acc; // temps réellement écoulé depuis le dernier envoi
    *send_acc = 0.0;

    // Tant que le rendez-vous ne nous a pas donné d'identifiant, on n'émet pas.
    let Some(my_id) = link.my_id else {
        return;
    };

    // VRAIE vitesse : variation de position depuis le dernier paquet / temps écoulé.
    let velocity = match *last_pos {
        Some(prev) => (pos - prev) / dt_send,
        None => Vec3::ZERO,
    };
    *last_pos = Some(pos);

    let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
    let pitch = camera
        .single()
        .map(|cam| cam.rotation.to_euler(EulerRot::XYZ).0)
        .unwrap_or(0.0);

    let (r, g, b) = link.my_color;

    // Si on est un client faible, on choisit notre PARENT (relais) : le plus petit
    // id joignable. On le met dans notre état (champ `parent`) pour que tout le monde
    // sache qu'on est sous tutelle, et de qui — c'est ce qui alimente les badges de
    // rôle. `parent = 0` = autonome (on émet à tout le monde nous-mêmes).
    let parent = if link.weak {
        link.peers
            .iter()
            .filter(|(id, _)| holes.map.get(id).map_or(false, |h| h.open))
            .min_by_key(|(id, _)| **id)
    } else {
        None
    };
    // L'identité (clé) de notre tuteur, ou `None` si on est autonome.
    let parent_id: Option<PeerId> = parent.map(|(id, _)| *id);

    // Un numéro de séquence STRICTEMENT croissant par paquet : le récepteur refusera
    // tout paquet de `seq` ≤ au dernier vu de notre part → un vieux paquet rejoué ne
    // peut plus nous rembobiner (anti-rejeu, chap. 5.2).
    *seq += 1;

    let me = PlayerState {
        id: my_id,
        x: pos.x,
        y: pos.y,
        z: pos.z,
        vx: velocity.x,
        vy: velocity.y,
        vz: velocity.z,
        yaw,
        pitch,
        r,
        g,
        b,
        parent: parent_id,
        seq: *seq,
    };
    // MODE FAIBLE UPLOAD : on n'émet PAS à tous les pairs. On envoie une seule fois
    // notre état (RELAY) au parent choisi plus haut, qui le recopiera à nos voisins
    // à notre place. Économie : 1 envoi au lieu de N. (Le download reste direct : on
    // continue de recevoir tout le monde, comme une vraie 4G.)
    if link.weak {
        if let Some((_, addr)) = parent {
            // On SCELLE notre état, puis on marque l'enveloppe « à recopier ». Le
            // parent ne peut que la porter : il ne peut pas en changer le contenu
            // sans casser notre sceau (cf. `encode_signed` / `mark_as_relay`).
            let mut sealed = encode_signed(&me, &link.identity);
            mark_as_relay(&mut sealed);
            let _ = link.socket.send_to(*addr, &sealed);
        }
        return;
    }

    // État SCELLÉ (signé) diffusé en direct à nos voisins.
    let bytes = encode_signed(&me, &link.identity);
    let me_xz = (pos.x, pos.z);

    // 0) FOCUS COLLANT (chap. 8.2a-bis) : on met à jour l'ensemble des pairs à plein débit AVANT
    //    d'allouer, avec hystérésis → on ne recompose pas le top-K à chaque image (fin du churn).
    link.refresh_focus(me_xz);

    // 1) PERTINENCE : un poids par pair, à partir de sa dernière position connue
    //    (lue dans sa file d'instantanés). Un pair inconnu → distance 0 → poids
    //    max, pour le découvrir vite.
    let peers: Vec<(PeerId, std::net::SocketAddr)> =
        link.peers.iter().map(|(id, addr)| (*id, *addr)).collect();
    let weights: Vec<f32> = peers
        .iter()
        .map(|(id, _)| {
            let d2 = avatars
                .map
                .get(id)
                .and_then(|p| p.buffer.back())
                .map(|s| dist2(me_xz, (s.pos.x, s.pos.z)))
                .unwrap_or(0.0);
            relevance_weight(d2)
        })
        .collect();

    // 2) AoI À DEUX TIERS (chap. 8.2 / 8.2a-bis) : le FOCUS COLLANT (link.focus) au plein débit,
    //    le reste en CONSCIENCE basse fidélité. Casse le « tout le monde flou » de la foule dense.
    let is_focus: Vec<bool> = peers.iter().map(|(id, _)| link.is_focus(id)).collect();
    let rates = allocate_tiers(&weights, &is_focus, SEND_BUDGET_HZ, SEND_HZ);

    // 3) CADENCEMENT par crédit : chaque pair accumule `débit × temps` ; dès qu'il
    //    atteint 1, on lui envoie un paquet et on retire 1. C'est ce qui espace
    //    régulièrement les envois au bon rythme pour chacun.
    // 12.3 : drapeau de repli relais, lu UNE fois (cache) — éteint par défaut → chemin intact.
    // On l'ANNONCE au démarrage : sans ça, impossible de savoir si l'env a bien pris dans le process.
    let relay = match *relay_fallback {
        Some(v) => v,
        None => {
            let on = relay_fallback_enabled();
            if on {
                println!("Repli relais NAT : ACTIF (RELAY_FALLBACK=1) — je relaierai via le rendez-vous si le perçage échoue.");
            } else {
                println!("Repli relais NAT : inactif — perçage direct seul. (Mets RELAY_FALLBACK=1 pour traverser les NAT symétriques.)");
            }
            *relay_fallback = Some(on);
            on
        }
    };
    for ((id, addr), rate) in peers.iter().zip(&rates) {
        // On ne diffuse l'état qu'aux pairs dont le trou NAT est OUVERT : sinon le
        // paquet mourrait dans leur box. Le perçage est fait par `net_punch` ; tant
        // que le trou n'est pas ouvert, on accumule juste un peu de crédit, prêt à
        // émettre dès que la connexion directe est établie.
        let open = holes.map.get(id).map_or(false, |h| h.open);
        // 12.3 — REPLI : on route via le rendez-vous (si le drapeau est allumé) dès que le pair le
        // VEUT — perçage abandonné OU pair qui nous joint déjà par relais (réciprocité immédiate, ferme
        // la fenêtre de reconnexion). Trou ni ouvert ni « wants_relay » = perçage en cours → on attend,
        // exactement comme avant (défaut byte-pour-byte intact quand `relay` est faux).
        let relayed = relay && holes.map.get(id).map_or(false, |h| h.wants_relay());
        if !open && !relayed {
            continue;
        }
        let credit = credits.entry(*id).or_insert(0.0);
        *credit += rate * dt_send;
        if *credit >= 1.0 {
            *credit -= 1.0;
            if open {
                let _ = link.socket.send_to(*addr, &bytes); // connexion directe (inchangé)
            } else {
                // Repli : on demande au rendez-vous de porter notre état SCELLÉ jusqu'à ce pair.
                let env = encode_relay_fwd(*id, &bytes);
                let _ = link.socket.send_to(link.rendezvous, &env);
            }
        }
    }
    // On oublie le crédit des pairs qui ne sont plus dans l'annuaire.
    credits.retain(|id, _| link.peers.contains_key(id));
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 12.3 — le repli relais est ÉTEINT par défaut (drapeau absent) et ne s'allume que sur `1`/`true`.
    /// Tant qu'il est éteint, `net_send` n'émet jamais vers un trou fermé (comportement historique).
    #[test]
    fn relay_fallback_eteint_par_defaut() {
        assert!(!relay_fallback_on(None)); // absent → OFF (chemin par défaut intact)
        assert!(!relay_fallback_on(Some("0")));
        assert!(!relay_fallback_on(Some("nope"))); // valeur inattendue → OFF (défaut sûr)
        assert!(relay_fallback_on(Some("1")));
        assert!(relay_fallback_on(Some("true")));
    }
}
