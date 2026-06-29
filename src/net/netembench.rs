//! BANC RÉEL DE LA REDONDANCE SUR LIEN LOSSY (Phase 3) — le pendant RÉEL de `lossbench` (`phase1`).
//!
//! `phase1` prouve le MODÈLE `p^k` en mémoire (la perte est un RNG xorshift). Ici on remplace les DEUX
//! approximations par du vrai :
//!   • la perte = un VRAI qdisc `netem` du kernel (posé hors du binaire, cf. `tools/netem-bench.sh`) ;
//!   • le mécanisme = le VRAI code de production (`encode_state_bundle`/`decode_state_bundle`, taille
//!     d'état réelle `SIGNED_STATE_SIZE`), à travers deux vrais sockets UDP sur `lo`.
//! Un émetteur envoie à chaque tick le bundle de ses K DERNIERS états ; un récepteur mesure la perte
//! RÉSIDUELLE (états jamais reçus). Sur un lien à perte ALÉATOIRE p, ce résiduel doit suivre ≈ `p^K`
//! → c'est le GAIN de la redondance, prouvé sur du VRAI réseau (et non plus seulement en simulation).
//!
//! Pourquoi pas `relay-loss` ? Ce banc-là fait churner le perçage relais sur `lo` (instable) et INJECTE
//! la perte dans le code (`RELAY_DROP_PCT`). Ici, zéro perçage (émission UDP directe) et la perte vient
//! du réseau réel → on isole exactement la question Phase 3 : « K copies divisent-elles la perte par p ? ».
//!
//! Lancement (la perte est posée par le SCRIPT, hors du binaire — AUCUN privilège dans ce code) :
//!   tools/netem-bench.sh 30        # netns rootless + `netem loss 30%` sur lo, puis ce banc
//! ou à la main, dans un netns où `lo` porte déjà du netem :
//!   jeu netem-bench [loss_nominal%] [n_etats] [rate_hz]
//!
//! NB d'honnêteté : les « états » sont factices mais de TAILLE RÉELLE (`SIGNED_STATE_SIZE` o) avec leur
//! seq encodé en tête — on mesure ici le TRANSPORT du bundle (perte/redondance), pas la crypto (qui est
//! signée et testée à part dans `message.rs`). Un datagramme UDP perdu = tout le bundle perdu : exactement
//! le modèle de `phase1`, mais avec de vrais octets et une vraie perte.

use std::net::UdpSocket;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use super::message::{decode_state_bundle, encode_state_bundle, SIGNED_STATE_SIZE};

/// Fabrique un état factice de TAILLE RÉELLE, avec son `seq` encodé en tête (u32 LE). Le reste est du
/// remplissage : on ne mesure que le transport, donc seul le seq compte pour recoller les morceaux.
fn etat_factice(seq: u32) -> [u8; SIGNED_STATE_SIZE] {
    let mut s = [0u8; SIGNED_STATE_SIZE];
    s[0..4].copy_from_slice(&seq.to_le_bytes());
    s
}

/// Relit le `seq` d'une tranche d'état rendue par `decode_state_bundle` (réciproque de `etat_factice`).
fn seq_de(tranche: &[u8]) -> Option<u32> {
    if tranche.len() < 4 {
        return None;
    }
    Some(u32::from_le_bytes([tranche[0], tranche[1], tranche[2], tranche[3]]))
}

/// Un run réseau RÉEL pour une redondance K donnée. Renvoie (perte résiduelle dans [0,1], états reçus).
/// L'émetteur et le récepteur sont deux threads d'un même process, reliés par UDP sur `lo` (donc à
/// travers le qdisc netem éventuel). Aucun perçage, aucun rendez-vous : juste le bundle face à la perte.
fn run_un_k(n: usize, k: usize, rate_hz: f64) -> (f64, usize) {
    let k = k.max(1);

    // --- Récepteur : port éphémère de lo, timeout court pour pouvoir sortir proprement à la fin. ---
    let rx = UdpSocket::bind("127.0.0.1:0").expect("bind récepteur");
    rx.set_read_timeout(Some(Duration::from_millis(100))).expect("read_timeout");
    let port = rx.local_addr().expect("local_addr").port();

    let recus = Arc::new(Mutex::new(vec![false; n]));
    let done = Arc::new(AtomicBool::new(false));

    let recus_rx = Arc::clone(&recus);
    let done_rx = Arc::clone(&done);
    let h = thread::spawn(move || {
        let mut buf = [0u8; 64 * 1024];
        loop {
            match rx.recv(&mut buf) {
                Ok(len) => {
                    // VRAI décodage de production : un datagramme = un bundle des K derniers états.
                    if let Some(parts) = decode_state_bundle(&buf[..len]) {
                        let mut g = recus_rx.lock().expect("lock recus");
                        for p in parts {
                            if let Some(seq) = seq_de(p) {
                                if (seq as usize) < g.len() {
                                    g[seq as usize] = true; // dédoublonnage natif : un seq vu = vu.
                                }
                            }
                        }
                    }
                }
                Err(_) => {
                    // Timeout de lecture : si l'émission est finie ET la file drainée, on conclut.
                    if done_rx.load(Ordering::Relaxed) {
                        break;
                    }
                }
            }
        }
    });

    // --- Émetteur : à chaque tick t, le bundle des K derniers seq [t-k+1 .. t] (bord clampé à 0). ---
    let tx = UdpSocket::bind("127.0.0.1:0").expect("bind émetteur");
    let cible = format!("127.0.0.1:{port}");
    let dt = Duration::from_secs_f64(1.0 / rate_hz);
    // Pré-fabrique les états (pas d'alloc dans la boucle chaude).
    let etats: Vec<[u8; SIGNED_STATE_SIZE]> = (0..n as u32).map(etat_factice).collect();

    let debut = Instant::now();
    for t in 0..n {
        let first = t.saturating_sub(k - 1);
        let refs: Vec<&[u8]> = (first..=t).map(|i| &etats[i][..]).collect();
        let paquet = encode_state_bundle(&refs); // VRAI encodage de production.
        let _ = tx.send_to(&paquet, &cible);

        // Pacing : viser rate_hz (cible cumulée = pas de dérive). Sur lo, le drain est ~instantané ;
        // ce pacing évite seulement de NOYER la file netem (qui drop alors par dépassement, pas par loss%).
        let cible_t = dt.mul_f64(t as f64 + 1.0);
        let ecoule = debut.elapsed();
        if cible_t > ecoule {
            thread::sleep(cible_t - ecoule);
        }
    }

    // Laisser les derniers bundles arriver + la file netem se vider, puis clore le récepteur.
    thread::sleep(Duration::from_millis(300));
    done.store(true, Ordering::Relaxed);
    let _ = h.join();

    let g = recus.lock().expect("lock recus final");
    let recu = g.iter().filter(|&&b| b).count();
    (1.0 - recu as f64 / n as f64, recu)
}

/// Affiche le tableau Phase 3 RÉEL : redondance K → perte résiduelle MESURÉE vs prédiction `p^K`, où
/// `p` est la perte EFFECTIVE mesurée à K=1 (plus honnête que le nominal netem : la file/cadence peuvent
/// décaler un peu). `cargo run -- netem-bench`. À lancer dans un netns où `lo` porte du `netem loss`.
pub fn run_netem_bench(loss_nominal: f64, n: usize, rate_hz: f64) {
    println!("=== PHASE 3 (RÉEL) — REDONDANCE SUR LIEN netem (UDP réel sur lo, vrai KIND_STATE_BUNDLE) ===");
    println!(
        "    {n} états @ {rate_hz:.0} Hz, état = {SIGNED_STATE_SIZE} o ; perte netem nominale annoncée ≈ {loss_nominal:.0} %"
    );
    println!("    (la perte RÉELLE est celle du kernel ; on la LIT à K=1 et on teste K=2/3/4 contre p^K)\n");

    let ks = [1usize, 2, 3, 4];
    let mut pertes = [0.0f64; 4];
    let mut recus = [0usize; 4];
    for (idx, &k) in ks.iter().enumerate() {
        let (perte, recu) = run_un_k(n, k, rate_hz);
        pertes[idx] = perte;
        recus[idx] = recu;
    }

    let p1 = pertes[0]; // perte EFFECTIVE mesurée à K=1 = le « p » réel du lien.
    println!("  {:>3}  {:>10}  {:>12}  {:>10}", "K", "reçus", "perte mesurée", "prédit p^K");
    for (idx, &k) in ks.iter().enumerate() {
        let predit = p1.powi(k as i32) * 100.0;
        println!(
            "  {:>3}  {:>6}/{:<3}  {:>11.1}%  {:>9.1}%",
            k, recus[idx], n, pertes[idx] * 100.0, predit
        );
    }
    println!("\n  Lecture : si la perte est ALÉATOIRE (netem loss simple), la colonne « mesurée » doit");
    println!("  coller à « prédit p^K » → la redondance DIVISE la perte sur un vrai lien. Le seuil");
    println!("  « vivant » du projet = fraîcheur ≤ 500 ms ; ici on mesure la PERTE RÉSIDUELLE d'états.");
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Le round-trip seq survit au VRAI encode/decode du bundle (fidélité de l'instrument de mesure :
    /// si on ne sait pas relire le seq après un bundle réel, la perte mesurée serait fausse).
    #[test]
    fn round_trip_seq_a_travers_un_vrai_bundle() {
        let etats: Vec<[u8; SIGNED_STATE_SIZE]> = (10u32..13).map(etat_factice).collect();
        let refs: Vec<&[u8]> = etats.iter().map(|e| &e[..]).collect();
        let paquet = encode_state_bundle(&refs);
        let parts = decode_state_bundle(&paquet).expect("bundle valide");
        let seqs: Vec<u32> = parts.iter().filter_map(|p| seq_de(p)).collect();
        assert_eq!(seqs, vec![10, 11, 12], "les seq doivent ressortir intacts et dans l'ordre");
    }

    /// SANS perte (pas de netem), un run réel doit livrer 100 % des états quel que soit K — garde-fou
    /// contre un banc cassé (récepteur qui rate, pacing qui noie la file, seq mal relu…).
    #[test]
    fn sans_perte_tout_arrive() {
        for k in [1usize, 2, 3] {
            let (perte, recu) = run_un_k(300, k, 3000.0);
            assert_eq!(recu, 300, "K={k} : tout doit arriver sans netem (reçu {recu}/300)");
            assert!(perte.abs() < 1e-9, "K={k} : perte nulle attendue, obtenu {perte}");
        }
    }
}
