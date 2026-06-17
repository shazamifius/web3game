//! LE LIEN : `NetLink`, la ressource Bevy qui tient le réseau d'un client.
//!
//! Présente uniquement en mode multijoueur. Elle contient la prise UDP, l'adresse
//! du rendez-vous, notre identifiant (attribué par le rendez-vous → `Option`
//! tant qu'on ne l'a pas), notre couleur, et l'ANNUAIRE des autres joueurs.

use super::transport::Socket;
use super::wire::RENDEZVOUS_PORT;
use bevy::prelude::Resource;
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Resource)]
pub struct NetLink {
    pub(crate) socket: Socket,
    pub(crate) rendezvous: SocketAddr,
    pub(crate) my_id: Option<u8>, // None tant que le rendez-vous ne nous a pas répondu
    pub(crate) my_color: (f32, f32, f32),
    pub(crate) world_hue: Option<u16>, // couleur de salle donnée par le serveur (None = pas connecté)
    pub(crate) peers: HashMap<u8, SocketAddr>, // les autres joueurs : id → adresse
}

impl NetLink {
    /// Prépare le réseau d'un client : prise sur un port éphémère (choisi par
    /// l'OS), et adresse du rendez-vous local.
    pub fn new(color: (f32, f32, f32)) -> std::io::Result<NetLink> {
        let socket = Socket::bind(0)?; // 0 = l'OS choisit un port libre
        let rendezvous = SocketAddr::from(([127, 0, 0, 1], RENDEZVOUS_PORT));
        println!(
            "Client réseau : port local {}, rendez-vous {}.",
            socket.local_addr()?,
            rendezvous
        );
        Ok(NetLink {
            socket,
            rendezvous,
            my_id: None,
            my_color: color,
            world_hue: None,
            peers: HashMap::new(),
        })
    }
}
