//! RÉCEPTION : vider la prise UDP et TRIER le courrier par son type.
//!
//!   - WELCOME (du rendez-vous) : on note notre identifiant et l'annuaire des pairs.
//!   - STATE   (d'un pair)       : on range l'instantané dans la file du bon joueur
//!                                 (et on crée son avatar 3D au premier paquet).
//!
//! Ce système ne bouge JAMAIS l'avatar lui-même (c'est `interpolate` qui anime).
//! Il fait aussi disparaître les avatars des joueurs sortis de l'annuaire.

use super::state::{
    RemoteAvatar, RemoteAvatars, RemoteHead, RemotePlayer, Snapshot, INTERP_DELAY, REMOTE_TIMEOUT,
};
use crate::net::control::decode_welcome;
use crate::net::link::NetLink;
use crate::net::message::decode;
use crate::net::wire::{kind, KIND_STATE, KIND_WELCOME};
use bevy::prelude::*;
use std::collections::VecDeque;

pub fn net_receive(
    time: Res<Time>,
    mut link: ResMut<NetLink>,
    mut avatars: ResMut<RemoteAvatars>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed_secs();

    // On vide la prise d'un coup (le Vec est à nous : on peut ensuite modifier `link`).
    let inbox = link.socket.poll();
    for (_from, bytes) in inbox {
        match kind(&bytes) {
            // --- Réponse du rendez-vous : notre id + la liste des autres -------
            Some(KIND_WELCOME) => {
                if let Some((your_id, roster)) = decode_welcome(&bytes) {
                    link.my_id = Some(your_id);
                    link.peers = roster.into_iter().collect();
                }
            }
            // --- État d'un pair : on le range pour l'interpolation -------------
            Some(KIND_STATE) => {
                let Some(state) = decode(&bytes) else { continue };
                if Some(state.id) == link.my_id {
                    continue; // jamais notre propre avatar
                }
                let snap = Snapshot {
                    t: now,
                    pos: Vec3::new(state.x, state.y, state.z),
                    vel: Vec3::new(state.vx, state.vy, state.vz),
                    yaw: state.yaw,
                    pitch: state.pitch,
                };
                if let Some(player) = avatars.map.get_mut(&state.id) {
                    // Avatar déjà connu : on empile l'instantané (~1 s d'historique).
                    player.buffer.push_back(snap);
                    while player.buffer.len() > 2 && now - player.buffer.front().unwrap().t > 1.0 {
                        player.buffer.pop_front();
                    }
                } else {
                    // Premier paquet de ce joueur : on crée son avatar, de SA couleur.
                    let parts = spawn_avatar(&mut commands, &mut meshes, &mut materials, &state);
                    let mut buffer = VecDeque::new();
                    buffer.push_back(snap);
                    avatars.map.insert(
                        state.id,
                        RemotePlayer {
                            body: parts.0,
                            head: parts.1,
                            buffer,
                            clock: now - INTERP_DELAY,
                            smooth_vel: Vec3::ZERO,
                            yaw_vel: 0.0,
                            pitch_vel: 0.0,
                        },
                    );
                    println!("Nouveau joueur {} apparu.", state.id);
                }
            }
            _ => {}
        }
    }

    // On retire l'avatar d'un joueur dont on n'a plus reçu d'état depuis un moment
    // (il est parti, ou injoignable). On se base sur l'âge du dernier instantané,
    // PAS sur l'annuaire : un pair peut nous envoyer son état avant que l'annuaire
    // ne nous l'ait listé — sinon l'avatar clignoterait (créé/supprimé en boucle).
    avatars.map.retain(|id, player| {
        let last_seen = player.buffer.back().map(|s| s.t).unwrap_or(now);
        let keep = now - last_seen < REMOTE_TIMEOUT;
        if !keep {
            commands.entity(player.body).despawn();
            println!("Joueur {id} parti.");
        }
        keep
    });
}

/// Crée l'avatar 3D d'un joueur distant (corps articulé + tête à nez directionnel).
/// Renvoie (entité du corps, entité du pivot de tête).
fn spawn_avatar(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    state: &crate::net::message::PlayerState,
) -> (Entity, Entity) {
    let torso = meshes.add(Capsule3d::new(0.17, 0.45));
    let head = meshes.add(Sphere::new(0.14));
    let limb = meshes.add(Capsule3d::new(0.07, 0.40));
    // Un petit « nez » (boîte plate) collé à l'avant de la tête : il rend
    // l'orientation lisible à distance.
    let nose = meshes.add(Cuboid::new(0.07, 0.05, 0.14));

    let skin = materials.add(body_skin(state.r, state.g, state.b));
    let skin_bright = materials.add(body_skin(state.r * 1.3, state.g * 1.3, state.b * 1.3));

    let mut head_entity = Entity::PLACEHOLDER;

    let body = commands
        .spawn((
            RemoteAvatar { id: state.id },
            Transform::from_xyz(state.x, state.y, state.z)
                .with_rotation(Quat::from_rotation_y(state.yaw)),
            Visibility::default(),
        ))
        .with_children(|p| {
            // Torse
            p.spawn((
                Mesh3d(torso),
                MeshMaterial3d(skin.clone()),
                Transform::from_xyz(0.0, 0.10, 0.0),
            ));
            // Bras (gauche / droit)
            for x in [-0.30, 0.30] {
                p.spawn((
                    Mesh3d(limb.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::from_xyz(x, 0.08, 0.0),
                ));
            }
            // Jambes (gauche / droite)
            for x in [-0.11, 0.11] {
                p.spawn((
                    Mesh3d(limb.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::from_xyz(x, -0.45, 0.0),
                ));
            }
            // Pivot de la tête (porte le tangage) : boule + nez.
            head_entity = p
                .spawn((
                    RemoteHead,
                    Transform::from_xyz(0.0, 0.62, 0.0),
                    Visibility::default(),
                ))
                .with_children(|h| {
                    h.spawn((
                        Mesh3d(head),
                        MeshMaterial3d(skin_bright.clone()),
                        Transform::default(),
                    ));
                    // Le nez pointe vers l'avant (−Z = « devant » dans Bevy).
                    h.spawn((
                        Mesh3d(nose),
                        MeshMaterial3d(skin_bright.clone()),
                        Transform::from_xyz(0.0, 0.0, -0.14),
                    ));
                })
                .id();
        })
        .id();

    (body, head_entity)
}

/// Matériau de skin émissif (glow néon) pour les avatars distants.
fn body_skin(r: f32, g: f32, b: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(0.02, 0.02, 0.03),
        emissive: LinearRgba::rgb(r, g, b),
        perceptual_roughness: 0.5,
        ..default()
    }
}
