//! LE LIEN : `NetLink`, la ressource Bevy qui tient le réseau d'un client.
//!
//! Présente uniquement en mode multijoueur. Elle contient la prise UDP, l'adresse
//! du rendez-vous, notre identifiant (attribué par le rendez-vous → `Option`
//! tant qu'on ne l'a pas), notre couleur, et l'ANNUAIRE des autres joueurs.

use super::crypto::{Identity, PeerId};
use super::transport::Socket;
use super::wire::RENDEZVOUS_PORT;
use bevy::prelude::Resource;
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Resource)]
pub struct NetLink {
    pub(crate) socket: Socket,
    pub(crate) rendezvous: SocketAddr,
    /// Notre identité : `None` tant que le rendez-vous ne nous a pas (encore) répondu
    /// (sert juste de garde « suis-je inscrit ? »). Dès le 1er WELCOME, on y met notre
    /// propre clé (`identity.id()`) : depuis le chap. 6.1, l'identité n'est plus un
    /// numéro attribué — c'est notre clé, qu'on connaît dès le départ.
    pub(crate) my_id: Option<PeerId>,
    pub(crate) my_color: (f32, f32, f32),
    /// Notre identité cryptographique (paire de clés). On SIGNE nos paquets avec ;
    /// notre clé publique EST notre identité, portée dans chaque paquet.
    pub(crate) identity: Identity,
    pub(crate) world_hue: Option<u16>, // couleur de salle donnée par le serveur (None = pas connecté)
    pub(crate) peers: HashMap<PeerId, SocketAddr>, // les autres joueurs : identité → adresse
    /// ANTI-REJEU (chap. 5.2) : dernier numéro de séquence accepté par pair. On
    /// refuse tout paquet de `seq` ≤ : un vieux paquet rejoué ne passe plus.
    pub(crate) last_seq: HashMap<PeerId, u64>,
    /// RÉPUTATION (chap. 5.4) : nombre de « fautes » constatées par pair (état
    /// impossible, orbe trichée…). Au-delà de `MAX_STRIKES`, le pair est mis en
    /// sourdine (« mute ») : on ignore tout ce qu'il envoie.
    pub(crate) strikes: HashMap<PeerId, u32>,
    pub(crate) weak: bool, // faible upload : on émet notre état via un parent (relais) au lieu de tous
}

/// Nombre de fautes au-delà duquel on coupe le son d'un pair (réputation). Chaque
/// nœud est ainsi le « Shield » de ce qu'il observe : il détecte et bannit localement.
pub(crate) const MAX_STRIKES: u32 = 5;

impl NetLink {
    /// Prépare le réseau d'un client : prise sur un port éphémère (choisi par
    /// l'OS), et adresse du rendez-vous local. `weak` = mode « faible upload » :
    /// on n'émet plus son état à tous les pairs, mais une seule fois à un parent
    /// (relais) qui le recopie à notre place.
    pub fn new(color: (f32, f32, f32), weak: bool) -> std::io::Result<NetLink> {
        let socket = Socket::bind(0)?; // 0 = l'OS choisit un port libre
        let rendezvous = rendezvous_addr();
        // On tire notre paire de clés une fois, au lancement. La privée reste ici.
        let identity = Identity::generate();
        println!(
            "Client réseau : port local {}, rendez-vous {}{}.",
            socket.local_addr()?,
            rendezvous,
            if weak { " (faible upload : via un parent)" } else { "" }
        );
        Ok(NetLink {
            socket,
            rendezvous,
            my_id: None,
            my_color: color,
            identity,
            world_hue: None,
            peers: HashMap::new(),
            last_seq: HashMap::new(),
            strikes: HashMap::new(),
            weak,
        })
    }

    /// Ce pair est-il en sourdine (trop de fautes) ? Si oui, on ignore ses paquets.
    pub(crate) fn is_muted(&self, id: PeerId) -> bool {
        self.strikes.get(&id).copied().unwrap_or(0) >= MAX_STRIKES
    }

    /// Inscrit une faute au compteur d'un pair et la journalise. Quand le seuil est
    /// franchi, on l'annonce : le pair est désormais ignoré (banni localement).
    pub(crate) fn add_strike(&mut self, id: PeerId, reason: &str) {
        let n = self.strikes.entry(id).or_insert(0);
        *n += 1;
        if *n == MAX_STRIKES {
            eprintln!("🛡 Pair {} mis en SOURDINE après {n} fautes (dernière : {reason}).", id.short());
        } else if *n < MAX_STRIKES {
            eprintln!("🛡 Faute de {} ({n}/{MAX_STRIKES}) : {reason}.", id.short());
        }
    }

    /// ANTI-REJEU : accepte un `seq` seulement s'il est STRICTEMENT plus grand que le
    /// dernier vu de ce pair (et le mémorise). Un rejeu (seq déjà vu ou plus ancien)
    /// renvoie `false`. Le premier paquet d'un pair (aucun seq mémorisé) est accepté.
    pub(crate) fn accept_seq(&mut self, id: PeerId, seq: u64) -> bool {
        match self.last_seq.get(&id) {
            Some(&last) if seq <= last => false, // rejeu ou paquet périmé
            _ => {
                self.last_seq.insert(id, seq);
                true
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn link_de_test() -> NetLink {
        NetLink::new((1.0, 1.0, 1.0), false).expect("socket de test")
    }

    fn pid(seed: u8) -> PeerId {
        PeerId::from_bytes([seed; 32])
    }

    /// ANTI-REJEU : on accepte des seq croissants, on refuse tout rejeu / paquet périmé.
    #[test]
    fn accept_seq_refuse_les_rejeus() {
        let mut link = link_de_test();
        let a = pid(3);
        assert!(link.accept_seq(a, 1)); // premier paquet de a : accepté
        assert!(link.accept_seq(a, 2)); // strictement croissant : accepté
        assert!(!link.accept_seq(a, 2)); // même seq rejoué : refusé
        assert!(!link.accept_seq(a, 1)); // seq plus ancien : refusé
        assert!(link.accept_seq(a, 5)); // on saute en avant : accepté
        // Un AUTRE pair a son propre compteur, indépendant.
        assert!(link.accept_seq(pid(4), 1));
    }

    /// RÉPUTATION : au bout de MAX_STRIKES fautes, le pair passe en sourdine.
    #[test]
    fn mute_apres_max_strikes() {
        let mut link = link_de_test();
        let tricheur = pid(9);
        assert!(!link.is_muted(tricheur));
        for _ in 0..MAX_STRIKES {
            link.add_strike(tricheur, "test");
        }
        assert!(link.is_muted(tricheur)); // banni localement
        // Un pair sans faute reste audible.
        assert!(!link.is_muted(pid(10)));
    }
}

/// Adresse du rendez-vous. Par défaut `127.0.0.1:4000` (tout sur le même PC) ;
/// surchargée par la variable d'environnement `RENDEZVOUS_ADDR` (ex.
/// `10.0.0.1:4000`) pour le test NAT en namespaces ou un vrai serveur distant.
pub(crate) fn rendezvous_addr() -> SocketAddr {
    if let Ok(s) = std::env::var("RENDEZVOUS_ADDR") {
        if let Ok(addr) = s.parse::<SocketAddr>() {
            return addr;
        }
        eprintln!("RENDEZVOUS_ADDR='{s}' illisible ; on retombe sur 127.0.0.1.");
    }
    SocketAddr::from(([127, 0, 0, 1], RENDEZVOUS_PORT))
}
