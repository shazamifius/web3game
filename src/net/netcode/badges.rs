//! BADGES DE RÔLE : un repère néon flottant au-dessus de chaque avatar distant,
//! pour VOIR d'un coup d'œil qui joue quel rôle dans l'archi P2P. Sans lui, ces
//! rôles sont invisibles à l'œil.
//!
//!   🟡 jaune  = maître de l'orbe (Own de la balle)
//!   🟢 vert   = tuteur (relais : il recopie pour un joueur à faible upload)
//!   🟠 orange = sous tutelle (joueur faible qui passe par un parent)
//!
//! Un avatar peut cumuler plusieurs badges (ils s'empilent en hauteur). On ne badge
//! pas SON PROPRE corps (pas d'avatar de soi en vue 1re personne) : on lit les rôles
//! sur les autres — ce qui suffit pour observer la mécanique depuis une 3e fenêtre.

use super::state::RemoteAvatars;
use crate::net::orb::Orb;
use bevy::prelude::*;
use std::collections::HashSet;

/// Badge « maître de l'orbe » d'un avatar (porte l'id de l'avatar qu'il surmonte).
#[derive(Component)]
pub(crate) struct BadgeOwn(pub(crate) u8);
/// Badge « tuteur » (ce joueur relaie pour au moins un joueur faible).
#[derive(Component)]
pub(crate) struct BadgeTutor(pub(crate) u8);
/// Badge « sous tutelle » (ce joueur faible passe par un parent).
#[derive(Component)]
pub(crate) struct BadgeWard(pub(crate) u8);

/// SYSTÈME : allume/éteint chaque badge selon le rôle COURANT de son avatar.
/// `ParamSet` car les trois requêtes touchent toutes `Visibility` en mutable : Bevy
/// exige qu'on n'en active qu'une à la fois (elles visent des entités disjointes).
pub fn update_role_badges(
    orb: Res<Orb>,
    avatars: Res<RemoteAvatars>,
    mut badges: ParamSet<(
        Query<(&BadgeOwn, &mut Visibility)>,
        Query<(&BadgeTutor, &mut Visibility)>,
        Query<(&BadgeWard, &mut Visibility)>,
    )>,
) {
    // Tuteurs actifs = les parents référencés par les avatars sous tutelle.
    let tutors: HashSet<u8> = avatars
        .map
        .values()
        .map(|p| p.parent)
        .filter(|&id| id != 0)
        .collect();
    let owner = orb.owner;

    for (b, mut vis) in badges.p0().iter_mut() {
        *vis = show(owner == Some(b.0));
    }
    for (b, mut vis) in badges.p1().iter_mut() {
        *vis = show(tutors.contains(&b.0));
    }
    for (b, mut vis) in badges.p2().iter_mut() {
        let ward = avatars.map.get(&b.0).map_or(false, |p| p.parent != 0);
        *vis = show(ward);
    }
}

fn show(on: bool) -> Visibility {
    if on {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

/// Matériau d'un badge : noir + émissif fort (il « glow » au bloom de la caméra).
/// Les trois teintes : 🟠 orange (tutelle), 🟡 jaune (orbe), 🟢 vert (tuteur).
pub(crate) fn badge_mat(r: f32, g: f32, b: f32) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::BLACK,
        emissive: LinearRgba::rgb(r, g, b),
        ..default()
    }
}
