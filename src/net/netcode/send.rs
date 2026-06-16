//! ENVOI : deux choses chaque image.
//!   1) un « battement de cœur » HELLO vers le rendez-vous (toutes les ~1 s),
//!      pour rester dans l'annuaire et recevoir la liste à jour des joueurs ;
//!   2) NOTRE état (position, vraie vitesse, orientation, couleur) à TOUS les
//!      pairs connus, à débit limité (SEND_HZ fois/s).

use super::state::SEND_HZ;
use crate::net::control::encode_hello;
use crate::net::link::NetLink;
use crate::net::message::{encode, PlayerState};
use bevy::prelude::*;

pub fn net_send(
    time: Res<Time>,
    mut send_acc: Local<f32>,
    mut hello_acc: Local<f32>,
    mut last_pos: Local<Option<Vec3>>,
    link: Res<NetLink>,
    player: Query<&Transform, With<crate::player::Player>>,
    camera: Query<&Transform, With<crate::player::PlayerCamera>>,
) {
    let dt = time.delta_secs();

    // 1) Battement de cœur vers le rendez-vous : « je suis toujours là ».
    *hello_acc += dt;
    if *hello_acc >= 1.0 {
        *hello_acc = 0.0;
        let _ = link.socket.send_to(link.rendezvous, &encode_hello());
    }

    // 2) Notre état vers tous les pairs (SEND_HZ/s). On accumule le temps et on
    //    n'envoie que quand l'intervalle est atteint.
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
    let Ok(transform) = player.single() else {
        return;
    };
    let pos = transform.translation;

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
    // Un même paquet, envoyé à CHAQUE pair (le P2P en étoile, depuis nous).
    for addr in link.peers.values() {
        let _ = link.socket.send_to(*addr, &bytes);
    }
}
