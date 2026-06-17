//! LA CRYPTO : signatures à clé publique (Ed25519) — la frontière de confiance.
//!
//! # Pourquoi ce fichier est spécial
//! C'est le SEUL endroit du projet qui s'appuie sur une bibliothèque externe
//! (`ed25519-dalek`). Partout ailleurs on code tout à la main pour comprendre
//! chaque octet ; ici, NON, et c'est volontaire : on ne code JAMAIS sa propre
//! crypto. L'arithmétique sur courbe elliptique est un nid à failles subtiles
//! (canaux auxiliaires, valeurs dégénérées…) que même des experts ratent. On
//! délègue donc le CALCUL de la signature, et on garde la main sur tout le reste
//! (le format de l'enveloppe, la distribution des clés, la règle de vérification).
//!
//! # L'idée de la signature à clé publique (le cœur du chapitre 5)
//!   - chaque joueur possède une PAIRE de clés : une privée (secrète, gardée pour
//!     soi) et une publique (partagée, c'est son IDENTITÉ) ;
//!   - SIGNER un message avec la clé privée produit une « signature » de 64 octets
//!     que SEUL le détenteur de la clé privée pouvait calculer ;
//!   - VÉRIFIER cette signature avec la clé publique prouve deux choses d'un coup :
//!       1) le message vient bien du détenteur de la clé (authenticité) ;
//!       2) il n'a pas été modifié d'un seul octet (intégrité).
//! N'importe qui peut vérifier ; personne ne peut forger sans la clé privée.
//! C'est exactement l'« enveloppe scellée » qu'on voulait : un relais peut porter
//! l'enveloppe, mais pas en changer le contenu sans casser le sceau.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::io::Read;

/// Taille d'une clé publique Ed25519 (octets) : c'est l'identité d'un joueur.
pub(crate) const PUBKEY_LEN: usize = 32;
/// Taille d'une signature Ed25519 (octets) : le « sceau » apposé sur un paquet.
pub(crate) const SIG_LEN: usize = 64;

/// L'identité cryptographique d'UNE session : sa paire de clés. La privée ne
/// quitte JAMAIS la mémoire de ce processus ; on ne publie que la publique.
pub(crate) struct Identity {
    signing: SigningKey,
}

impl Identity {
    /// Tire une nouvelle paire de clés au hasard. La graine (32 octets) vient du
    /// générateur d'aléa du système d'exploitation (`/dev/urandom`) : c'est LUI la
    /// source de hasard, on ne « fabrique » pas l'aléa nous-mêmes (autre règle d'or).
    pub(crate) fn generate() -> Identity {
        let seed = os_random_seed();
        Identity { signing: SigningKey::from_bytes(&seed) }
    }

    /// Notre clé PUBLIQUE (notre identité), prête à être envoyée sur le réseau.
    pub(crate) fn public(&self) -> [u8; PUBKEY_LEN] {
        self.signing.verifying_key().to_bytes()
    }

    /// Appose notre sceau sur un message : 64 octets que seul le détenteur de
    /// notre clé privée pouvait produire pour CES octets précis.
    pub(crate) fn sign(&self, message: &[u8]) -> [u8; SIG_LEN] {
        self.signing.sign(message).to_bytes()
    }
}

/// Vérifie qu'une signature correspond bien à (ce message, cette clé publique).
/// Renvoie `false` au moindre doute : clé publique invalide, sceau qui ne colle
/// pas, message altéré. On ne fait JAMAIS confiance par défaut.
pub(crate) fn verify(message: &[u8], sig: &[u8; SIG_LEN], pubkey: &[u8; PUBKEY_LEN]) -> bool {
    let Ok(verifying_key) = VerifyingKey::from_bytes(pubkey) else {
        return false; // clé publique mal formée → on rejette
    };
    let signature = Signature::from_bytes(sig);
    verifying_key.verify(message, &signature).is_ok()
}

/// Lit 32 octets d'aléa cryptographique depuis le système (`/dev/urandom`).
/// On évite ainsi une dépendance de plus (`rand`) : la seule chose qu'on demande
/// au monde extérieur, c'est de l'entropie — et l'OS est fait pour ça.
fn os_random_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut seed))
        .expect("impossible de lire /dev/urandom pour générer les clés");
    seed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_valide_est_acceptee() {
        let id = Identity::generate();
        let pubkey = id.public();
        let msg = b"position du joueur 7";
        let sig = id.sign(msg);
        assert!(verify(msg, &sig, &pubkey));
    }

    #[test]
    fn message_altere_est_rejete() {
        let id = Identity::generate();
        let pubkey = id.public();
        let sig = id.sign(b"je suis a la position A");
        // Le moindre octet changé → le sceau ne colle plus.
        assert!(!verify(b"je suis a la position B", &sig, &pubkey));
    }

    #[test]
    fn signature_d_un_autre_est_rejetee() {
        let moi = Identity::generate();
        let autre = Identity::generate();
        let msg = b"transfert de l'orbe";
        let sig = autre.sign(msg); // signé par QUELQU'UN D'AUTRE
        // Vérifiée contre MA clé publique → refusée : pas d'usurpation possible.
        assert!(!verify(msg, &sig, &moi.public()));
    }
}
