//! MATH maison — un `Vec3` minimal, sans aucune dépendance moteur.
//!
//! # Pourquoi
//! Le cœur réseau ne doit dépendre d'AUCUN moteur 3D (règle du projet : la logique
//! réseau reste la même quel que soit le moteur). On avait emprunté `bevy::prelude::Vec3`
//! par commodité ; en retirant Bevy (bascule Unreal, sidecar), on remplace ce seul type
//! de maths par un équivalent maison, byte-pour-byte compatible côté wire (3 × `f32`).
//!
//! L'API reproduit FIDÈLEMENT le sous-ensemble de `bevy::Vec3` qu'on utilisait
//! (`new/ZERO/Y`, champs `x,y,z`, `length`, `distance(_squared)`, `normalize_or_zero`,
//! `clamp` composante par composante, et les opérateurs arithmétiques) — pour que la
//! migration soit un simple changement d'`use`, sans toucher à la logique.

/// Un vecteur 3D de `f32`. Même disposition mémoire et même sémantique que `bevy::Vec3`
/// pour le sous-ensemble qu'on emploie.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

// API VOLONTAIREMENT COMPLÈTE. Ce `Vec3` est la brique maths réutilisable du cœur (et du
// futur jeu Unreal) : on fournit le sous-ensemble usuel d'un vrai type vectoriel même si tout
// n'est pas encore appelé côté cœur headless (qui n'a besoin que de `distance_squared` & co).
// Le crippler serait un faux pas ; on assume donc quelques membres encore inemployés.
#[allow(dead_code)]
impl Vec3 {
    /// Le vecteur nul.
    pub const ZERO: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 0.0 };
    /// L'axe X unitaire.
    pub const X: Vec3 = Vec3 { x: 1.0, y: 0.0, z: 0.0 };
    /// L'axe Y unitaire (le « haut » dans notre repère, comme Bevy).
    pub const Y: Vec3 = Vec3 { x: 0.0, y: 1.0, z: 0.0 };
    /// L'axe Z unitaire.
    pub const Z: Vec3 = Vec3 { x: 0.0, y: 0.0, z: 1.0 };

    /// Construit un vecteur à partir de ses trois composantes.
    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Vec3 { x, y, z }
    }

    /// Produit scalaire.
    pub fn dot(self, rhs: Vec3) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    /// Longueur au carré (évite la racine quand on ne compare que des distances).
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }

    /// Longueur (norme euclidienne).
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }

    /// Distance au carré entre deux points (comme `bevy::Vec3::distance_squared`).
    pub fn distance_squared(self, rhs: Vec3) -> f32 {
        (self - rhs).length_squared()
    }

    /// Distance euclidienne entre deux points.
    pub fn distance(self, rhs: Vec3) -> f32 {
        (self - rhs).length()
    }

    /// Vecteur unitaire de même direction, ou `ZERO` si la longueur est nulle/non finie
    /// (comme `bevy::Vec3::normalize_or_zero`).
    pub fn normalize_or_zero(self) -> Vec3 {
        let len = self.length();
        if len.is_finite() && len > 0.0 {
            self / len
        } else {
            Vec3::ZERO
        }
    }

    /// Borne chaque composante entre `min` et `max` (composante par composante,
    /// comme `bevy::Vec3::clamp`).
    pub fn clamp(self, min: Vec3, max: Vec3) -> Vec3 {
        Vec3::new(
            self.x.clamp(min.x, max.x),
            self.y.clamp(min.y, max.y),
            self.z.clamp(min.z, max.z),
        )
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Vec3;
    fn mul(self, s: f32) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl std::ops::Div<f32> for Vec3 {
    type Output = Vec3;
    fn div(self, s: f32) -> Vec3 {
        Vec3::new(self.x / s, self.y / s, self.z / s)
    }
}

impl std::ops::AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Vec3) {
        *self = *self + rhs;
    }
}

impl std::ops::SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Vec3) {
        *self = *self - rhs;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_au_carre_correcte() {
        let a = Vec3::new(0.0, 0.0, 0.0);
        let b = Vec3::new(3.0, 4.0, 0.0);
        assert_eq!(a.distance_squared(b), 25.0);
        assert_eq!(a.distance(b), 5.0);
    }

    #[test]
    fn normalize_zero_reste_zero() {
        assert_eq!(Vec3::ZERO.normalize_or_zero(), Vec3::ZERO);
        let n = Vec3::new(0.0, 5.0, 0.0).normalize_or_zero();
        assert!((n.length() - 1.0).abs() < 1e-6);
        assert_eq!(n, Vec3::Y);
    }

    #[test]
    fn operateurs_arithmetiques() {
        let a = Vec3::new(1.0, 2.0, 3.0);
        let b = Vec3::new(4.0, 5.0, 6.0);
        assert_eq!(a + b, Vec3::new(5.0, 7.0, 9.0));
        assert_eq!(b - a, Vec3::new(3.0, 3.0, 3.0));
        assert_eq!(a * 2.0, Vec3::new(2.0, 4.0, 6.0));
        assert_eq!((b - a) / 3.0, Vec3::new(1.0, 1.0, 1.0));
        assert_eq!(-a, Vec3::new(-1.0, -2.0, -3.0));
    }

    #[test]
    fn clamp_composante_par_composante() {
        let v = Vec3::new(-5.0, 0.5, 10.0);
        let lo = Vec3::new(-1.0, -1.0, -1.0);
        let hi = Vec3::new(1.0, 1.0, 1.0);
        assert_eq!(v.clamp(lo, hi), Vec3::new(-1.0, 0.5, 1.0));
    }
}
