//! RÉCEPTION : vider la prise UDP et TRIER le courrier par son type.
//!
//!   - WELCOME (du rendez-vous) : on note notre identifiant et l'annuaire des pairs.
//!   - STATE   (d'un pair)       : on range l'instantané dans la file du bon joueur
//!                                 (et on crée son avatar 3D au premier paquet).
//!
//! Ce système ne bouge JAMAIS l'avatar lui-même (c'est `interpolate` qui anime).
//! Il fait aussi disparaître les avatars des joueurs sortis de l'annuaire.

use super::badges::{badge_mat, BadgeOwn, BadgeTutor, BadgeWard};
use super::state::{
    RemoteAvatar, RemoteAvatars, RemoteHead, RemotePlayer, Snapshot, INTERP_DELAY, REMOTE_TIMEOUT,
};
use crate::net::control::decode_welcome;
use crate::net::link::NetLink;
use crate::net::message::{decode, decode_relay, encode, PlayerState};
use crate::net::orb::{apply_incoming, decode_orb, Orb};
use crate::net::punch::{decode_punch, Holes};
use crate::net::wire::{kind, KIND_ORB, KIND_PUNCH, KIND_RELAY, KIND_STATE, KIND_WELCOME};
use bevy::prelude::*;
use std::collections::VecDeque;

pub fn net_receive(
    time: Res<Time>,
    mut link: ResMut<NetLink>,
    mut avatars: ResMut<RemoteAvatars>,
    mut holes: ResMut<Holes>,
    mut orb: ResMut<Orb>,
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
                if let Some((your_id, world_hue, roster)) = decode_welcome(&bytes) {
                    link.my_id = Some(your_id);
                    link.world_hue = Some(world_hue);
                    link.peers = roster.into_iter().collect();
                }
            }
            // --- PUNCH d'un pair : son paquet est ARRIVÉ, donc notre trou de
            //     retour est ouvert. On confirme la connexion directe. -----------
            Some(KIND_PUNCH) => {
                if let Some(id) = decode_punch(&bytes) {
                    let hole = holes.map.entry(id).or_default();
                    if !hole.open {
                        hole.open = true;
                        println!("Trou OUVERT avec le pair {id} ! Connexion directe établie.");
                    }
                }
            }
            // --- RELAY d'un joueur à faible upload (W) : on est son PARENT. On
            //     RECOPIE son état à tous nos pairs (sauf W lui-même), ré-émis en
            //     KIND_STATE. L'id reste celui de W → ses voisins le rangent sous
            //     SON avatar : on n'est qu'un porteur d'octets, pas une autorité. ---
            Some(KIND_RELAY) => {
                if let Some(state) = decode_relay(&bytes) {
                    // 1) on RECOPIE à nos voisins (sauf l'auteur) — rôle de porteur.
                    let relayed = encode(&state);
                    for (id, addr) in &link.peers {
                        if *id != state.id {
                            let _ = link.socket.send_to(*addr, &relayed);
                        }
                    }
                    // 2) on l'affiche AUSSI chez nous : le parent voit son protégé.
                    ingest_state(
                        state, now, link.my_id, &mut holes, &mut avatars, &mut commands,
                        &mut meshes, &mut materials,
                    );
                }
            }
            // --- État de l'orbe : seul le maître l'émet. On l'accepte si elle
            //     SUPPLANTE notre version (cf. règle d'autorité dans `orb`). -------
            Some(KIND_ORB) => {
                if let Some(w) = decode_orb(&bytes) {
                    apply_incoming(&mut orb, w, now);
                }
            }
            // --- État d'un pair : on le range pour l'interpolation -------------
            Some(KIND_STATE) => {
                if let Some(state) = decode(&bytes) {
                    ingest_state(
                        state, now, link.my_id, &mut holes, &mut avatars, &mut commands,
                        &mut meshes, &mut materials,
                    );
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

/// Range un état reçu (direct ou relayé) : marque le trou ouvert, empile
/// l'instantané dans la file du joueur, et crée son avatar au premier paquet.
/// Mutualisé entre `KIND_STATE` (état direct) et `KIND_RELAY` (état recopié par un
/// parent) pour que le traitement soit identique — y compris chez le parent, qui
/// doit voir son protégé comme n'importe quel autre joueur.
#[allow(clippy::too_many_arguments)]
fn ingest_state(
    state: PlayerState,
    now: f32,
    my_id: Option<u8>,
    holes: &mut Holes,
    avatars: &mut RemoteAvatars,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    if Some(state.id) == my_id {
        return; // jamais notre propre avatar
    }
    // Recevoir un état prouve aussi que le trou est ouvert (au cas où le PUNCH
    // serait passé inaperçu) : on le marque ouvert sans bruit.
    holes.map.entry(state.id).or_default().open = true;
    let snap = Snapshot {
        t: now,
        pos: Vec3::new(state.x, state.y, state.z),
        vel: Vec3::new(state.vx, state.vy, state.vz),
        yaw: state.yaw,
        pitch: state.pitch,
    };
    if let Some(player) = avatars.map.get_mut(&state.id) {
        // Avatar déjà connu : on empile l'instantané (~1 s d'historique).
        player.parent = state.parent; // rôle à jour (sous tutelle ? de qui ?)
        player.buffer.push_back(snap);
        while player.buffer.len() > 2 && now - player.buffer.front().unwrap().t > 1.0 {
            player.buffer.pop_front();
        }
    } else {
        // Premier paquet de ce joueur : on crée son avatar, de SA couleur.
        let parts = spawn_avatar(commands, meshes, materials, &state);
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
                parent: state.parent,
            },
        );
        println!("Nouveau joueur {} apparu.", state.id);
    }
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

    // Badges de rôle (cachés au départ ; `update_role_badges` les allume).
    // 🟠 orange = sous tutelle, 🟡 jaune = maître de l'orbe, 🟢 vert = tuteur.
    let id = state.id;
    let badge_diamond = meshes.add(Cuboid::new(0.13, 0.13, 0.13));
    let badge_ball = meshes.add(Sphere::new(0.09));
    let badge_roof = meshes.add(Cuboid::new(0.5, 0.05, 0.5));
    let mat_ward = materials.add(badge_mat(1.7, 0.7, 0.1));
    let mat_own = materials.add(badge_mat(1.6, 1.4, 0.2));
    let mat_tutor = materials.add(badge_mat(0.2, 1.6, 0.4));

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

            // Badges de rôle, empilés à des hauteurs distinctes (lisibles en cas de
            // cumul). Cachés ici ; allumés selon le rôle par `update_role_badges`.
            p.spawn((
                BadgeWard(id),
                Mesh3d(badge_diamond),
                MeshMaterial3d(mat_ward),
                Transform::from_xyz(0.0, 0.95, 0.0).with_rotation(Quat::from_rotation_y(0.785)),
                Visibility::Hidden,
            ));
            p.spawn((
                BadgeOwn(id),
                Mesh3d(badge_ball),
                MeshMaterial3d(mat_own),
                Transform::from_xyz(0.0, 1.15, 0.0),
                Visibility::Hidden,
            ));
            p.spawn((
                BadgeTutor(id),
                Mesh3d(badge_roof),
                MeshMaterial3d(mat_tutor),
                Transform::from_xyz(0.0, 1.35, 0.0),
                Visibility::Hidden,
            ));
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
