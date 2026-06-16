//! RÉCEPTION : relever les paquets reçus et les RANGER dans la file du bon joueur.
//!
//! Ce système ne bouge JAMAIS l'avatar lui-même (c'est `interpolate` qui anime).
//! Il se contente d'empiler les instantanés, et de créer l'avatar 3D au premier
//! paquet d'un nouveau joueur.

use super::state::{RemoteAvatar, RemoteAvatars, RemoteHead, RemotePlayer, Snapshot, INTERP_DELAY};
use crate::net::link::NetLink;
use bevy::prelude::*;
use std::collections::VecDeque;

pub fn net_receive(
    link: Res<NetLink>,
    time: Res<Time>,
    mut avatars: ResMut<RemoteAvatars>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let now = time.elapsed_secs();
    for state in link.peer.poll() {
        if state.id == link.my_id {
            continue; // on ignore les paquets venant de soi-même
        }

        // L'instantané qu'on vient de recevoir, horodaté à MAINTENANT.
        let snap = Snapshot {
            t: now,
            pos: Vec3::new(state.x, state.y, state.z),
            vel: Vec3::new(state.vx, state.vy, state.vz),
            yaw: state.yaw,
            pitch: state.pitch,
        };

        if let Some(player) = avatars.map.get_mut(&state.id) {
            // Avatar déjà connu : on empile l'instantané (sans bouger l'avatar ;
            // c'est `net_interpolate` qui l'animera). On garde ~1 s d'historique.
            player.buffer.push_back(snap);
            while player.buffer.len() > 2 && now - player.buffer.front().unwrap().t > 1.0 {
                player.buffer.pop_front();
            }
        } else {
            // Premier paquet de ce joueur : on crée son avatar, de SA couleur.
            let torso = meshes.add(Capsule3d::new(0.17, 0.45));
            let head = meshes.add(Sphere::new(0.14));
            let limb = meshes.add(Capsule3d::new(0.07, 0.40));
            // Un petit « nez » (boîte plate) collé à l'avant de la tête : c'est lui
            // qui rend l'orientation lisible à distance.
            let nose = meshes.add(Cuboid::new(0.07, 0.05, 0.14));

            let skin = materials.add(body_skin(state.r, state.g, state.b));
            // Tête + nez un peu plus vifs pour bien ressortir.
            let skin_bright =
                materials.add(body_skin(state.r * 1.3, state.g * 1.3, state.b * 1.3));

            // On capture l'entité « tête » créée dans la fermeture des enfants.
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
                    // Pivot de la tête : porté par le corps, à hauteur du cou. C'est
                    // CETTE entité qu'on incline (tangage) ; elle contient la boule
                    // et le nez, qui tournent donc ensemble.
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

            let mut buffer = VecDeque::new();
            buffer.push_back(snap);
            // On démarre l'horloge de lecture déjà en retard de INTERP_DELAY :
            // on a ainsi tout de suite la bonne marge derrière le plus récent.
            avatars.map.insert(
                state.id,
                RemotePlayer {
                    body,
                    head: head_entity,
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
