//! La couche réseau du jeu — faite main, avec la bibliothèque standard de Rust
//! pour le transport (`std::net`), plus une fine couche qui la relie à Bevy.
//!
//! On voit exactement comment une position de joueur devient une suite d'octets,
//! part dans le réseau et revient. Aucune « boîte noire » côté réseau.
//!
//! # Organisation de ce fichier
//!   1) LE MESSAGE        : `PlayerState` + `encode` / `decode` (octets bruts)
//!   2) LE PAIR (PEER)    : `NetPeer`, la prise UDP (envoyer / relever le courrier)
//!   3) LA COULEUR        : couleur de skin aléatoire (`random_color`)
//!   4) LE MODE DÉMO      : `run_demo` (texte seul, pour observer les paquets)
//!   5) L'INTÉGRATION JEU : ressources + systèmes Bevy qui branchent tout ça en 3D
//!
//! # Comment jouer à deux fenêtres (sur le même PC)
//!   Terminal 1 :  nix-shell --run "cargo run -- a"
//!   Terminal 2 :  nix-shell --run "cargo run -- b"
//! Bouge en ZQSD dans une fenêtre : ton avatar (de ta couleur) apparaît et bouge
//! dans l'autre fenêtre.
//!
//! # Tester un vrai mauvais réseau (plus tard)
//!   sudo tc qdisc add dev lo root netem delay 100ms loss 10%   # dégrade
//!   sudo tc qdisc del dev lo root                              # remet normal

use bevy::prelude::*;
use std::collections::VecDeque;
use std::f32::consts::{PI, TAU};
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

// ============================================================================
// 1) LE MESSAGE : ce qu'un joueur envoie aux autres
// ============================================================================

/// L'état d'un joueur transmis sur le réseau : qui (`id`), où (`x,y,z`) et de
/// quelle couleur (`r,g,b`, son « skin »).
#[derive(Clone, Copy, Debug)]
pub struct PlayerState {
    pub id: u8,    // identifiant du joueur (1 octet : jusqu'à 255 joueurs)
    pub x: f32,    // position gauche/droite
    pub y: f32,    // position haut/bas (hauteur)
    pub z: f32,    // position avant/arrière
    pub yaw: f32,  // orientation du corps gauche/droite (radians)
    pub pitch: f32, // inclinaison de la tête haut/bas (radians)
    pub r: f32,    // couleur du skin : rouge
    pub g: f32,    // couleur du skin : vert
    pub b: f32,    // couleur du skin : bleu
}

// Taille exacte d'un paquet, calculée à la main pour bien comprendre :
//   1 octet (id) + 8 nombres f32 de 4 octets (x,y,z,yaw,pitch,r,g,b)
//   = 1 + 32 = 33 octets.
const PACKET_SIZE: usize = 1 + 4 * 8;

/// « Sérialiser » : transformer la fiche `PlayerState` en octets bruts à envoyer.
/// `to_le_bytes` découpe chaque nombre en 4 octets (sens « little-endian »).
/// L'émetteur et le récepteur doivent juste utiliser le même sens — on choisit LE.
fn encode(p: &PlayerState) -> [u8; PACKET_SIZE] {
    let mut buf = [0u8; PACKET_SIZE];
    buf[0] = p.id;
    buf[1..5].copy_from_slice(&p.x.to_le_bytes());
    buf[5..9].copy_from_slice(&p.y.to_le_bytes());
    buf[9..13].copy_from_slice(&p.z.to_le_bytes());
    buf[13..17].copy_from_slice(&p.yaw.to_le_bytes());
    buf[17..21].copy_from_slice(&p.pitch.to_le_bytes());
    buf[21..25].copy_from_slice(&p.r.to_le_bytes());
    buf[25..29].copy_from_slice(&p.g.to_le_bytes());
    buf[29..33].copy_from_slice(&p.b.to_le_bytes());
    buf
}

/// L'inverse : reconstruire un `PlayerState` à partir des octets reçus.
/// Renvoie `None` si le paquet est trop court — on ne fait jamais confiance
/// aveuglément à ce qui vient du réseau.
fn decode(buf: &[u8]) -> Option<PlayerState> {
    if buf.len() < PACKET_SIZE {
        return None;
    }
    let id = buf[0];
    // `?` = « si la conversion rate, renvoie None tout de suite ».
    let x = f32::from_le_bytes(buf[1..5].try_into().ok()?);
    let y = f32::from_le_bytes(buf[5..9].try_into().ok()?);
    let z = f32::from_le_bytes(buf[9..13].try_into().ok()?);
    let yaw = f32::from_le_bytes(buf[13..17].try_into().ok()?);
    let pitch = f32::from_le_bytes(buf[17..21].try_into().ok()?);
    let r = f32::from_le_bytes(buf[21..25].try_into().ok()?);
    let g = f32::from_le_bytes(buf[25..29].try_into().ok()?);
    let b = f32::from_le_bytes(buf[29..33].try_into().ok()?);
    Some(PlayerState { id, x, y, z, yaw, pitch, r, g, b })
}

// ============================================================================
// 2) LE PAIR (PEER) : la prise réseau d'une session
// ============================================================================

/// La connexion réseau d'UNE session : sa boîte aux lettres UDP (`socket`)
/// et l'adresse de l'autre joueur (`remote`).
pub struct NetPeer {
    socket: UdpSocket,
    remote: SocketAddr,
}

impl NetPeer {
    /// Ouvre la prise locale et mémorise à qui on parle (tout sur 127.0.0.1).
    pub fn bind(local_port: u16, remote_port: u16) -> std::io::Result<NetPeer> {
        let socket = UdpSocket::bind(("127.0.0.1", local_port))?;
        // Mode non-bloquant : lire le réseau ne met JAMAIS le jeu en pause.
        // « Y a-t-il du courrier ? Non ? Tant pis, on continue. »
        socket.set_nonblocking(true)?;
        let remote = SocketAddr::from(([127, 0, 0, 1], remote_port));
        Ok(NetPeer { socket, remote })
    }

    /// Envoie notre position. Un seul paquet, aucun accusé de réception (c'est l'UDP).
    pub fn send(&self, state: &PlayerState) -> std::io::Result<()> {
        self.socket.send_to(&encode(state), self.remote)?;
        Ok(())
    }

    /// Relève TOUS les paquets arrivés depuis le dernier appel. Ne bloque jamais.
    pub fn poll(&self) -> Vec<PlayerState> {
        let mut received = Vec::new();
        let mut buf = [0u8; 64];
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((n, _from)) => {
                    if let Some(state) = decode(&buf[..n]) {
                        received.push(state);
                    }
                }
                // `WouldBlock` = boîte vide pour l'instant : ce n'est pas une erreur.
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        received
    }
}

// ============================================================================
// 3) LA COULEUR DE SKIN : une couleur vive aléatoire à chaque lancement
// ============================================================================

/// La couleur de skin de CETTE session, choisie au démarrage. On la garde dans
/// une ressource Bevy pour que le perso ET le réseau utilisent la même.
#[derive(Resource, Clone, Copy)]
pub struct MyColor(pub f32, pub f32, pub f32);

/// Tire une couleur vive aléatoire (rouge/vert/bleu, valeurs faites pour « glow »).
/// On évite toute dépendance externe : petit générateur pseudo-aléatoire maison.
pub fn random_color() -> (f32, f32, f32) {
    // Graine = nanosecondes actuelles, mélangées à l'identifiant du processus
    // (pour que deux fenêtres lancées au même instant aient des couleurs différentes).
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos())
        .unwrap_or(0);
    let mut x = nanos ^ std::process::id().wrapping_mul(2_654_435_761);
    // « xorshift » : on brasse les bits pour obtenir un nombre bien mélangé.
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    let hue = (x % 360) as f32; // une teinte au hasard sur le cercle des couleurs
    hsv_to_rgb(hue, 1.0, 1.2) // saturation max, valeur > 1 pour le néon
}

/// Convertit une couleur Teinte/Saturation/Valeur en Rouge/Vert/Bleu.
/// (La teinte donne « quelle couleur » ; on s'en sert pour tirer au hasard.)
fn hsv_to_rgb(h: f32, s: f32, v: f32) -> (f32, f32, f32) {
    let c = v * s;
    let h2 = h / 60.0;
    let x = c * (1.0 - ((h2 % 2.0) - 1.0).abs());
    let (r, g, b) = match h2 as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    (r + m, g + m, b + m)
}

// ============================================================================
// 4) LE MODE DÉMO : observer les paquets en texte, sans la 3D
// ============================================================================

/// `cargo run -- net-demo a` (ou `b`) : deux sessions s'envoient une position
/// qui tourne en cercle et affichent ce qu'elles reçoivent. Utile pour voir le
/// réseau seul. (Le vrai jeu, lui, se lance avec `cargo run -- a` / `b`.)
pub fn run_demo(role: &str) {
    let (local_port, remote_port, id) = ports_for_role(role);
    let peer = match NetPeer::bind(local_port, remote_port) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Impossible d'ouvrir le port {local_port} : {e}");
            return;
        }
    };
    let (r, g, b) = random_color();
    println!("Démo '{role}' : écoute {local_port}, parle à {remote_port}, joueur {id}.\n");

    let start = Instant::now();
    loop {
        let t = start.elapsed().as_secs_f32();
        let me =
            PlayerState { id, x: t.cos() * 2.0, y: 0.7, z: t.sin() * 2.0, yaw: t, pitch: 0.0, r, g, b };
        if let Err(e) = peer.send(&me) {
            eprintln!("Envoi raté : {e}");
        }
        for other in peer.poll() {
            println!(
                "  ← reçu du joueur {} : x={:.2}  y={:.2}  z={:.2}",
                other.id, other.x, other.y, other.z
            );
        }
        std::thread::sleep(Duration::from_millis(200));
    }
}

/// Selon le rôle ('a' ou 'b'), choisit les ports et l'identifiant.
/// 'a' écoute sur 5000 et parle à 5001 ; 'b' fait l'inverse.
fn ports_for_role(role: &str) -> (u16, u16, u8) {
    match role {
        "b" | "B" => (5001, 5000, 2),
        _ => (5000, 5001, 1), // 'a' par défaut
    }
}

// ============================================================================
// 5) INTÉGRATION DANS LE JEU (Bevy)
// ============================================================================

/// Le lien réseau de la session, rangé dans une ressource pour que les systèmes
/// Bevy puissent l'utiliser. Présent uniquement en mode multijoueur.
#[derive(Resource)]
pub struct NetLink {
    peer: NetPeer,
    my_id: u8,
    my_color: (f32, f32, f32),
}

impl NetLink {
    /// Prépare le lien réseau pour le rôle donné, avec notre couleur de skin.
    pub fn new(role: &str, color: (f32, f32, f32)) -> std::io::Result<NetLink> {
        let (local, remote, id) = ports_for_role(role);
        let peer = NetPeer::bind(local, remote)?;
        println!("Multijoueur '{role}' : écoute {local}, parle à {remote}, joueur {id}.");
        Ok(NetLink { peer, my_id: id, my_color: color })
    }
}

// ----------------------------------------------------------------------------
// Réglages du « netcode » (chapitre 2 — fluidité)
// ----------------------------------------------------------------------------
// On envoie moins souvent que 60/s : 20 instantanés/s suffisent, car on REMPLIT
// les trous par interpolation. Et on dessine toujours un peu dans le passé (le
// « retard d'interpolation ») pour avoir TOUJOURS deux instantanés autour du
// moment qu'on veut afficher — même si un paquet arrive en retard ou se perd.
const SEND_HZ: f32 = 20.0; // paquets envoyés par seconde
const INTERP_DELAY: f32 = 0.10; // on dessine 100 ms dans le passé

/// Un « instantané » reçu : où était le joueur distant, et À QUEL MOMENT on l'a
/// reçu (horloge interne du jeu). On en garde plusieurs pour glisser entre eux.
#[derive(Clone, Copy)]
struct Snapshot {
    t: f32, // instant de réception (secondes)
    pos: Vec3,
    yaw: f32,
    pitch: f32,
}

/// Tout ce qu'on retient d'un joueur distant : ses deux entités 3D (corps + tête)
/// et la file de ses derniers instantanés, qu'on interpole image par image.
struct RemotePlayer {
    body: Entity,
    head: Entity,
    buffer: VecDeque<Snapshot>,
}

/// Mémorise tous les joueurs distants connus, par identifiant.
#[derive(Resource, Default)]
pub struct RemoteAvatars {
    map: std::collections::HashMap<u8, RemotePlayer>,
}

/// Marque le CORPS d'un joueur distant (position + orientation gauche/droite).
#[derive(Component)]
pub struct RemoteAvatar {
    pub id: u8,
}

/// Marque le pivot de la TÊTE d'un joueur distant (inclinaison haut/bas).
#[derive(Component)]
pub struct RemoteHead;

/// Système : envoie NOTRE position (et couleur) à l'autre joueur, chaque frame.
/// (À 60 images/s ça fait 60 petits paquets/s — on lissera/ralentira plus tard.)
pub fn net_send(
    time: Res<Time>,
    mut accumulator: Local<f32>,
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
    *accumulator = 0.0;

    let Ok(transform) = player.single() else {
        return;
    };
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
        x: transform.translation.x,
        y: transform.translation.y,
        z: transform.translation.z,
        yaw,
        pitch,
        r,
        g,
        b,
    };
    let _ = link.peer.send(&me); // on ignore l'échec : le prochain paquet repart
}

/// Système : relève les positions reçues et met à jour (ou crée) l'avatar du
/// joueur distant correspondant.
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
            avatars
                .map
                .insert(state.id, RemotePlayer { body, head: head_entity, buffer });
            println!("Nouveau joueur {} apparu.", state.id);
        }
    }
}

/// Système : à CHAQUE image, place chaque avatar distant à l'état interpolé
/// entre ses deux instantanés qui entourent « maintenant − retard ». C'est lui
/// qui transforme des paquets espacés et saccadés en un mouvement fluide.
pub fn net_interpolate(
    time: Res<Time>,
    avatars: Res<RemoteAvatars>,
    // Corps et tête sont deux `Transform` : on sépare les requêtes avec `Without`.
    mut bodies: Query<&mut Transform, (With<RemoteAvatar>, Without<RemoteHead>)>,
    mut heads: Query<&mut Transform, (With<RemoteHead>, Without<RemoteAvatar>)>,
) {
    // Le moment qu'on veut dessiner : un peu dans le passé, pour avoir de la marge.
    let render_t = time.elapsed_secs() - INTERP_DELAY;

    for player in avatars.map.values() {
        if player.buffer.is_empty() {
            continue;
        }
        let (pos, yaw, pitch) = sample(&player.buffer, render_t);

        if let Ok(mut t) = bodies.get_mut(player.body) {
            t.translation = pos;
            t.rotation = Quat::from_rotation_y(yaw);
        }
        if let Ok(mut t) = heads.get_mut(player.head) {
            t.rotation = Quat::from_rotation_x(pitch);
        }
    }
}

/// Donne l'état interpolé à l'instant `t` à partir de la file d'instantanés.
/// On cherche la paire (a, b) qui encadre `t` et on glisse de a vers b.
fn sample(buf: &VecDeque<Snapshot>, t: f32) -> (Vec3, f32, f32) {
    // Avant le plus ancien instantané connu : on tient sa position (rien à interpoler).
    let first = buf.front().unwrap();
    if t <= first.t {
        return (first.pos, first.yaw, first.pitch);
    }
    // On parcourt les paires successives jusqu'à trouver celle qui encadre `t`.
    for i in 0..buf.len() - 1 {
        let a = buf[i];
        let b = buf[i + 1];
        if t <= b.t {
            // `alpha` = où se trouve `t` entre a.t et b.t, ramené dans [0, 1].
            let span = (b.t - a.t).max(1e-6); // évite la division par zéro
            let alpha = ((t - a.t) / span).clamp(0.0, 1.0);
            let pos = a.pos.lerp(b.pos, alpha);
            let yaw = lerp_angle(a.yaw, b.yaw, alpha);
            let pitch = a.pitch + (b.pitch - a.pitch) * alpha;
            return (pos, yaw, pitch);
        }
    }
    // Après le dernier instantané (on a « rattrapé » le réseau, ou un paquet
    // manque) : on tient la dernière position connue en attendant la suite.
    let last = buf.back().unwrap();
    (last.pos, last.yaw, last.pitch)
}

/// Interpole entre deux angles par le plus court chemin (gère le passage par
/// ±180° : sinon, en tournant, le corps ferait brièvement un tour à l'envers).
fn lerp_angle(a: f32, b: f32, t: f32) -> f32 {
    let mut diff = (b - a) % TAU;
    if diff > PI {
        diff -= TAU;
    } else if diff < -PI {
        diff += TAU;
    }
    a + diff * t
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
