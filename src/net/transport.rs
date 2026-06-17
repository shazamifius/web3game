//! LA CONNEXION : une prise réseau UDP générique, faite main.
//!
//! C'est la seule partie qui parle vraiment au système. Elle ne sait rien des
//! messages : elle envoie des octets à une adresse, et relève tous les octets
//! reçus (avec l'adresse de l'expéditeur). Le tri se fait au-dessus (`wire`).

use std::net::{SocketAddr, UdpSocket};

/// Une prise UDP non-bloquante, sur toutes les interfaces (0.0.0.0).
pub(crate) struct Socket {
    socket: UdpSocket,
}

impl Socket {
    /// Ouvre la prise sur le port donné. `port = 0` → l'OS en choisit un libre
    /// (« port éphémère ») : pratique quand on lance plein de clients.
    ///
    /// On écoute sur `0.0.0.0` (toutes les interfaces) et pas seulement
    /// `127.0.0.1` : indispensable pour que des « machines » différentes (ex.
    /// namespaces réseau du test NAT) puissent nous joindre.
    pub(crate) fn bind(port: u16) -> std::io::Result<Socket> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        // Mode non-bloquant : lire le réseau ne met JAMAIS le jeu en pause.
        socket.set_nonblocking(true)?;
        Ok(Socket { socket })
    }

    /// L'adresse locale réellement obtenue (utile quand on a demandé le port 0).
    pub(crate) fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    /// Envoie un paquet d'octets à une adresse. Aucun accusé de réception (UDP).
    pub(crate) fn send_to(&self, addr: SocketAddr, bytes: &[u8]) -> std::io::Result<()> {
        self.socket.send_to(bytes, addr)?;
        Ok(())
    }

    /// Relève TOUS les paquets arrivés depuis le dernier appel, avec l'adresse de
    /// l'expéditeur. Ne bloque jamais.
    pub(crate) fn poll(&self) -> Vec<(SocketAddr, Vec<u8>)> {
        let mut received = Vec::new();
        let mut buf = [0u8; 1024];
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((n, from)) => received.push((from, buf[..n].to_vec())),
                // `WouldBlock` = boîte vide pour l'instant : ce n'est pas une erreur.
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
        received
    }
}
