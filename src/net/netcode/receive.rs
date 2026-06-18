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
use crate::net::message::{decode_canonical, sig_ok, PlayerState};
use crate::net::orb::{apply_incoming, decode_orb, orb_sig_ok, Orb, OrbApply, OrbWire};
use crate::net::punch::{decode_punch, Holes};
use crate::net::wire::{kind, KIND_ORB, KIND_PUNCH, KIND_RELAY, KIND_STATE, KIND_WELCOME, PROTO_VERSION};
use bevy::prelude::*;
use std::collections::VecDeque;

/// Jetons rechargés par seconde et par adresse (rate-limit). On attend ~45 paquets/s
/// d'un pair honnête (état 20 Hz + orbe 20 Hz + extras) : 150 laisse de la marge.
const BUCKET_RATE: f32 = 150.0;
/// Réserve maximale de jetons (tolère une courte rafale sans pénaliser un pair honnête).
const BUCKET_CAP: f32 = 300.0;
/// Plafond d'avatars distants affichés (anti-DoS : un attaquant ne peut pas nous
/// faire créer 255 avatars en variant l'id). Large pour un voisinage AoI normal.
const MAX_AVATARS: usize = 64;

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
    mut buckets: Local<std::collections::HashMap<std::net::SocketAddr, f32>>,
    mut flood_warned: Local<bool>,
) {
    let now = time.elapsed_secs();
    let dt = time.delta_secs();

    // RATE-LIMIT (chap. 5.5) : on recharge le « seau à jetons » de chaque adresse
    // (BUCKET_RATE jetons/s, plafonné à BUCKET_CAP). Chaque paquet reçu coûte 1 jeton.
    // Une adresse qui inonde épuise son seau → ses paquets en trop sont jetés, AVANT
    // même d'être analysés : un attaquant ne peut plus saturer le CPU à 10 000 Hz.
    for credit in buckets.values_mut() {
        *credit = (*credit + dt * BUCKET_RATE).min(BUCKET_CAP);
    }

    // On vide la prise d'un coup (le Vec est à nous : on peut ensuite modifier `link`).
    let inbox = link.socket.poll();
    for (from, bytes) in inbox {
        // Débit : ce paquet a-t-il un jeton ? Sinon, c'est une inondation → on jette.
        let credit = buckets.entry(from).or_insert(BUCKET_CAP);
        if *credit < 1.0 {
            if !*flood_warned {
                *flood_warned = true;
                eprintln!("🛡 Inondation détectée depuis {from} : paquets en excès ignorés.");
            }
            continue;
        }
        *credit -= 1.0;

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
            Some(KIND_RELAY) => match check_packet(&bytes, &link) {
                Checked::Good(state) => {
                    // muet, ou rejeu (seq périmé) → on jette en silence. Pas de strike
                    // sur le rejeu : un tiers pourrait rejouer un vieux paquet VALIDE de
                    // la victime → ce serait, là encore, l'accuser à tort.
                    if !link.is_muted(state.id) && link.accept_seq(state.id, state.seq) {
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
                Checked::Faulty(id) => link.add_strike(id, "relais : état signé impossible (NaN)"),
                Checked::Unknown => {}
            },
            // --- État de l'orbe : seul le maître l'émet, et il le SIGNE (chap. 5.3).
            //     On vérifie le sceau avec la clé du maître revendiqué, puis on
            //     applique si ça SUPPLANTE notre version (autorité) ET si le saut de
            //     version reste plausible (anti-vol / anti-gel à version=65535). -----
            Some(KIND_ORB) => match check_orb(&bytes, &link) {
                OrbChecked::Good(w) => {
                    let owner = w.owner;
                    if !link.is_muted(owner) {
                        // L'autorité + la borne de version vivent dans `apply_incoming` ;
                        // un saut aberrant (vol/gel) en revient comme une faute.
                        if let OrbApply::Implausible = apply_incoming(&mut orb, w, now) {
                            link.add_strike(owner, "orbe : saut de version aberrant");
                        }
                    }
                }
                OrbChecked::Faulty(id) => link.add_strike(id, "orbe : état signé impossible (NaN)"),
                OrbChecked::Unknown => {}
            },
            // --- État d'un pair : sceau + anti-rejeu + réputation, puis on le range.
            Some(KIND_STATE) => match check_packet(&bytes, &link) {
                Checked::Good(state) => {
                    // muet ou rejeu → on jette en silence (pas de strike sur le rejeu :
                    // un tiers pourrait rejouer un paquet valide de la victime → framing).
                    if !link.is_muted(state.id) && link.accept_seq(state.id, state.seq) {
                        ingest_state(
                            state, now, link.my_id, &mut holes, &mut avatars, &mut commands,
                            &mut meshes, &mut materials,
                        );
                    }
                }
                Checked::Faulty(id) => link.add_strike(id, "état signé impossible (NaN)"),
                Checked::Unknown => {}
            },
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

/// Résultat de la vérification d'un paquet d'état. On sépare TROIS cas, et c'est
/// volontaire pour la réputation :
///   - `Good`    : sceau valide + contenu correct → on traite ;
///   - `Faulty`  : sceau VALIDE mais contenu impossible (ex. NaN) → faute ATTRIBUABLE
///                 (seul le détenteur de la clé a pu signer ça) → on inscrit un strike ;
///   - `Unknown` : sceau invalide, ou clé pas encore connue → on JETTE sans accuser.
/// Ce dernier point est capital : frapper l'id revendiqué sur un sceau invalide
/// permettrait à un attaquant de FAIRE ACCUSER sa victime (forger « id=victime »
/// pourri pour la faire bannir). On n'accuse donc JAMAIS sur une signature invalide.
enum Checked {
    Good(PlayerState),
    Faulty(u8),
    Unknown,
}

/// Vérifie le sceau d'un paquet d'état (direct ou relayé). L'id revendiqué est à
/// l'octet 2. C'est ICI que meurt l'usurpation : un paquet « id = 3 » qui n'est pas
/// signé par la clé privée de 3 a un sceau invalide → jeté (sans accuser le vrai 3).
fn check_packet(bytes: &[u8], link: &NetLink) -> Checked {
    let Some(&claimed_id) = bytes.get(2) else {
        return Checked::Unknown;
    };
    let Some(pubkey) = link.pubkeys.get(&claimed_id) else {
        return Checked::Unknown; // annuaire en retard : on n'a pas (encore) sa clé
    };
    if !sig_ok(bytes, pubkey) {
        return Checked::Unknown; // sceau invalide → non attribuable → on jette sans accuser
    }
    match decode_canonical(bytes) {
        Some(state) => Checked::Good(state),
        None => Checked::Faulty(claimed_id), // signé MAIS contenu impossible → faute
    }
}

/// Pendant pour l'orbe : l'id du maître revendiqué est aussi à l'octet 2.
enum OrbChecked {
    Good(OrbWire),
    Faulty(u8),
    Unknown,
}

fn check_orb(bytes: &[u8], link: &NetLink) -> OrbChecked {
    let Some(&owner) = bytes.get(2) else {
        return OrbChecked::Unknown;
    };
    let Some(pubkey) = link.pubkeys.get(&owner) else {
        return OrbChecked::Unknown;
    };
    if !orb_sig_ok(bytes, pubkey) {
        return OrbChecked::Unknown; // sceau invalide → jeté sans accuser
    }
    match decode_orb(bytes) {
        Some(w) => OrbChecked::Good(w),
        None => OrbChecked::Faulty(owner),
    }
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
        // Plafond anti-DoS : on refuse de créer un avatar de plus au-delà de la
        // limite (un attaquant ne peut pas nous noyer sous 255 avatars en variant
        // l'id). Les avatars déjà connus continuent d'être mis à jour normalement.
        if avatars.map.len() >= MAX_AVATARS {
            return;
        }
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
