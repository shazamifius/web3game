//! ENVOI : deux choses chaque image.
//!   1) un « battement de cœur » HELLO vers le rendez-vous (toutes les ~1 s),
//!      pour rester dans l'annuaire et recevoir la liste à jour des joueurs ;
//!   2) NOTRE état (position, vraie vitesse, orientation, couleur) à TOUS les
//!      pairs connus, à débit limité (SEND_HZ fois/s).

use super::state::{RemoteAvatars, SEND_HZ};
use crate::net::aoi::{allocate_rates, dist2, relevance_weight, SEND_BUDGET_HZ};
use crate::net::control::encode_hello;
use crate::net::link::NetLink;
use crate::net::message::{encode, encode_relay, PlayerState};
use crate::net::punch::Holes;
use bevy::prelude::*;
use std::collections::HashMap;

pub fn net_send(
    time: Res<Time>,
    mut send_acc: Local<f32>,
    mut hello_acc: Local<f32>,
    mut last_pos: Local<Option<Vec3>>,
    mut credits: Local<HashMap<u8, f32>>,
    link: Res<NetLink>,
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
        let _ = link.socket.send_to(link.rendezvous, &encode_hello(pos.x, pos.z));
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
    let parent_id = parent.map(|(id, _)| *id).unwrap_or(0);

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
    };
    // MODE FAIBLE UPLOAD : on n'émet PAS à tous les pairs. On envoie une seule fois
    // notre état (RELAY) au parent choisi plus haut, qui le recopiera à nos voisins
    // à notre place. Économie : 1 envoi au lieu de N. (Le download reste direct : on
    // continue de recevoir tout le monde, comme une vraie 4G.)
    if link.weak {
        if let Some((_, addr)) = parent {
            let _ = link.socket.send_to(*addr, &encode_relay(&me));
        }
        return;
    }

    let bytes = encode(&me);
    let me_xz = (pos.x, pos.z);

    // 1) PERTINENCE : un poids par pair, à partir de sa dernière position connue
    //    (lue dans sa file d'instantanés). Un pair inconnu → distance 0 → poids
    //    max, pour le découvrir vite.
    let peers: Vec<(u8, std::net::SocketAddr)> =
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

    // 2) WATER-FILLING : un débit (Hz) par pair, plafonné à SEND_HZ, somme ≤ budget.
    //    Budget non saturé (peu de monde) → tout le monde au plafond. Saturé → ça
    //    se répartit par pertinence, en douceur, jamais zéro.
    let rates = allocate_rates(&weights, SEND_BUDGET_HZ, SEND_HZ);

    // 3) CADENCEMENT par crédit : chaque pair accumule `débit × temps` ; dès qu'il
    //    atteint 1, on lui envoie un paquet et on retire 1. C'est ce qui espace
    //    régulièrement les envois au bon rythme pour chacun.
    for ((id, addr), rate) in peers.iter().zip(&rates) {
        // On ne diffuse l'état qu'aux pairs dont le trou NAT est OUVERT : sinon le
        // paquet mourrait dans leur box. Le perçage est fait par `net_punch` ; tant
        // que le trou n'est pas ouvert, on accumule juste un peu de crédit, prêt à
        // émettre dès que la connexion directe est établie.
        if !holes.map.get(id).map_or(false, |h| h.open) {
            continue;
        }
        let credit = credits.entry(*id).or_insert(0.0);
        *credit += rate * dt_send;
        if *credit >= 1.0 {
            *credit -= 1.0;
            let _ = link.socket.send_to(*addr, &bytes);
        }
    }
    // On oublie le crédit des pairs qui ne sont plus dans l'annuaire.
    credits.retain(|id, _| link.peers.contains_key(id));
}
