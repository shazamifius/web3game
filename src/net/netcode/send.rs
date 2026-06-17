//! ENVOI : deux choses chaque image.
//!   1) un « battement de cœur » HELLO vers le rendez-vous (toutes les ~1 s),
//!      pour rester dans l'annuaire et recevoir la liste à jour des joueurs ;
//!   2) NOTRE état (position, vraie vitesse, orientation, couleur) à TOUS les
//!      pairs connus, à débit limité (SEND_HZ fois/s).

use super::state::{RemoteAvatars, SEND_HZ};
use crate::net::aoi::{FULL_BUDGET, REDUCE_FACTOR};
use crate::net::control::encode_hello;
use crate::net::link::NetLink;
use crate::net::message::{encode, PlayerState};
use bevy::prelude::*;

pub fn net_send(
    time: Res<Time>,
    mut send_acc: Local<f32>,
    mut hello_acc: Local<f32>,
    mut last_pos: Local<Option<Vec3>>,
    mut tick: Local<u32>,
    link: Res<NetLink>,
    avatars: Res<RemoteAvatars>,
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
    };
    let bytes = encode(&me);

    // BUDGET DE PRIORITÉ : on classe les pairs par distance (on lit leur dernière
    // position connue dans leur file d'instantanés), puis on donne le plein débit
    // aux FULL_BUDGET plus proches, et un débit réduit (1 paquet sur REDUCE_FACTOR)
    // aux plus lointains. Une foule ne coûte donc jamais plus qu'un budget fixe.
    let me_xz = (pos.x, pos.z);
    let mut ranked: Vec<(_, f32)> = link
        .peers
        .iter()
        .map(|(id, addr)| {
            let d2 = avatars
                .map
                .get(id)
                .and_then(|p| p.buffer.back())
                .map(|s| crate::net::aoi::dist2(me_xz, (s.pos.x, s.pos.z)))
                .unwrap_or(0.0); // pair encore inconnu → priorité haute, pour le découvrir
            (*addr, d2)
        })
        .collect();
    ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

    *tick = tick.wrapping_add(1);
    for (rank, (addr, _)) in ranked.iter().enumerate() {
        let full = rank < FULL_BUDGET;
        if full || *tick % REDUCE_FACTOR == 0 {
            let _ = link.socket.send_to(*addr, &bytes);
        }
    }
}
