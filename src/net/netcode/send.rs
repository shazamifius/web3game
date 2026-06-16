//! ENVOI : transmettre NOTRE état (position, vitesse, orientation, couleur) à
//! l'autre joueur, à débit limité (SEND_HZ fois par seconde).

use super::state::SEND_HZ;
use crate::net::link::NetLink;
use crate::net::message::PlayerState;
use bevy::prelude::*;

/// Système : envoie notre état SEND_HZ fois/s. On calcule notre VRAIE vitesse ici
/// (variation de position / temps) : pas besoin de la deviner à la réception.
pub fn net_send(
    time: Res<Time>,
    mut accumulator: Local<f32>,
    mut last_pos: Local<Option<Vec3>>,
    link: Res<NetLink>,
    player: Query<&Transform, With<crate::player::Player>>,
    camera: Query<&Transform, With<crate::player::PlayerCamera>>,
) {
    // On n'envoie pas à chaque image (60/s) mais SEND_HZ fois/s : on accumule le
    // temps écoulé, et on n'envoie un paquet que quand l'intervalle est atteint.
    *accumulator += time.delta_secs();
    let interval = 1.0 / SEND_HZ;
    if *accumulator < interval {
        return;
    }
    let dt_send = *accumulator; // temps réellement écoulé depuis le dernier envoi
    *accumulator = 0.0;

    let Ok(transform) = player.single() else {
        return;
    };
    let pos = transform.translation;

    // VRAIE vitesse : variation de position depuis le dernier paquet, divisée par
    // le temps écoulé. Au tout premier envoi, on n'a pas de précédent → vitesse nulle.
    let velocity = match *last_pos {
        Some(prev) => (pos - prev) / dt_send,
        None => Vec3::ZERO,
    };
    *last_pos = Some(pos);

    // L'orientation gauche/droite vit sur le corps (lacet = rotation autour de Y).
    let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
    // L'inclinaison haut/bas vit sur la tête/caméra (tangage = rotation autour de X).
    let pitch = camera
        .single()
        .map(|cam| cam.rotation.to_euler(EulerRot::XYZ).0)
        .unwrap_or(0.0);

    let (r, g, b) = link.my_color;
    let me = PlayerState {
        id: link.my_id,
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
    let _ = link.peer.send(&me); // on ignore l'échec : le prochain paquet repart
}
