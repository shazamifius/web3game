//! ANIMATION : à chaque image, place chaque avatar distant au bon endroit.
//!
//! Deux mécanismes ici :
//!   1) l'HORLOGE DE LECTURE adaptative (l'idée « à la Discord ») : décide QUEL
//!      instant on affiche, en restant ~INTERP_DELAY dans le passé ;
//!   2) la RÉCONCILIATION par ressort amorti : glisse l'avatar vers la cible
//!      calculée par `predict::sample`, sans à-coup ni dépassement.

use super::predict::sample;
use super::smooth::{smooth_damp_angle, smooth_damp_vec3};
use super::state::{RemoteAvatar, RemoteAvatars, RemoteHead, CATCHUP_GAIN, INTERP_DELAY, MAX_WARP};
use bevy::prelude::*;

pub fn net_interpolate(
    time: Res<Time>,
    mut avatars: ResMut<RemoteAvatars>,
    // Corps et tête sont deux `Transform` : on sépare les requêtes avec `Without`.
    mut bodies: Query<&mut Transform, (With<RemoteAvatar>, Without<RemoteHead>)>,
    mut heads: Query<&mut Transform, (With<RemoteHead>, Without<RemoteAvatar>)>,
) {
    let dt = time.delta_secs();

    for player in avatars.map.values_mut() {
        if player.buffer.is_empty() {
            continue;
        }

        // --- HORLOGE DE LECTURE ADAPTATIVE (l'idée « à la Discord ») ----------
        // On veut rester ~INTERP_DELAY derrière le paquet le plus récent. Si on a
        // pris du retard (la file s'allonge devant nous), on lit un peu plus vite
        // pour rattraper ; si on risque la disette, on lit plus lentement. Le tout
        // borné à ±MAX_WARP : l'avatar suit toujours son vrai chemin, sans sauter.
        let newest = player.buffer.back().unwrap().t;
        let lead = newest - player.clock; // de combien la file est en avance sur nous
        let warp = (CATCHUP_GAIN * (lead - INTERP_DELAY)).clamp(-MAX_WARP, MAX_WARP);
        player.clock += dt * (1.0 + warp);

        // La CIBLE : interpolée si on a les deux points, prédite (extrapolée) sinon.
        let (pos, yaw, pitch) = sample(&player.buffer, player.clock);

        if let Ok(mut t) = bodies.get_mut(player.body) {
            // RÉCONCILIATION par RESSORT AMORTI (SmoothDamp) : au lieu de glisser à
            // taux fixe (qui traîne toujours derrière une cible en mouvement), le
            // ressort tient compte de sa propre vitesse → il rattrape vite, SANS
            // dépasser. Une correction (prédiction fausse) est absorbée en douceur.
            t.translation = smooth_damp_vec3(t.translation, pos, &mut player.smooth_vel, dt);
            let current_yaw = t.rotation.to_euler(EulerRot::YXZ).0;
            let new_yaw = smooth_damp_angle(current_yaw, yaw, &mut player.yaw_vel, dt);
            t.rotation = Quat::from_rotation_y(new_yaw);
        }
        if let Ok(mut t) = heads.get_mut(player.head) {
            let current_pitch = t.rotation.to_euler(EulerRot::XYZ).0;
            let new_pitch = smooth_damp_angle(current_pitch, pitch, &mut player.pitch_vel, dt);
            t.rotation = Quat::from_rotation_x(new_pitch);
        }
    }
}
