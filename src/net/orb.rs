//! L'ORBE PARTAGÉE : le premier objet du monde qui n'appartient à PERSONNE par
//! naissance — et c'est tout l'intérêt.
//!
//! # La règle d'or du P2P sans serveur
//! Pour tout objet du monde, à chaque instant, UN SEUL pair décide de sa vérité :
//! on l'appelle le **maître** (l'autorité). Les autres ne font que recopier.
//!   - ton avatar  → tu en es le maître (toi seul décides où il est) ;
//!   - l'orbe      → personne au départ ; le DERNIER à l'avoir touchée en devient
//!                   le maître. La propriété saute donc de main en main.
//!
//! # Pourquoi c'est la base du chapitre 4
//! Ce transfert de propriété au contact, c'est une mini-migration d'autorité que
//! tu déclenches À LA MAIN. Le `version` (compteur qui ne fait que monter) en est
//! l'ancêtre du consensus ; le départage des « courses » (deux joueurs frappent en
//! même temps) préfigure ce que les *Shields* régleront proprement.
//!
//! # Le protocole (paquet KIND_ORB)
//!   maître → pairs : position, vitesse, id du maître, n° de version, couleur.
//! Règle d'émission : SEUL le maître émet. Règle de réception : on accepte l'état
//! entrant s'il SUPPLANTE le nôtre (version plus haute, ou égale mais id plus petit).

use super::link::NetLink;
use super::punch::Holes;
use super::wire::KIND_ORB;
use crate::world::{ROOM_HEIGHT, ROOM_SIZE};
use bevy::prelude::*;

/// Rayon de l'orbe (m) : sert au rendu, aux rebonds et à la détection de contact.
const ORB_RADIUS: f32 = 0.35;
/// Position de repos de l'orbe (au centre, à hauteur des yeux), avant toute prise.
const ORB_START: Vec3 = Vec3::new(0.0, 1.5, 0.0);
/// Vitesse imprimée à l'orbe quand on la frappe (m/s).
const HIT_SPEED: f32 = 5.0;
/// Demi-largeur du joueur pour le test de contact (≈ celle du module `player`).
const PLAYER_RADIUS: f32 = 0.30;
/// Fréquence à laquelle le maître diffuse l'état de l'orbe (paquets/s).
const ORB_SEND_HZ: f32 = 20.0;
/// Délai sans nouvelle du maître au-delà duquel on le présume parti (s). À 20 Hz,
/// 2 s = ~40 battements manqués. On le veut GÉNÉREUX exprès : c'est la règle des
/// vrais systèmes (Raft & co.), où le délai d'élection est un GROS multiple du
/// battement, sinon une simple micro-coupure (PC chargé, rafale de paquets perdus)
/// fait basculer l'orbe à tort — et comme l'élection retombe sur le plus petit id,
/// on verrait la balle changer de maître toute seule, sans que personne ne la touche.
const MASTER_TIMEOUT: f32 = 2.0;
/// Couleur de l'orbe tant que personne ne l'a touchée (blanc bleuté néon).
const NEUTRAL_COLOR: (f32, f32, f32) = (0.85, 0.85, 1.0);

/// Le paquet « état de l'orbe » tel qu'il voyage sur le réseau (avant/après octets).
pub(crate) struct OrbWire {
    pub owner: u8,    // id du maître (celui qui a touché l'orbe en dernier)
    pub version: u16, // compteur de propriété : +1 à chaque transfert
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub vx: f32,
    pub vy: f32,
    pub vz: f32,
    pub r: f32, // couleur (celle du maître)
    pub g: f32,
    pub b: f32,
}

// Taille exacte, calculée à la main : 1 (type) + 1 (owner) + 2 (version u16)
//   + 9 nombres f32 de 4 octets (x,y,z, vx,vy,vz, r,g,b) = 4 + 36 = 40 octets.
const ORB_SIZE: usize = 1 + 1 + 2 + 4 * 9;

/// Sérialiser l'état de l'orbe en octets bruts (même sens little-endian que `message`).
pub(crate) fn encode_orb(o: &OrbWire) -> [u8; ORB_SIZE] {
    let mut buf = [0u8; ORB_SIZE];
    buf[0] = KIND_ORB;
    buf[1] = o.owner;
    buf[2..4].copy_from_slice(&o.version.to_le_bytes());
    buf[4..8].copy_from_slice(&o.x.to_le_bytes());
    buf[8..12].copy_from_slice(&o.y.to_le_bytes());
    buf[12..16].copy_from_slice(&o.z.to_le_bytes());
    buf[16..20].copy_from_slice(&o.vx.to_le_bytes());
    buf[20..24].copy_from_slice(&o.vy.to_le_bytes());
    buf[24..28].copy_from_slice(&o.vz.to_le_bytes());
    buf[28..32].copy_from_slice(&o.r.to_le_bytes());
    buf[32..36].copy_from_slice(&o.g.to_le_bytes());
    buf[36..40].copy_from_slice(&o.b.to_le_bytes());
    buf
}

/// Reconstruire un `OrbWire` depuis les octets reçus. `None` si trop court ou pas
/// du bon type — on ne fait jamais confiance aveuglément au réseau.
pub(crate) fn decode_orb(buf: &[u8]) -> Option<OrbWire> {
    if buf.len() < ORB_SIZE || buf[0] != KIND_ORB {
        return None;
    }
    let owner = buf[1];
    let version = u16::from_le_bytes(buf[2..4].try_into().ok()?);
    let x = f32::from_le_bytes(buf[4..8].try_into().ok()?);
    let y = f32::from_le_bytes(buf[8..12].try_into().ok()?);
    let z = f32::from_le_bytes(buf[12..16].try_into().ok()?);
    let vx = f32::from_le_bytes(buf[16..20].try_into().ok()?);
    let vy = f32::from_le_bytes(buf[20..24].try_into().ok()?);
    let vz = f32::from_le_bytes(buf[24..28].try_into().ok()?);
    let r = f32::from_le_bytes(buf[28..32].try_into().ok()?);
    let g = f32::from_le_bytes(buf[32..36].try_into().ok()?);
    let b = f32::from_le_bytes(buf[36..40].try_into().ok()?);
    Some(OrbWire { owner, version, x, y, z, vx, vy, vz, r, g, b })
}

/// L'état logique de l'orbe, partagé par tout le client (une seule par monde).
#[derive(Resource)]
pub struct Orb {
    pub(crate) pos: Vec3,
    pub(crate) vel: Vec3,
    pub(crate) owner: Option<u8>, // None = personne ne l'a encore touchée
    pub(crate) version: u16,
    pub(crate) color: (f32, f32, f32),
    mat: Handle<StandardMaterial>,        // pour recolorer l'orbe au changement de maître
    shown: Option<(f32, f32, f32)>,       // dernière couleur réellement appliquée
    last_heard: f32,                      // instant du dernier paquet reçu du maître (pour la migration)
}

/// Marque l'entité 3D (la sphère) qui matérialise l'orbe à l'écran.
#[derive(Component)]
pub struct OrbBall;

/// Décide si un état entrant (reçu du réseau) doit SUPPLANTER l'état courant.
/// C'est toute la logique d'autorité, en une fonction :
///   - version plus haute        → touche plus récente, il l'emporte ;
///   - version égale + id ≤       → flux normal du maître courant (==) OU départage
///                                  d'une course en faveur du plus petit id (<).
/// (`<=` couvre les deux cas ; `>` à version égale = on garde notre état.)
fn supersedes(in_ver: u16, in_owner: u8, cur_ver: u16, cur_owner: Option<u8>) -> bool {
    match cur_owner {
        None => true, // pas encore de maître : le premier paquet fait foi
        Some(cur) => in_ver > cur_ver || (in_ver == cur_ver && in_owner <= cur),
    }
}

/// STARTUP (client) : crée la sphère néon de l'orbe et installe la ressource `Orb`.
pub fn setup_orb(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(Sphere::new(ORB_RADIUS));
    let mat = materials.add(StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(NEUTRAL_COLOR.0, NEUTRAL_COLOR.1, NEUTRAL_COLOR.2),
        ..default()
    });

    commands.spawn((
        OrbBall,
        Mesh3d(mesh),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(ORB_START),
    ));

    commands.insert_resource(Orb {
        pos: ORB_START,
        vel: Vec3::ZERO,
        owner: None,
        version: 0,
        color: NEUTRAL_COLOR,
        mat,
        shown: Some(NEUTRAL_COLOR),
        last_heard: 0.0,
    });
}

/// UPDATE (client) : détecte le CONTACT entre notre joueur et l'orbe, et déclenche
/// le transfert de propriété — on devient maître, on imprime une vitesse, et on
/// incrémente la version. On ne frappe qu'au FRONT du contact (pas à chaque image
/// tant qu'on reste collé), grâce au booléen mémorisé `touching`.
pub fn orb_grab(
    link: Res<NetLink>,
    mut orb: ResMut<Orb>,
    player: Query<&Transform, With<crate::player::Player>>,
    mut touching: Local<bool>,
) {
    let Some(my_id) = link.my_id else {
        return;
    };
    let Ok(pt) = player.single() else {
        return;
    };
    let pc = pt.translation;

    // Contact « cylindre » : on rapproche l'orbe du corps en la projetant sur la
    // hauteur du joueur (des pieds ~0,2 m à la tête ~1,4 m), puis on mesure.
    let closest = Vec3::new(pc.x, orb.pos.y.clamp(0.2, 1.4), pc.z);
    let touch = orb.pos.distance(closest) < ORB_RADIUS + PLAYER_RADIUS;

    if touch && !*touching {
        // Direction de frappe : depuis la poitrine du joueur vers l'orbe (en 3D, donc
        // l'orbe monte si elle est plus haute → elle rebondira aussi sol/plafond).
        let dir = (orb.pos - Vec3::new(pc.x, 0.9, pc.z)).normalize_or_zero();
        let dir = if dir == Vec3::ZERO { Vec3::Y } else { dir };
        orb.vel = dir * HIT_SPEED;
        orb.owner = Some(my_id);
        orb.version = orb.version.wrapping_add(1); // débordement sans importance ici
        orb.color = link.my_color;
        println!("Orbe frappée — tu en es le maître (v{}).", orb.version);
    }
    *touching = touch;
}

/// UPDATE (client) : fait vivre l'orbe et l'affiche.
///   - si JE suis le maître  → je simule la physique (avance + rebonds sur 6 faces) ;
///   - si quelqu'un d'autre l'est → j'EXTRAPOLE (pos += vitesse·dt), recalé à chaque
///     paquet reçu : ça lisse les 20 Hz du réseau sans rien recalculer.
/// Puis on place la sphère et on la recolore si le maître a changé.
pub fn orb_simulate(
    time: Res<Time>,
    link: Res<NetLink>,
    mut orb: ResMut<Orb>,
    mut ball: Query<&mut Transform, With<OrbBall>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    let am_owner = orb.owner.is_some() && orb.owner == link.my_id;

    if orb.owner.is_some() {
        let step = orb.vel * dt;
        orb.pos += step;

        if am_owner {
            // Rebonds élastiques sur les 6 parois (l'orbe garde son énergie : façon
            // logo flottant). Seul le maître fait ça : les autres se recalent sur lui.
            let h = ROOM_SIZE / 2.0 - ORB_RADIUS;
            if orb.pos.x < -h {
                orb.pos.x = -h;
                orb.vel.x = orb.vel.x.abs();
            } else if orb.pos.x > h {
                orb.pos.x = h;
                orb.vel.x = -orb.vel.x.abs();
            }
            if orb.pos.z < -h {
                orb.pos.z = -h;
                orb.vel.z = orb.vel.z.abs();
            } else if orb.pos.z > h {
                orb.pos.z = h;
                orb.vel.z = -orb.vel.z.abs();
            }
            let (lo, hi) = (ORB_RADIUS, ROOM_HEIGHT - ORB_RADIUS);
            if orb.pos.y < lo {
                orb.pos.y = lo;
                orb.vel.y = orb.vel.y.abs();
            } else if orb.pos.y > hi {
                orb.pos.y = hi;
                orb.vel.y = -orb.vel.y.abs();
            }
        }
    }

    if let Ok(mut tf) = ball.single_mut() {
        tf.translation = orb.pos;
        tf.rotate_y(dt * 1.5); // légère rotation : l'orbe « vit »
    }

    // Recolore uniquement quand la couleur change (nouveau maître) — pas chaque image.
    if orb.shown != Some(orb.color) {
        if let Some(mat) = materials.get_mut(&orb.mat) {
            let (r, g, b) = orb.color;
            mat.emissive = LinearRgba::rgb(r, g, b);
        }
        orb.shown = Some(orb.color);
    }
}

/// UPDATE (client) : si JE suis le maître, diffuser l'état de l'orbe à tous les
/// pairs joignables (trou NAT ouvert), à `ORB_SEND_HZ`. Les non-maîtres se taisent.
pub fn orb_send(
    time: Res<Time>,
    mut acc: Local<f32>,
    link: Res<NetLink>,
    holes: Res<Holes>,
    orb: Res<Orb>,
) {
    let Some(my_id) = link.my_id else {
        return;
    };
    if orb.owner != Some(my_id) {
        return; // autorité unique : seul le maître émet
    }

    *acc += time.delta_secs();
    let interval = 1.0 / ORB_SEND_HZ;
    if *acc < interval {
        return;
    }
    *acc = 0.0;

    let bytes = encode_orb(&OrbWire {
        owner: my_id,
        version: orb.version,
        x: orb.pos.x,
        y: orb.pos.y,
        z: orb.pos.z,
        vx: orb.vel.x,
        vy: orb.vel.y,
        vz: orb.vel.z,
        r: orb.color.0,
        g: orb.color.1,
        b: orb.color.2,
    });

    for (id, addr) in &link.peers {
        if holes.map.get(id).map_or(false, |h| h.open) {
            let _ = link.socket.send_to(*addr, &bytes);
        }
    }
}

/// Appliquer un paquet d'orbe reçu du réseau (appelé depuis `net_receive`, qui est
/// le seul à vider la prise UDP). On n'écrase notre état QUE s'il est supplanté.
/// `now` sert à dater ce battement : c'est lui qui prouve que le maître est vivant
/// (cf. `orb_migrate`).
pub(crate) fn apply_incoming(orb: &mut Orb, w: OrbWire, now: f32) {
    if supersedes(w.version, w.owner, orb.version, orb.owner) {
        // Trace : on ne log QUE lorsque le maître change vraiment (pas à chaque
        // battement du maître courant). Avec les logs de `orb_grab` (frappe) et
        // `orb_migrate` (reprise), la console explique alors CHAQUE changement de
        // couleur : frappe / migration / adoption d'un maître distant.
        if orb.owner != Some(w.owner) {
            println!(
                "Orbe : j'adopte le maître {} (v{}) reçu du réseau.",
                w.owner, w.version
            );
        }
        orb.owner = Some(w.owner);
        orb.version = w.version;
        orb.pos = Vec3::new(w.x, w.y, w.z);
        orb.vel = Vec3::new(w.vx, w.vy, w.vz);
        orb.color = (w.r, w.g, w.b);
        orb.last_heard = now; // le maître vient de se manifester : il est vivant
    }
}

/// UPDATE (client) : la MIGRATION D'HÔTE de l'orbe — le cœur du chapitre 4.
///
/// Si le maître ne s'est plus manifesté depuis `MASTER_TIMEOUT`, on le présume parti
/// et on élit son remplaçant. L'élection est **déterministe** : chaque témoin prend
/// le plus petit id parmi {soi} ∪ {pairs connus}, l'ancien maître exclu. Comme tout
/// le monde a la même liste et la même règle, tout le monde désigne le MÊME gagnant
/// sans avoir à voter. Seul le gagnant se proclame ; il incrémente la version, ce qui
/// règle d'office un éventuel « split-brain » (l'ancien maître, s'il réapparaît, verra
/// une version plus haute et abdiquera via `supersedes`).
///
/// Limite assumée (à lever plus tard) : on fait confiance à la liste `peers` du
/// rendez-vous pour savoir qui est encore là. Si le rendez-vous est mort ET que le
/// plus petit id élu est lui aussi parti, l'orbe peut rester figée — la détection de
/// vivacité fine viendra avec le relais/parent (chap. 4.1) et l'anti-triche (chap. 5).
pub fn orb_migrate(time: Res<Time>, link: Res<NetLink>, mut orb: ResMut<Orb>) {
    let Some(my_id) = link.my_id else {
        return;
    };
    // Rien à reprendre si l'orbe n'a pas de maître, ou si c'est déjà moi.
    let Some(owner) = orb.owner else {
        return;
    };
    if owner == my_id {
        return;
    }
    // Le maître bat-il encore ? Tant qu'on l'entend, pas de migration.
    let now = time.elapsed_secs();
    if now - orb.last_heard < MASTER_TIMEOUT {
        return;
    }

    // ÉLECTION déterministe : le plus petit id, l'ancien maître écarté.
    let mut winner = my_id;
    for id in link.peers.keys() {
        if *id != owner && *id < winner {
            winner = *id;
        }
    }

    if winner == my_id {
        // C'est moi : je reprends l'orbe à son dernier état connu (position + vitesse
        // extrapolées) et je relance sa simulation. La version monte → mes paquets
        // supplantent tout le reste.
        orb.owner = Some(my_id);
        orb.version = orb.version.wrapping_add(1);
        orb.color = link.my_color;
        orb.last_heard = now;
        println!("Maître {owner} disparu — je reprends l'orbe (v{}).", orb.version);
    }
    // Sinon : quelqu'un d'autre est élu ; on attend simplement son premier paquet.
}
