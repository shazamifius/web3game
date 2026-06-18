//! LE MODE NAT-TEST : le hole punching en TEXTE, sans la 3D.
//!
//! Nos clients de jeu sont des fenêtres graphiques : impossibles à lancer dans un
//! `ip netns` (pas d'écran). Ce mode rejoue EXACTEMENT le même scénario réseau que
//! le jeu, mais en texte, pour qu'on puisse le faire tourner dans deux namespaces
//! réseau (deux « machines » derrière deux NAT) et VOIR les trous s'ouvrir.
//!
//! Scénario (le même que dans le jeu) :
//!   1. on s'inscrit au rendez-vous (HELLO) ; il lit notre adresse PUBLIQUE (vue
//!      après le NAT) et nous renvoie la liste des autres (WELCOME) ;
//!   2. pour chaque pair, on envoie des PUNCH en rafale : chacun ouvre, dans NOTRE
//!      box, le trou de retour vers ce pair. Les premiers meurent, les suivants
//!      passent quand le pair perce de son côté ;
//!   3. dès qu'un paquet du pair arrive → « trou OUVERT » → on s'échange des
//!      données applicatives (un petit STATE), preuve que la voie directe marche.
//!
//! Lancement (voir tools/test-nat.sh) :
//!   RENDEZVOUS_ADDR=10.0.0.1:4000 cargo run -- nat-test alice

use super::control::{decode_welcome, encode_hello};
use super::crypto::Identity;
use super::link::rendezvous_addr;
use super::message::{decode, encode, PlayerState};
use super::punch::{decode_punch, encode_punch};
use super::transport::Socket;
use super::wire::{kind, KIND_PUNCH, KIND_STATE, KIND_WELCOME};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::{Duration, Instant};

/// Cadences (s) : voir le jeu pour les équivalents (HELLO 1 s, PUNCH 0.25 s).
const HELLO_PERIOD: f32 = 1.0;
const PUNCH_PERIOD: f32 = 0.25;
const DATA_PERIOD: f32 = 0.5;
const TICK: Duration = Duration::from_millis(50);
/// Au-delà de tant d'essais sans réponse, on cesse de logguer (mais on continue) :
/// sans doute un NAT symétrique → ce sera le rôle du relais (chapitre 5).
const PUNCH_LOG_LIMIT: u32 = 6;

/// L'état d'un trou vers un pair (version texte de `punch::HoleState`).
struct Hole {
    addr: SocketAddr,
    open: bool,
    tries: u32,
    punch_acc: f32,
    data_acc: f32,
}

pub fn run_nat_test(label: &str) {
    let socket = match Socket::bind(0) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Impossible d'ouvrir la prise : {e}");
            return;
        }
    };
    let rendezvous = rendezvous_addr();
    let local = socket.local_addr().ok();
    println!("[{label}] prise locale {local:?}, rendez-vous {rendezvous}.");
    println!("[{label}] j'envoie HELLO au rendez-vous et j'attends la liste des pairs…\n");

    // Identité de ce client de test : le rendez-vous exige une clé publique dans HELLO.
    let identity = Identity::generate();
    let mut my_id: Option<u8> = None;
    let mut holes: HashMap<u8, Hole> = HashMap::new();
    let mut hello_acc = HELLO_PERIOD; // pour dire HELLO dès le premier tour
    let start = Instant::now();
    let mut last = Instant::now();

    loop {
        let dt = last.elapsed().as_secs_f32();
        last = Instant::now();

        // 1) Battement HELLO vers le rendez-vous (position bidon : pas de 3D ici).
        hello_acc += dt;
        if hello_acc >= HELLO_PERIOD {
            hello_acc = 0.0;
            let _ = socket.send_to(rendezvous, &encode_hello(0.0, 0.0, &identity.public()));
        }

        // 2) On relève le courrier.
        for (_from, bytes) in socket.poll() {
            match kind(&bytes) {
                Some(KIND_WELCOME) => {
                    if let Some((id, _hue, roster)) = decode_welcome(&bytes) {
                        if my_id != Some(id) {
                            my_id = Some(id);
                            println!("[{label}] le rendez-vous m'a donné l'identifiant {id}.");
                        }
                        // On (re)synchronise la liste des pairs à percer.
                        for (pid, addr, _pubkey) in roster {
                            holes.entry(pid).or_insert(Hole {
                                addr,
                                open: false,
                                tries: 0,
                                punch_acc: PUNCH_PERIOD,
                                data_acc: 0.0,
                            });
                        }
                    }
                }
                Some(KIND_PUNCH) => {
                    if let Some(pid) = decode_punch(&bytes) {
                        if let Some(h) = holes.get_mut(&pid) {
                            if !h.open {
                                h.open = true;
                                let s = start.elapsed().as_secs_f32();
                                println!(
                                    "[{label}] ✅ trou OUVERT avec le pair {pid} à t={s:.2}s — connexion DIRECTE établie !"
                                );
                            }
                        }
                    }
                }
                Some(KIND_STATE) => {
                    if let Some(st) = decode(&bytes) {
                        if let Some(h) = holes.get_mut(&st.id) {
                            h.open = true; // recevoir des données prouve le trou ouvert
                        }
                        println!("[{label}]    ← données reçues du pair {} (la voie directe fonctionne).", st.id);
                    }
                }
                _ => {}
            }
        }

        // 3) Pour chaque pair : percer tant que c'est fermé, sinon échanger des données.
        let me = my_id;
        for (pid, h) in holes.iter_mut() {
            let Some(id) = me else { break };
            if !h.open {
                h.punch_acc += dt;
                if h.punch_acc >= PUNCH_PERIOD {
                    h.punch_acc = 0.0;
                    h.tries += 1;
                    let _ = socket.send_to(h.addr, &encode_punch(id));
                    if h.tries <= PUNCH_LOG_LIMIT {
                        println!("[{label}] PUNCH vers le pair {pid} (essai {}) — j'ouvre mon trou de retour.", h.tries);
                        if h.tries == PUNCH_LOG_LIMIT {
                            println!("[{label}] pair {pid} : pas de réponse ; on continue en silence (NAT symétrique ? → relais au chap. 5).");
                        }
                    }
                }
            } else {
                // Trou ouvert : on envoie un petit STATE pour prouver que ça circule.
                h.data_acc += dt;
                if h.data_acc >= DATA_PERIOD {
                    h.data_acc = 0.0;
                    let ping = PlayerState {
                        id,
                        x: 0.0, y: 0.7, z: 0.0,
                        vx: 0.0, vy: 0.0, vz: 0.0,
                        yaw: 0.0, pitch: 0.0,
                        r: 0.5, g: 0.5, b: 0.5,
                        parent: 0, seq: 0,
                    };
                    let _ = socket.send_to(h.addr, &encode(&ping));
                }
            }
        }

        std::thread::sleep(TICK);
    }
}
