//! LE LIEN : `NetLink`, la ressource Bevy qui tient la connexion d'une session.
//!
//! Présente uniquement en mode multijoueur (`cargo run -- a` / `b`). C'est par
//! elle que les systèmes du `netcode` envoient et reçoivent.

use super::transport::{ports_for_role, NetPeer};
use bevy::prelude::Resource;

#[derive(Resource)]
pub struct NetLink {
    // Champs accessibles au reste du module réseau (les systèmes du netcode).
    pub(crate) peer: NetPeer,
    pub(crate) my_id: u8,
    pub(crate) my_color: (f32, f32, f32),
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
