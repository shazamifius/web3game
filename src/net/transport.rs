//! LA CONNEXION : une prise réseau générique, faite main.
//!
//! C'est la seule partie qui parle vraiment au système. Elle ne sait rien des
//! messages : elle envoie des octets à une adresse, et relève tous les octets
//! reçus (avec l'adresse de l'expéditeur). Le tri se fait au-dessus (`wire`).
//!
//! # Deux backends (dette D25, ajouté le 20 juin)
//! - **`Udp`** : la VRAIE prise UDP, inchangée — c'est elle que le jeu et `sim` utilisent.
//! - **`Bus`** : un BUS MÉMOIRE synchrone, pour le banc léger (`coopsim`) à très grande échelle.
//!   Au lieu de passer par l'OS, `send_to` dépose les octets dans la boîte du destinataire d'un
//!   routeur PARTAGÉ, et `poll` vide la sienne. Livraison INSTANTANÉE, sans perte, dans l'ordre →
//!   le temps de simulation se découple du temps mural (on peut avancer à dt FIXE, sans `sleep`),
//!   ce que l'UDP réel interdisait (cf. l'échec de fidélité T0.2). Le chemin `Udp` est byte-pour-byte
//!   INCHANGÉ : ajouter le bus ne peut pas casser le jeu ni `sim` (prouvé par les 75 tests + `sim`).
//!
//! ⚠ BUS_DOUTE — le bus modélise un réseau PARFAIT (0 latence, 0 perte, ordre strict). Il sert à
//! mesurer l'ÉCHELLE (perception/débit ∝ N), PAS le réalisme réseau (latence/jitter/reorder/perte
//! = c'est le rôle de `sim` + `tc netem`). Ne JAMAIS tirer de conclusion « réseau réel » d'un run bus.

use std::collections::{HashMap, VecDeque};
use std::net::{SocketAddr, UdpSocket};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Le ROUTEUR du bus mémoire : une boîte aux lettres (file de paquets `(expéditeur, octets)`) par
/// adresse d'endpoint. Partagé entre tous les endpoints d'un même banc via `Arc<Mutex<…>>`.
/// ⚠ BUS_DOUTE — ORIENTATION : on prend `Arc<Mutex>` (et non `Rc<RefCell>`, plus léger mono-thread)
/// UNIQUEMENT pour que `Socket` reste `Send + Sync` (la prise peut être partagée entre threads, ex. le sidecar au cœur continu).
/// `coopsim` est mono-thread → le mutex n'est jamais contendu, le coût est négligeable. À revoir si on
/// voulait un jour un bus multi-thread (là le mutex deviendrait un vrai point de contention).
#[derive(Default)]
pub(crate) struct BusRouter {
    mailboxes: HashMap<SocketAddr, VecDeque<(SocketAddr, Vec<u8>)>>,
}

/// Un handle de bus partagé : `Socket::bus(addr, bus.clone())` crée un endpoint à `addr` dessus.
pub(crate) type Bus = Arc<Mutex<BusRouter>>;

/// Crée un bus mémoire vide, prêt à accueillir des endpoints (`Socket::bus`).
pub(crate) fn new_bus() -> Bus {
    Arc::new(Mutex::new(BusRouter::default()))
}

/// Le backend d'une `Socket` : soit la vraie prise UDP, soit un endpoint sur un bus mémoire.
enum Backend {
    Udp(UdpSocket),
    /// Endpoint de bus : notre propre adresse (l'expéditeur qu'on inscrit dans nos envois) + le
    /// routeur partagé où l'on dépose/relève les paquets.
    Bus { addr: SocketAddr, router: Bus },
}

/// Une prise réseau non-bloquante (UDP réel) OU un endpoint de bus mémoire.
///
/// Elle compte aussi (chap. 7.4) le total d'octets ÉMIS et REÇUS, pour mesurer la bande passante
/// RÉELLE par nœud — la métrique qui décide si le protocole passe à 55 000 joueurs (en P2P le goulot
/// est l'upload par nœud, pas le CPU). Compteurs atomiques : `send_to`/`poll` ne prennent que
/// `&self`, et la prise peut être partagée entre threads. Le comptage est IDENTIQUE sur
/// les deux backends (octets de charge utile) → le débit mesuré sur bus est comparable à l'UDP.
pub(crate) struct Socket {
    backend: Backend,
    bytes_sent: AtomicU64,
    bytes_recv: AtomicU64,
}

impl Socket {
    /// Ouvre la prise UDP sur le port donné. `port = 0` → l'OS en choisit un libre
    /// (« port éphémère ») : pratique quand on lance plein de clients.
    ///
    /// On écoute sur `0.0.0.0` (toutes les interfaces) et pas seulement
    /// `127.0.0.1` : indispensable pour que des « machines » différentes (ex.
    /// namespaces réseau du test NAT) puissent nous joindre. — CHEMIN INCHANGÉ.
    pub(crate) fn bind(port: u16) -> std::io::Result<Socket> {
        let socket = UdpSocket::bind(("0.0.0.0", port))?;
        // Mode non-bloquant : lire le réseau ne met JAMAIS le jeu en pause.
        socket.set_nonblocking(true)?;
        Ok(Socket {
            backend: Backend::Udp(socket),
            bytes_sent: AtomicU64::new(0),
            bytes_recv: AtomicU64::new(0),
        })
    }

    /// Crée un endpoint de BUS MÉMOIRE à l'adresse `addr` sur le routeur `router` (dette D25).
    /// Inscrit notre boîte aux lettres dans le routeur. Sert UNIQUEMENT au banc léger `coopsim`.
    /// ⚠ BUS_DOUTE — `addr` est une adresse SYNTHÉTIQUE (le banc l'attribue, ex. 127.0.0.1:port) :
    /// elle n'a pas besoin d'être routable, elle sert juste de CLÉ d'aiguillage dans le routeur.
    pub(crate) fn bus(addr: SocketAddr, router: Bus) -> Socket {
        router.lock().unwrap().mailboxes.entry(addr).or_default();
        Socket {
            backend: Backend::Bus { addr, router },
            bytes_sent: AtomicU64::new(0),
            bytes_recv: AtomicU64::new(0),
        }
    }

    /// Total d'octets ÉMIS depuis l'ouverture de la prise (chap. 7.4).
    pub(crate) fn bytes_sent(&self) -> u64 {
        self.bytes_sent.load(Ordering::Relaxed)
    }

    /// Total d'octets REÇUS depuis l'ouverture de la prise (chap. 7.4).
    pub(crate) fn bytes_recv(&self) -> u64 {
        self.bytes_recv.load(Ordering::Relaxed)
    }

    /// L'adresse locale réellement obtenue (utile quand on a demandé le port 0 ; sur bus, `addr`).
    pub(crate) fn local_addr(&self) -> std::io::Result<SocketAddr> {
        match &self.backend {
            Backend::Udp(s) => s.local_addr(),
            Backend::Bus { addr, .. } => Ok(*addr),
        }
    }

    /// Envoie un paquet d'octets à une adresse. Aucun accusé de réception.
    /// - `Udp` : `sendto` réel (CHEMIN INCHANGÉ).
    /// - `Bus` : dépose `(notre addr, octets)` dans la boîte du destinataire (livraison instantanée).
    ///   ⚠ BUS_DOUTE — si le destinataire n'existe pas dans le routeur, le paquet est SILENCIEUSEMENT
    ///   perdu (comme un datagramme UDP vers une adresse morte). On NE crée PAS sa boîte à la volée
    ///   (sinon un envoi vers une adresse bidon ferait enfler le routeur — pendant du `MAX_KNOWN`).
    pub(crate) fn send_to(&self, addr: SocketAddr, bytes: &[u8]) -> std::io::Result<()> {
        match &self.backend {
            Backend::Udp(s) => {
                let n = s.send_to(bytes, addr)?;
                self.bytes_sent.fetch_add(n as u64, Ordering::Relaxed);
            }
            Backend::Bus { addr: me, router } => {
                let mut r = router.lock().unwrap();
                if let Some(box_) = r.mailboxes.get_mut(&addr) {
                    box_.push_back((*me, bytes.to_vec()));
                    self.bytes_sent.fetch_add(bytes.len() as u64, Ordering::Relaxed);
                }
                // destinataire inconnu → paquet perdu (pas d'erreur, comme l'UDP)
            }
        }
        Ok(())
    }

    /// Relève TOUS les paquets arrivés depuis le dernier appel, avec l'adresse de l'expéditeur.
    /// Ne bloque jamais.
    /// - `Udp` : draine la prise réelle (CHEMIN INCHANGÉ).
    /// - `Bus` : vide notre boîte aux lettres dans le routeur partagé.
    pub(crate) fn poll(&self) -> Vec<(SocketAddr, Vec<u8>)> {
        match &self.backend {
            Backend::Udp(s) => {
                let mut received = Vec::new();
                // Tampon de lecture. 2 Ko : large pour nos paquets (état signé = 182 o) et pour un
                // WELCOME qui transporte les clés publiques. À très grande échelle, le WELCOME devra
                // être découpé en morceaux (chantier futur).
                let mut buf = [0u8; 2048];
                loop {
                    match s.recv_from(&mut buf) {
                        Ok((n, from)) => {
                            self.bytes_recv.fetch_add(n as u64, Ordering::Relaxed);
                            received.push((from, buf[..n].to_vec()));
                        }
                        // `WouldBlock` = boîte vide pour l'instant : ce n'est pas une erreur.
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                        Err(_) => break,
                    }
                }
                received
            }
            Backend::Bus { addr, router } => {
                let mut r = router.lock().unwrap();
                let drained: Vec<(SocketAddr, Vec<u8>)> = match r.mailboxes.get_mut(addr) {
                    Some(box_) => box_.drain(..).collect(),
                    None => Vec::new(),
                };
                let total: usize = drained.iter().map(|(_, b)| b.len()).sum();
                self.bytes_recv.fetch_add(total as u64, Ordering::Relaxed);
                drained
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([127, 0, 0, 1], port))
    }

    /// Le bus mémoire livre un paquet d'un endpoint à un autre, avec le bon expéditeur, et compte
    /// les octets des deux côtés (débit comparable à l'UDP). Aller-retour A→B puis B→A.
    #[test]
    fn bus_livre_entre_deux_endpoints() {
        let bus = new_bus();
        let a = Socket::bus(addr(1), bus.clone());
        let b = Socket::bus(addr(2), bus.clone());

        a.send_to(addr(2), b"coucou").unwrap();
        // Avant de relever, A n'a rien reçu ; B a un paquet en attente.
        assert!(a.poll().is_empty());
        let recu = b.poll();
        assert_eq!(recu, vec![(addr(1), b"coucou".to_vec())]); // (expéditeur A, charge)
        assert_eq!(a.bytes_sent(), 6);
        assert_eq!(b.bytes_recv(), 6);
        // Une 2e relève ne redonne rien (boîte vidée).
        assert!(b.poll().is_empty());

        // Réponse B→A.
        b.send_to(addr(1), b"salut").unwrap();
        assert_eq!(a.poll(), vec![(addr(2), b"salut".to_vec())]);
    }

    /// Un envoi vers une adresse INCONNUE du routeur est silencieusement perdu (comme l'UDP vers une
    /// adresse morte) et ne compte pas d'octets émis — il ne fait pas non plus enfler le routeur.
    #[test]
    fn bus_perd_silencieusement_vers_une_adresse_inconnue() {
        let bus = new_bus();
        let a = Socket::bus(addr(1), bus.clone());
        a.send_to(addr(999), b"dans le vide").unwrap(); // 999 jamais enregistrée
        assert_eq!(a.bytes_sent(), 0); // rien émis (destinataire inexistant)
        assert_eq!(bus.lock().unwrap().mailboxes.len(), 1); // seule la boîte de A existe
    }
}
