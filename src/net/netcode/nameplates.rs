//! ÉTIQUETTES DE RÔLE : un petit texte flottant au-dessus de chaque avatar distant
//! pour lire, en toutes lettres, qui joue quel rôle dans l'archi P2P. Rendu en
//! overlay 2D (UI) : on projette la position 3D de la tête sur l'écran, et on y pose
//! un texte qui la suit.
//!
//! Texte affiché : « Joueur N » + ses rôles actifs : OWN BALLE (maître de l'orbe),
//! TUTEUR (relais actif), SOUS TUTELLE (joueur faible relayé). On n'étiquette pas
//! son PROPRE corps (pas d'avatar de soi en vue 1re personne) : on lit les rôles des
//! autres, idéalement depuis une 3e fenêtre témoin.

use super::state::{RemoteAvatar, RemoteAvatars};
use crate::net::crypto::PeerId;
use crate::net::orb::Orb;
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

/// Marque une étiquette UI (le lien id → entité est tenu par `Nameplates`).
#[derive(Component)]
pub(crate) struct Nameplate;

/// Associe chaque avatar (identité) à son étiquette UI, pour les créer/supprimer.
#[derive(Resource, Default)]
pub struct Nameplates {
    map: HashMap<PeerId, Entity>,
}

/// SYSTÈME : maintient une étiquette par avatar — la crée/supprime avec lui, la
/// positionne à l'écran au-dessus de sa tête, et y écrit ses rôles courants.
pub fn update_nameplates(
    mut commands: Commands,
    mut plates: ResMut<Nameplates>,
    orb: Res<Orb>,
    avatars: Res<RemoteAvatars>,
    camera: Query<(&Camera, &GlobalTransform)>,
    bodies: Query<(&RemoteAvatar, &GlobalTransform)>,
    mut labels: Query<(&mut Node, &mut Text, &mut Visibility), With<Nameplate>>,
) {
    let Ok((cam, cam_tf)) = camera.single() else {
        return;
    };

    // Position monde (au-dessus de la tête) de chaque avatar, par identité.
    let mut world: HashMap<PeerId, Vec3> = HashMap::new();
    for (a, gt) in bodies.iter() {
        world.insert(a.id, gt.translation() + Vec3::Y * 1.15);
    }

    // Tuteurs actifs = les parents référencés par les avatars sous tutelle.
    let tutors: HashSet<PeerId> = avatars
        .map
        .values()
        .filter_map(|p| p.parent)
        .collect();

    // 1) Retirer les étiquettes des avatars disparus OU passés en imposteur LOD (8.2c :
    //    on n'étiquette que le focus, sinon une foule de 500 = 500 labels UI illisibles).
    plates.map.retain(|id, ent| {
        let keep = avatars.map.get(id).map_or(false, |p| p.detailed);
        if !keep {
            commands.entity(*ent).despawn();
        }
        keep
    });
    // 2) Créer une étiquette pour chaque avatar DÉTAILLÉ (focus) sans étiquette.
    for (id, p) in avatars.map.iter() {
        if !p.detailed {
            continue; // 8.2c : la foule en conscience (LOD) n'est pas étiquetée
        }
        plates.map.entry(*id).or_insert_with(|| {
            commands
                .spawn((
                    Nameplate,
                    Text::new(""),
                    TextFont { font_size: 15.0, ..default() },
                    TextColor(Color::WHITE),
                    Node {
                        position_type: PositionType::Absolute,
                        ..default()
                    },
                ))
                .id()
        });
    }

    // 3) Positionner et remplir chaque étiquette.
    for (id, ent) in plates.map.iter() {
        let Ok((mut node, mut text, mut vis)) = labels.get_mut(*ent) else {
            continue;
        };
        // Projeter la tête sur l'écran (None si hors champ / derrière la caméra).
        match world
            .get(id)
            .and_then(|w| cam.world_to_viewport(cam_tf, *w).ok())
        {
            Some(screen) => {
                *vis = Visibility::Visible;
                node.left = Val::Px(screen.x);
                node.top = Val::Px(screen.y);

                let mut roles = Vec::new();
                if orb.owner == Some(*id) {
                    roles.push("OWN BALLE");
                }
                if tutors.contains(id) {
                    roles.push("TUTEUR");
                }
                if avatars.map.get(id).map_or(false, |p| p.parent.is_some()) {
                    roles.push("SOUS TUTELLE");
                }
                let nom = id.short();
                text.0 = if roles.is_empty() {
                    format!("Joueur {nom}")
                } else {
                    format!("Joueur {nom} — {}", roles.join(", "))
                };
            }
            None => *vis = Visibility::Hidden,
        }
    }
}
