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
use crate::net::message::{decode_verified, PlayerState};
use crate::net::orb::{apply_incoming, decode_orb, Orb};
use crate::net::punch::{decode_punch, Holes};
use crate::net::wire::{kind, KIND_ORB, KIND_PUNCH, KIND_RELAY, KIND_STATE, KIND_WELCOME, PROTO_VERSION};
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
    mut warned_version: Local<bool>,
) {
    let now = time.elapsed_secs();

    // On vide la prise d'un coup (le Vec est à nous : on peut ensuite modifier `link`).
    let inbox = link.socket.poll();
    for (_from, bytes) in inbox {
        // GARDE DE VERSION : tout paquet a [type, version] en tête. S'il vient d'un
        // binaire d'une AUTRE version, on le rejette en bloc — et on le signale UNE
        // fois, pour rendre le piège visible (au lieu du « bonhomme invisible »
        // d'avant, où un binaire pas à jour lisait les octets de travers en silence).
        if bytes.len() >= 2 && bytes[1] != PROTO_VERSION {
            if !*warned_version {
                *warned_version = true;
                eprintln!(
                    "⚠ Paquet ignoré : version protocole {} ≠ la mienne ({}). Un binaire \
                     n'est pas à jour — ferme tout et relance depuis le même build.",
                    bytes[1], PROTO_VERSION
                );
            }
            continue;
        }
        match kind(&bytes) {
            // --- Réponse du rendez-vous : notre id + la liste des autres -------
            //     Le roster porte maintenant la CLÉ PUBLIQUE de chaque pair : on
            //     remplit deux annuaires, les adresses (pour émettre) et les clés
            //     (pour VÉRIFIER leurs signatures).
            Some(KIND_WELCOME) => {
                if let Some((your_id, world_hue, roster)) = decode_welcome(&bytes) {
                    link.my_id = Some(your_id);
                    link.world_hue = Some(world_hue);
                    link.peers = roster.iter().map(|(id, addr, _)| (*id, *addr)).collect();
                    link.pubkeys = roster.iter().map(|(id, _, pk)| (*id, *pk)).collect();
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
            //     VÉRIFIE le sceau de W, puis on RECOPIE l'enveloppe SCELLÉE telle
            //     quelle (octets verbatim), juste rebasculée en KIND_STATE. On ne
            //     ré-encode SURTOUT pas : ainsi on ne peut pas altérer son état (le
            //     sceau le prouve à ses voisins). Porteur d'octets, jamais faussaire.
            Some(KIND_RELAY) => {
                if let Some(state) = verify_packet(&bytes, &link) {
                    // 1) on RECOPIE l'enveloppe scellée à nos voisins (sauf l'auteur),
                    //    rebasculée en KIND_STATE (la forme que W a signée).
                    let mut forward = bytes.clone();
                    forward[0] = KIND_STATE;
                    for (id, addr) in &link.peers {
                        if *id != state.id {
                            let _ = link.socket.send_to(*addr, &forward);
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
            //     (Signature de l'orbe : chapitre 5.3, avec les Shields.) ----------
            Some(KIND_ORB) => {
                if let Some(w) = decode_orb(&bytes) {
                    apply_incoming(&mut orb, w, now);
                }
            }
            // --- État d'un pair : on VÉRIFIE son sceau, puis on le range. Un paquet
            //     non signé / forgé / dont on n'a pas encore la clé est ignoré. -----
            Some(KIND_STATE) => {
                if let Some(state) = verify_packet(&bytes, &link) {
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

/// Vérifie le sceau d'un paquet d'état (direct ou relayé) avec la clé publique de
/// l'émetteur revendiqué (son id est à l'octet 2). Renvoie `None` si :
///   - on n'a pas ENCORE sa clé (annuaire du rendez-vous en retard) → ignoré en silence ;
///   - le sceau ne colle PAS (forgé, altéré, ou mauvaise identité) → jeté.
/// C'est ICI que meurt l'usurpation d'identité : un paquet « id = 3 » qui n'est pas
/// signé par la clé privée de 3 est refusé — impossible de se faire passer pour autrui.
/// (Chapitre 5.4 : compter les sceaux invalides par pair = la graine de la réputation.)
fn verify_packet(bytes: &[u8], link: &NetLink) -> Option<PlayerState> {
    let claimed_id = *bytes.get(2)?;
    let pubkey = link.pubkeys.get(&claimed_id)?;
    decode_verified(bytes, pubkey)
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
