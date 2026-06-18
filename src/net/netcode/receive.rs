//! RÉCEPTION : vider la prise UDP et TRIER le courrier par son type.
//!
//!   - WELCOME (du rendez-vous) : on note l'annuaire des pairs (et notre identité).
//!   - STATE   (d'un pair)       : on range l'instantané dans la file du bon joueur
//!                                 (et on crée son avatar 3D au premier paquet).
//!
//! # Identité auto-certifiante (chap. 6.1)
//! On ne consulte plus d'annuaire de clés : chaque paquet PORTE la clé publique de
//! son émetteur, et `sig_ok` vérifie le sceau CONTRE cette clé embarquée. L'identité
//! s'auto-prouve ; le rendez-vous ne peut plus mentir sur « qui est qui ».
//!
//! Ce système ne bouge JAMAIS l'avatar lui-même (c'est `interpolate` qui anime).

use super::state::{
    RemoteAvatar, RemoteAvatars, RemoteHead, RemotePlayer, Snapshot, INTERP_DELAY, REMOTE_TIMEOUT,
};
use crate::net::accuse::decode_accuse;
use crate::net::anticheat::move_plausible;
use crate::net::control::decode_welcome;
use crate::net::crypto::{PeerId, POW_BITS};
use crate::net::link::NetLink;
use crate::net::message::{claimed_id, decode_canonical, sig_ok, PlayerState};
use crate::net::orb::{apply_incoming, claimed_owner, decode_orb, orb_sig_ok, Orb, OrbApply, OrbWire};
use crate::net::punch::{decode_punch, Holes};
use crate::net::wire::{
    kind, KIND_ACCUSE, KIND_ORB, KIND_PUNCH, KIND_RELAY, KIND_STATE, KIND_WELCOME, PROTO_VERSION,
};
use bevy::prelude::*;
use std::collections::VecDeque;

/// Jetons rechargés par seconde et par adresse (rate-limit). On attend ~45 paquets/s
/// d'un pair honnête (état 20 Hz + orbe 20 Hz + extras) : 150 laisse de la marge.
const BUCKET_RATE: f32 = 150.0;
/// Réserve maximale de jetons (tolère une courte rafale sans pénaliser un pair honnête).
const BUCKET_CAP: f32 = 300.0;
/// Plafond d'avatars distants affichés (anti-DoS : un attaquant ne peut pas nous
/// faire créer trop d'avatars). Large pour un voisinage AoI normal.
const MAX_AVATARS: usize = 64;
/// Plafond du nombre de « seaux » d'adresses suivis (chap. 6.5). Sans borne, un
/// attaquant qui USURPE des milliers d'adresses sources nous ferait créer autant
/// d'entrées → mémoire saturée. Au-delà, on jette les seaux PLEINS (= adresses
/// inactives, qui ont rechargé à fond) pour faire de la place.
const MAX_BUCKETS: usize = 4096;
/// Relais (chap. 6.5) : paquets RELAY qu'on accepte de recopier par protégé et par
/// seconde, et réserve max. Petit exprès : un relais ne dédie qu'une fraction bornée
/// de son upload à amplifier autrui → fin de l'amplification réfléchie illimitée.
const RELAY_RATE: f32 = 30.0;
const RELAY_CAP: f32 = 60.0;
/// Nombre max de voisins à qui un relais ré-émet UN paquet (chap. 6.5). Borne le
/// facteur d'amplification : 1 paquet entrant → au plus `MAX_RELAY_FANOUT` sortants.
const MAX_RELAY_FANOUT: usize = 12;

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
    mut relay_credits: Local<std::collections::HashMap<PeerId, f32>>,
    mut flood_warned: Local<bool>,
) {
    let now = time.elapsed_secs();
    let dt = time.delta_secs();

    // RATE-LIMIT (chap. 5.5) : on recharge le « seau à jetons » de chaque adresse,
    // et (chap. 6.5) le budget de relais de chaque protégé.
    for credit in buckets.values_mut() {
        *credit = (*credit + dt * BUCKET_RATE).min(BUCKET_CAP);
    }
    for credit in relay_credits.values_mut() {
        *credit = (*credit + dt * RELAY_RATE).min(RELAY_CAP);
    }
    // ÉVICTION (chap. 6.5) : si on suit trop d'adresses (usurpation de sources), on
    // jette les seaux pleins — ce sont des adresses inactives → mémoire bornée.
    if buckets.len() > MAX_BUCKETS {
        buckets.retain(|_, c| *c < BUCKET_CAP);
    }

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

        // GARDE DE VERSION : tout paquet a [type, version] en tête.
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
            // --- Réponse du rendez-vous : la liste des autres (avec leur clé) -------
            //     Le rendez-vous ne nous attribue plus de numéro : notre identité est
            //     notre propre clé. On note juste « on est inscrit » + l'annuaire.
            Some(KIND_WELCOME) => {
                if let Some((world_hue, roster)) = decode_welcome(&bytes) {
                    link.my_id = Some(link.identity.id());
                    link.world_hue = Some(world_hue);
                    link.peers = roster.into_iter().collect();
                }
            }
            // --- PUNCH d'un pair : son paquet est ARRIVÉ, donc notre trou de retour
            //     est ouvert. On confirme la connexion directe. ----------------------
            Some(KIND_PUNCH) => {
                if let Some(id) = decode_punch(&bytes) {
                    let hole = holes.map.entry(id).or_default();
                    if !hole.open {
                        hole.open = true;
                        println!("Trou OUVERT avec le pair {} ! Connexion directe établie.", id.short());
                    }
                }
            }
            // --- RELAY d'un joueur à faible upload : on est son PARENT. On VÉRIFIE
            //     son sceau, puis on RECOPIE l'enveloppe SCELLÉE verbatim (juste
            //     rebasculée en KIND_STATE). Porteur d'octets, jamais faussaire. -----
            Some(KIND_RELAY) => match check_packet(&bytes) {
                Checked::Good(state) => {
                    if !link.is_muted(state.id) && link.accept_seq(state.id, state.seq) {
                        if teleported(&avatars, &state, now) {
                            // On ne RECOPIE même pas un tricheur (pas d'amplification de triche).
                            link.punish(state.id, "relais : téléport (vitesse impossible)");
                        } else {
                            // 6.5 : on ne RECOPIE que dans la limite du budget de relais de
                            // ce protégé, et vers au plus MAX_RELAY_FANOUT voisins → le
                            // facteur d'amplification réfléchie est borné.
                            let rc = relay_credits.entry(state.id).or_insert(RELAY_CAP);
                            if *rc >= 1.0 {
                                *rc -= 1.0;
                                let mut forward = bytes.clone();
                                forward[0] = KIND_STATE;
                                let mut sent = 0usize;
                                for (id, addr) in &link.peers {
                                    if *id != state.id {
                                        let _ = link.socket.send_to(*addr, &forward);
                                        sent += 1;
                                        if sent >= MAX_RELAY_FANOUT {
                                            break;
                                        }
                                    }
                                }
                            }
                            // On affiche le protégé chez nous même si le budget est épuisé.
                            ingest_state(
                                state, now, link.my_id, &mut holes, &mut avatars, &mut commands,
                                &mut meshes, &mut materials,
                            );
                        }
                    }
                }
                Checked::Faulty(id) => link.punish(id, "relais : état signé impossible (NaN)"),
                Checked::Unknown => {}
            },
            // --- État de l'orbe : seul le maître l'émet, et il le SIGNE. On vérifie
            //     le sceau (clé embarquée), puis on applique si ça SUPPLANTE notre
            //     version ET si le saut reste plausible (anti-vol / anti-gel). --------
            Some(KIND_ORB) => match check_orb(&bytes) {
                OrbChecked::Good(w) => {
                    let owner = w.owner;
                    if !link.is_muted(owner) {
                        // 6.4 : dernière position connue du revendiqueur (preuve de contact).
                        let claimer_pos =
                            avatars.map.get(&owner).and_then(|p| p.buffer.back()).map(|s| s.pos);
                        match apply_incoming(&mut orb, w, now, claimer_pos) {
                            OrbApply::Implausible => link.punish(owner, "orbe : saut de version aberrant"),
                            OrbApply::NoContact => link.punish(owner, "orbe : revendiquée sans contact"),
                            _ => {}
                        }
                    }
                }
                OrbChecked::Faulty(id) => link.punish(id, "orbe : état signé impossible (NaN)"),
                OrbChecked::Unknown => {}
            },
            // --- État d'un pair : sceau + anti-rejeu + réputation, puis on le range.
            Some(KIND_STATE) => match check_packet(&bytes) {
                Checked::Good(state) => {
                    if !link.is_muted(state.id) && link.accept_seq(state.id, state.seq) {
                        if teleported(&avatars, &state, now) {
                            link.punish(state.id, "téléport (vitesse impossible)");
                        } else {
                            ingest_state(
                                state, now, link.my_id, &mut holes, &mut avatars, &mut commands,
                                &mut meshes, &mut materials,
                            );
                        }
                    }
                }
                Checked::Faulty(id) => link.punish(id, "état signé impossible (NaN)"),
                Checked::Unknown => {}
            },
            // --- ACCUSATION d'un témoin (chap. 6.7) : « j'ai banni ce tricheur ». On
            //     n'agit qu'au QUORUM d'accusateurs distincts (anti-framing), et
            //     l'accusateur doit avoir payé sa preuve de travail et ne pas être
            //     déjà muet chez nous (un banni ne vote plus). ----------------------
            Some(KIND_ACCUSE) => {
                if let Some((accuser, offender)) = decode_accuse(&bytes) {
                    if accuser.has_pow(POW_BITS) && accuser != offender && !link.is_muted(accuser) {
                        link.record_accusation(offender, accuser);
                    }
                }
            }
            _ => {}
        }
    }

    // On retire l'avatar d'un joueur dont on n'a plus reçu d'état depuis un moment.
    avatars.map.retain(|id, player| {
        let last_seen = player.buffer.back().map(|s| s.t).unwrap_or(now);
        let keep = now - last_seen < REMOTE_TIMEOUT;
        if !keep {
            commands.entity(player.body).despawn();
            println!("Joueur {} parti.", id.short());
        }
        keep
    });
}

/// Résultat de la vérification d'un paquet d'état. Trois cas, pour la réputation :
///   - `Good`    : sceau valide + contenu correct → on traite ;
///   - `Faulty`  : sceau VALIDE mais contenu impossible (NaN) → faute ATTRIBUABLE
///                 (seul le détenteur de la clé a pu signer ça) → strike ;
///   - `Unknown` : sceau invalide → on JETTE sans accuser (sinon un attaquant
///                 forgerait « id = victime » pourri pour faire bannir la victime).
enum Checked {
    Good(PlayerState),
    Faulty(PeerId),
    Unknown,
}

/// Vérifie le sceau d'un paquet d'état (direct ou relayé). La clé est LUE dans le
/// paquet (`sig_ok`) : pas d'annuaire, identité auto-certifiée. C'est ICI que meurt
/// l'usurpation : un paquet signé par une autre clé que celle qu'il revendique a un
/// sceau invalide → jeté.
fn check_packet(bytes: &[u8]) -> Checked {
    if !sig_ok(bytes) {
        return Checked::Unknown; // sceau invalide → non attribuable → jeté sans accuser
    }
    match decode_canonical(bytes) {
        // 6.2 : une identité sans preuve de travail est tout bonnement ignorée
        // (elle n'a pas « payé » son entrée) — et on ne la juge pas (pas de strike).
        Some(state) if !state.id.has_pow(POW_BITS) => Checked::Unknown,
        Some(state) => Checked::Good(state),
        None => match claimed_id(bytes) {
            Some(id) if id.has_pow(POW_BITS) => Checked::Faulty(id), // signé + payé MAIS contenu impossible
            _ => Checked::Unknown,
        },
    }
}

/// VALIDATION DE MOUVEMENT (chap. 6.3) : l'état reçu implique-t-il un déplacement
/// physiquement impossible depuis le dernier qu'on a accepté de ce joueur ? On
/// compare à son dernier instantané connu. Le tout premier paquet (aucun historique)
/// n'est pas jugé (rien à comparer). Un « oui » est une faute ATTRIBUABLE : l'état
/// est validement signé, mais sa téléportation trahit un triche.
fn teleported(avatars: &RemoteAvatars, state: &PlayerState, now: f32) -> bool {
    match avatars.map.get(&state.id).and_then(|p| p.buffer.back()) {
        Some(prev) => {
            let dt = now - prev.t;
            !move_plausible(prev.pos, Vec3::new(state.x, state.y, state.z), dt)
        }
        None => false,
    }
}

/// Pendant pour l'orbe.
enum OrbChecked {
    Good(OrbWire),
    Faulty(PeerId),
    Unknown,
}

fn check_orb(bytes: &[u8]) -> OrbChecked {
    if !orb_sig_ok(bytes) {
        return OrbChecked::Unknown; // sceau invalide → jeté sans accuser
    }
    match decode_orb(bytes) {
        Some(w) if !w.owner.has_pow(POW_BITS) => OrbChecked::Unknown, // identité non minée → ignorée
        Some(w) => OrbChecked::Good(w),
        None => match claimed_owner(bytes) {
            Some(id) if id.has_pow(POW_BITS) => OrbChecked::Faulty(id),
            _ => OrbChecked::Unknown,
        },
    }
}

/// Range un état reçu (direct ou relayé) : marque le trou ouvert, empile
/// l'instantané dans la file du joueur, et crée son avatar au premier paquet.
#[allow(clippy::too_many_arguments)]
fn ingest_state(
    state: PlayerState,
    now: f32,
    my_id: Option<PeerId>,
    holes: &mut Holes,
    avatars: &mut RemoteAvatars,
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    if Some(state.id) == my_id {
        return; // jamais notre propre avatar
    }
    holes.map.entry(state.id).or_default().open = true;
    let snap = Snapshot {
        t: now,
        pos: Vec3::new(state.x, state.y, state.z),
        vel: Vec3::new(state.vx, state.vy, state.vz),
        yaw: state.yaw,
        pitch: state.pitch,
    };
    if let Some(player) = avatars.map.get_mut(&state.id) {
        player.parent = state.parent; // rôle à jour (sous tutelle ? de qui ?)
        player.buffer.push_back(snap);
        while player.buffer.len() > 2 && now - player.buffer.front().unwrap().t > 1.0 {
            player.buffer.pop_front();
        }
    } else {
        // Plafond anti-DoS : on refuse de créer un avatar de plus au-delà de la limite.
        if avatars.map.len() >= MAX_AVATARS {
            return;
        }
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
        println!("Nouveau joueur {} apparu.", state.id.short());
    }
}

/// Crée l'avatar 3D d'un joueur distant (corps articulé + tête à nez directionnel).
/// Renvoie (entité du corps, entité du pivot de tête).
fn spawn_avatar(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    state: &PlayerState,
) -> (Entity, Entity) {
    let torso = meshes.add(Capsule3d::new(0.17, 0.45));
    let head = meshes.add(Sphere::new(0.14));
    let limb = meshes.add(Capsule3d::new(0.07, 0.40));
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
            p.spawn((
                Mesh3d(torso),
                MeshMaterial3d(skin.clone()),
                Transform::from_xyz(0.0, 0.10, 0.0),
            ));
            for x in [-0.30, 0.30] {
                p.spawn((
                    Mesh3d(limb.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::from_xyz(x, 0.08, 0.0),
                ));
            }
            for x in [-0.11, 0.11] {
                p.spawn((
                    Mesh3d(limb.clone()),
                    MeshMaterial3d(skin.clone()),
                    Transform::from_xyz(x, -0.45, 0.0),
                ));
            }
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
