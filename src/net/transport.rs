//! LA CONNEXION : la prise réseau d'une session (une boîte aux lettres UDP).
//!
//! C'est la seule partie qui parle vraiment au système d'exploitation. Tout est
//! en UDP brut, non-bloquant, sur 127.0.0.1 (le même PC pour l'instant).

use super::message::{decode, encode, PlayerState};
use std::net::{SocketAddr, UdpSocket};

/// La connexion réseau d'UNE session : sa boîte aux lettres UDP (`socket`)
/// et l'adresse de l'autre joueur (`remote`).
pub(crate) struct NetPeer {
    socket: UdpSocket,
    remote: SocketAddr,
}

impl NetPeer {
    /// Ouvre la prise locale et mémorise à qui on parle (tout sur 127.0.0.1).
    pub(crate) fn bind(local_port: u16, remote_port: u16) -> std::io::Result<NetPeer> {
        let socket = UdpSocket::bind(("127.0.0.1", local_port))?;
        // Mode non-bloquant : lire le réseau ne met JAMAIS le jeu en pause.
        // « Y a-t-il du courrier ? Non ? Tant pis, on continue. »
        socket.set_nonblocking(true)?;
        let remote = SocketAddr::from(([127, 0, 0, 1], remote_port));
        Ok(NetPeer { socket, remote })
    }

    /// Envoie notre position. Un seul paquet, aucun accusé de réception (c'est l'UDP).
    pub(crate) fn send(&self, state: &PlayerState) -> std::io::Result<()> {
        self.socket.send_to(&encode(state), self.remote)?;
        Ok(())
    }

    /// Relève TOUS les paquets arrivés depuis le dernier appel. Ne bloque jamais.
    pub(crate) fn poll(&self) -> Vec<PlayerState> {
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

/// Selon le rôle ('a' ou 'b'), choisit les ports et l'identifiant.
/// 'a' écoute sur 5000 et parle à 5001 ; 'b' fait l'inverse.
pub(crate) fn ports_for_role(role: &str) -> (u16, u16, u8) {
    match role {
        "b" | "B" => (5001, 5000, 2),
        _ => (5000, 5001, 1), // 'a' par défaut
    }
}
