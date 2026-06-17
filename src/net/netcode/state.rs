//! L'ÉTAT du netcode : les RÉGLAGES (constantes à tourner), l'instantané reçu,
//! la fiche d'un joueur distant, et les marqueurs des entités 3D.

use bevy::prelude::*;
use std::collections::VecDeque;

// ----------------------------------------------------------------------------
// RÉGLAGES — les « boutons » du netcode. Tourne-les et observe l'effet.
// ----------------------------------------------------------------------------
/// Paquets envoyés par seconde (on remplit les trous par interpolation).
pub(super) const SEND_HZ: f32 = 20.0;
/// On dessine toujours un peu dans le passé, pour avoir une marge de paquets.
pub(super) const INTERP_DELAY: f32 = 0.10;
/// On ne PRÉDIT jamais plus que ça à l'aveugle (sécurité si le joueur disparaît).
pub(super) const MAX_EXTRAPOLATION: f32 = 0.25;
/// Temps pour ~rejoindre la cible (ressort amorti) : ↑ = + lisse, + de retard.
pub(super) const SMOOTH_TIME: f32 = 0.08;
/// Réactivité de l'horloge de lecture au retard accumulé.
pub(super) const CATCHUP_GAIN: f32 = 1.5;
/// Accélère/ralentit la lecture de ±10 % max (invisible à l'œil).
pub(super) const MAX_WARP: f32 = 0.10;
/// Sans nouvel état d'un joueur depuis ce délai (s), on retire son avatar.
pub(super) const REMOTE_TIMEOUT: f32 = 5.0;

/// Un « instantané » reçu : où était le joueur distant, à quelle VITESSE il allait,
/// et À QUEL MOMENT on l'a reçu. On en garde plusieurs pour glisser entre eux.
#[derive(Clone, Copy)]
pub(super) struct Snapshot {
    pub(super) t: f32,    // instant de réception (secondes)
    pub(super) pos: Vec3,
    pub(super) vel: Vec3, // vitesse RÉELLE envoyée par l'émetteur (m/s) — sert à prédire
    pub(super) yaw: f32,
    pub(super) pitch: f32,
}

/// Tout ce qu'on retient d'un joueur distant : ses deux entités 3D (corps + tête),
/// la file de ses derniers instantanés, et l'état interne du lissage (ressort amorti).
pub(super) struct RemotePlayer {
    pub(super) body: Entity,
    pub(super) head: Entity,
    pub(super) buffer: VecDeque<Snapshot>,
    pub(super) clock: f32, // notre horloge de LECTURE perso (avance plus/moins vite que le temps réel)
    // Vitesses internes du ressort amorti (SmoothDamp) : une par grandeur lissée.
    pub(super) smooth_vel: Vec3,
    pub(super) yaw_vel: f32,
    pub(super) pitch_vel: f32,
    /// Rôle : id de SON tuteur (relais) si ce joueur est sous tutelle, sinon 0.
    /// Alimente les badges de rôle (cf. `badges`).
    pub(super) parent: u8,
}

/// Mémorise tous les joueurs distants connus, par identifiant.
#[derive(Resource, Default)]
pub struct RemoteAvatars {
    pub(super) map: std::collections::HashMap<u8, RemotePlayer>,
}

/// Marque le CORPS d'un joueur distant (position + orientation gauche/droite).
/// `pub(crate)` car il apparaît dans la signature des systèmes publics du netcode.
#[derive(Component)]
pub(crate) struct RemoteAvatar {
    /// Identifiant du joueur : pas encore lu, mais servira au départ d'un joueur.
    #[allow(dead_code)]
    pub(super) id: u8,
}

/// Marque le pivot de la TÊTE d'un joueur distant (inclinaison haut/bas).
#[derive(Component)]
pub(crate) struct RemoteHead;
