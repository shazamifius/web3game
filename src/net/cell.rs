//! LE RÉSUMÉ DE CELLULE (chapitre 8.3, D22) : percevoir une FOULE sans recevoir N flux.
//!
//! # Le mur que ça enlève
//! 8.2 a borné le DÉBIT reçu (focus plein + conscience à miettes), mais la conscience partage un
//! budget FIXE entre TOUS les pairs lointains : à N = 5000, chacun reçoit `~80 Hz / 5000 ≈ 1
//! mise à jour par MINUTE` d'un lointain → la foule lointaine devient une purée figée (effondrement
//! de fraîcheur en 1/N). Le débit reste plat (bien), mais l'INFO devient inutile.
//!
//! # L'idée
//! Le monde est découpé en CELLULES (grille, `aoi::cell_of`). Chaque cellule a un HÔTE élu
//! (`NetLink::cell_host` : plus petit id connu dans la cellule). L'hôte produit UN **résumé basse
//! fréquence** de sa cellule — combien d'occupants + quelques positions REPRÉSENTATIVES (un
//! échantillon, pas les 500 individus) — et le diffuse. Un observateur reçoit alors **1 flux
//! résumé par cellule** au lieu de N flux individuels → réception = `O(focus + cellules)`,
//! indépendante de N, ET avec une fraîcheur correcte de la foule lointaine.
//!
//! # AUTHENTIFIÉ depuis D26-couche-1 (chap. 8.3e) — le résumé n'est PLUS anonyme
//! Avant, le résumé était le SEUL paquet sans signature : n'importe qui forgeait un résumé pour
//! n'importe quelle cellule, et un `ts = u64::MAX` ÉPINGLAIT le mensonge à vie. Désormais l'hôte
//! **embarque sa clé publique** (`host`) et **SIGNE** son résumé — exactement comme un état joueur
//! ([`message`]). La fraîcheur n'est plus l'horloge murale `ts` (attaquable) mais un **`seq`
//! monotone PAR HÔTE** (jumeau de l'anti-rejeu des états). À l'ingestion ([`NetLink::ingest_summary`])
//! on vérifie le sceau ET que `host == cell_host(cellule)` → un non-hôte ne peut plus rien forger.
//! *(L'hôte LÉGITIME peut encore mentir sur SA cellule : c'est la couche 2 = corroboration, 8.8.)*
//!
//! # Consultatif, PAS autoritaire (le point qui SIMPLIFIE)
//! Contrairement à l'orbe, un résumé n'a aucune AUTORITÉ : il ne fait que décrire une région. Si
//! deux hôtes résument la même cellule (désaccord transitoire de migration en grande foule), on a
//! juste **deux flux redondants** — un peu de gaspillage, AUCUNE corruption. Donc 8.3 n'a PAS
//! besoin que la migration soit durcie d'abord (D11).
//!
//! 8.3c : l'émission par l'hôte (épidémique, fanout borné), le relais et l'ingestion sont câblés
//! dans `bot.rs` (et `NetLink::build_my_cell_summary` / `ingest_summary`). Preuve d'échelle = 8.3d.

use super::crypto::{verify, Identity, PeerId, PUBKEY_LEN, SIG_LEN};
use super::wire::{KIND_CELL_SUMMARY, PROTO_VERSION};

/// Nombre MAX de positions représentatives dans un résumé. 16 × 40 o (id+pos) = 640 o de samples,
/// + l'en-tête + la signature reste sous un datagramme. Borne le coût d'un résumé quelle que soit
/// la taille de la foule.
pub(crate) const MAX_CELL_SAMPLES: usize = 16;

// Décalages des champs dans le CORPS signé (avant la signature), calculés à la main pour bien
// comprendre — même esprit que `message.rs`. La clé `host` est EMBARQUÉE (auto-certification) :
// c'est elle qui sert à vérifier le sceau.
//   [0] type | [1] version | [2..34] host (clé, 32 o) | [34..38] cell.0 | [38..42] cell.1
//   | [42..46] count (u32) | [46..54] seq (u64) | [54] n_samples (u8) | [55..] samples (n×40 o : id+x+z)
//   puis | [..] signature (64 o) APRÈS le corps.
const OFF_HOST: usize = 2;
const OFF_CELL: usize = OFF_HOST + PUBKEY_LEN; // 34
const OFF_COUNT: usize = OFF_CELL + 8; // 42
const OFF_SEQ: usize = OFF_COUNT + 4; // 46
const OFF_NSAMP: usize = OFF_SEQ + 8; // 54
/// Longueur de l'en-tête du corps (tout sauf les échantillons et la signature) = 55 octets.
const HEADER: usize = OFF_NSAMP + 1;
/// Taille d'un échantillon : id (32) + x (4) + z (4) = 40 octets (chap. 8.3★ : l'ID rend
/// l'échantillon auto-identifiant → l'ingestion peut UNIONNER des personnes distinctes au lieu
/// d'écraser un résumé par cellule, ce qui « thrashait » sous découverte sparse).
const SAMPLE_SIZE: usize = PUBKEY_LEN + 8;

/// Le RÉSUMÉ d'une cellule, produit et SIGNÉ par son hôte (chap. 8.3 / D26-couche-1) : QUI EST LÀ
/// (combien) et OÙ (quelques positions représentatives), à bas débit. Inonde un observateur d'UNE
/// info de foule au lieu de N états individuels — et il est désormais AUTHENTIFIÉ.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CellSummary {
    pub(crate) cell: (i32, i32),
    /// HÔTE émetteur = sa clé publique, EMBARQUÉE (auto-certifiante, comme `PlayerState.id`).
    /// C'est contre ELLE qu'on vérifie le sceau, et c'est elle qu'on compare à `cell_host`.
    pub(crate) host: PeerId,
    /// Occupants ESTIMÉS de la cellule (≥ le nombre d'échantillons : la foule peut dépasser 16).
    pub(crate) count: u32,
    /// FRAÎCHEUR = compteur monotone PAR HÔTE (chap. D26-couche-1) : sert à l'ingestion à ne garder
    /// que le résumé le PLUS FRAIS du MÊME hôte (`seq` strictement plus grand). Les relais le portent
    /// VERBATIM. Ce n'est PLUS l'horloge murale `ts` (un `u64::MAX` épinglait le mensonge à vie) :
    /// un non-hôte ne peut pas forger ce `seq`, et un changement d'hôte légitime le remet à zéro.
    pub(crate) seq: u64,
    /// Échantillon REPRÉSENTATIF (≤ `MAX_CELL_SAMPLES`) — `(id, x, z)` de quelques occupants. L'ID
    /// (chap. 8.3★) rend chaque échantillon AUTO-IDENTIFIANT : l'ingestion peut alors UNIONNER les
    /// personnes distinctes vues à travers tous les résumés reçus (perception = |union|), au lieu de
    /// ne garder qu'un résumé par cellule (qui « thrashait » dès qu'on retirait l'élection d'hôte).
    pub(crate) samples: Vec<(PeerId, f32, f32)>,
    /// SCEAU Ed25519 (64 o) du corps, apposé par l'hôte. Porté verbatim par les relais (jamais
    /// re-signé) → la copie fraîche de l'hôte bat les vieilles encore en vol sans qu'un relais
    /// puisse altérer le contenu. Zéros tant que le résumé n'est pas scellé (`sign_summary`).
    pub(crate) sig: [u8; SIG_LEN],
}

/// Construit le résumé d'une cellule à partir des positions de ses occupants connus (chap. 8.3).
/// `host` = clé de l'hôte émetteur ; `seq` = son compteur monotone de fraîcheur (fourni par
/// l'appelant → fonction PURE). `count` = nombre réel d'occupants ; `samples` = un échantillon
/// RÉPARTI (pas les 16 premiers : pas régulier → représentatif de toute la foule). Le sceau `sig`
/// est laissé à ZÉRO : il faut appeler `sign_summary` ensuite (l'identité reste hors d'ici).
pub(crate) fn build_cell_summary(
    cell: (i32, i32),
    host: PeerId,
    occupants: &[(PeerId, f32, f32)],
    seq: u64,
) -> CellSummary {
    let count = occupants.len() as u32;
    let samples: Vec<(PeerId, f32, f32)> = if occupants.len() <= MAX_CELL_SAMPLES {
        occupants.to_vec()
    } else {
        // Pas régulier : on couvre toute la liste (échantillon réparti, pas les premiers).
        let stride = occupants.len() / MAX_CELL_SAMPLES;
        (0..MAX_CELL_SAMPLES).map(|k| occupants[k * stride]).collect()
    };
    CellSummary { cell, host, count, seq, samples, sig: [0u8; SIG_LEN] }
}

/// Sérialise le CORPS d'un résumé (tout SAUF la signature), forme canonique qui sera signée puis
/// vérifiée. Tronque à `MAX_CELL_SAMPLES` (borne de coût). Déterministe : un relais re-sérialise
/// le même corps à partir de la struct décodée → le sceau porté verbatim reste valide.
fn encode_cell_summary_body(s: &CellSummary) -> Vec<u8> {
    let n = s.samples.len().min(MAX_CELL_SAMPLES);
    let mut buf = Vec::with_capacity(HEADER + n * SAMPLE_SIZE);
    buf.push(KIND_CELL_SUMMARY);
    buf.push(PROTO_VERSION);
    buf.extend_from_slice(s.host.bytes());
    buf.extend_from_slice(&s.cell.0.to_le_bytes());
    buf.extend_from_slice(&s.cell.1.to_le_bytes());
    buf.extend_from_slice(&s.count.to_le_bytes());
    buf.extend_from_slice(&s.seq.to_le_bytes());
    buf.push(n as u8);
    for &(id, x, z) in s.samples.iter().take(n) {
        buf.extend_from_slice(id.bytes());
        buf.extend_from_slice(&x.to_le_bytes());
        buf.extend_from_slice(&z.to_le_bytes());
    }
    buf
}

/// SCELLE un résumé : calcule la signature de son corps canonique avec l'identité de l'hôte et la
/// range dans `s.sig`. `s.host` DOIT être `identity.id()`, sinon le sceau ne collera pas à la clé
/// embarquée (exactement ce qui rend l'usurpation impossible — on ne signe que pour SA clé).
pub(crate) fn sign_summary(s: &mut CellSummary, identity: &Identity) {
    let body = encode_cell_summary_body(s);
    s.sig = identity.sign(&body);
}

/// Le sceau d'un résumé est-il valide ? On re-sérialise son corps canonique et on vérifie sa `sig`
/// contre la clé `host` EMBARQUÉE (auto-certification, aucun annuaire). On n'appelle ceci qu'APRÈS le
/// contrôle (bon marché) `host == cell_host` (le contrôle cher après le pas cher borne le coût CPU
/// d'un flot de faux résumés). `false` au moindre défaut (clé invalide, corps falsifié, clé usurpée).
pub(crate) fn summary_sig_ok(s: &CellSummary) -> bool {
    verify(&encode_cell_summary_body(s), &s.sig, s.host.bytes())
}

/// Sérialise un résumé COMPLET (corps + signature) pour l'envoi ou le relais. La `sig` est apposée
/// verbatim (jamais recalculée) → un résumé reçu puis relayé reste vérifiable par le sceau de l'hôte.
pub(crate) fn encode_cell_summary(s: &CellSummary) -> Vec<u8> {
    let mut buf = encode_cell_summary_body(s);
    buf.extend_from_slice(&s.sig);
    buf
}

/// Désérialise un résumé (corps + signature). `None` si type/version/taille invalides, ou si un
/// échantillon est non fini (NaN/Inf → jamais de poison numérique dans l'AoI ; on REJETTE tout le
/// résumé plutôt que de muter sa forme, sinon un relais re-sérialiserait un corps différent et
/// casserait le sceau). N'AUTHENTIFIE PAS : appeler `sig_ok_cell_summary` séparément.
pub(crate) fn decode_cell_summary(buf: &[u8]) -> Option<CellSummary> {
    if buf.len() < HEADER + SIG_LEN || buf[0] != KIND_CELL_SUMMARY || buf[1] != PROTO_VERSION {
        return None;
    }
    let mut hb = [0u8; PUBKEY_LEN];
    hb.copy_from_slice(&buf[OFF_HOST..OFF_HOST + PUBKEY_LEN]);
    let host = PeerId::from_bytes(hb);
    let cx = i32::from_le_bytes([buf[OFF_CELL], buf[OFF_CELL + 1], buf[OFF_CELL + 2], buf[OFF_CELL + 3]]);
    let cz = i32::from_le_bytes([buf[OFF_CELL + 4], buf[OFF_CELL + 5], buf[OFF_CELL + 6], buf[OFF_CELL + 7]]);
    let count = u32::from_le_bytes([buf[OFF_COUNT], buf[OFF_COUNT + 1], buf[OFF_COUNT + 2], buf[OFF_COUNT + 3]]);
    let seq = u64::from_le_bytes([
        buf[OFF_SEQ], buf[OFF_SEQ + 1], buf[OFF_SEQ + 2], buf[OFF_SEQ + 3],
        buf[OFF_SEQ + 4], buf[OFF_SEQ + 5], buf[OFF_SEQ + 6], buf[OFF_SEQ + 7],
    ]);
    let n = buf[OFF_NSAMP] as usize;
    if n > MAX_CELL_SAMPLES {
        return None; // au-delà de la borne de coût → paquet malformé
    }
    let body_len = HEADER + n * SAMPLE_SIZE;
    if buf.len() != body_len + SIG_LEN {
        return None; // taille non canonique : on n'accepte que la forme exacte (re-sérialisable)
    }
    let mut samples = Vec::with_capacity(n);
    let mut o = HEADER;
    for _ in 0..n {
        let mut idb = [0u8; PUBKEY_LEN];
        idb.copy_from_slice(&buf[o..o + PUBKEY_LEN]);
        let id = PeerId::from_bytes(idb);
        let p = o + PUBKEY_LEN;
        let x = f32::from_le_bytes([buf[p], buf[p + 1], buf[p + 2], buf[p + 3]]);
        let z = f32::from_le_bytes([buf[p + 4], buf[p + 5], buf[p + 6], buf[p + 7]]);
        o += SAMPLE_SIZE;
        if !x.is_finite() || !z.is_finite() {
            return None; // un seul flottant non fini → rejet du résumé entier (forme préservée)
        }
        samples.push((id, x, z));
    }
    let mut sig = [0u8; SIG_LEN];
    sig.copy_from_slice(&buf[body_len..body_len + SIG_LEN]);
    Some(CellSummary { cell: (cx, cz), host, count, seq, samples, sig })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pid(seed: u8) -> PeerId {
        PeerId::from_bytes([seed; PUBKEY_LEN])
    }

    /// Aller-retour : un résumé scellé, encodé puis décodé revient identique ET son sceau tient.
    #[test]
    fn resume_signe_survit_a_l_aller_retour() {
        let identity = Identity::generate();
        let mut s = build_cell_summary((-3, 7), identity.id(), &[(pid(1), 1.0, -2.0), (pid(2), 3.5, 4.0)], 12);
        s.count = 42;
        sign_summary(&mut s, &identity);
        let bytes = encode_cell_summary(&s);
        let recu = decode_cell_summary(&bytes).expect("doit se décoder");
        assert!(summary_sig_ok(&recu)); // le sceau tient après aller-retour
        assert_eq!(recu, s);
    }

    /// La construction : `count` = vrai total, l'échantillon est borné ET RÉPARTI (couvre toute la
    /// foule, pas seulement le début).
    #[test]
    fn build_compte_tout_mais_echantillonne_reparti() {
        // 100 occupants alignés en x = 0,1,2,…99. Le résumé dit count=100 et n'en porte que 16.
        let occ: Vec<(PeerId, f32, f32)> = (0..100).map(|i| (pid(1), i as f32, 0.0)).collect();
        let s = build_cell_summary((0, 0), pid(1), &occ, 0);
        assert_eq!(s.count, 100);
        assert_eq!(s.samples.len(), MAX_CELL_SAMPLES);
        // Réparti : le 1er échantillon est au début, le dernier loin dans la foule (pas tous au début).
        assert_eq!(s.samples[0], (pid(1), 0.0, 0.0));
        assert!(s.samples.last().unwrap().1 >= 80.0); // x du dernier échantillon → couvre le fond de la foule
    }

    /// Petite cellule (≤ 16) : tout le monde est dans l'échantillon, count = taille.
    #[test]
    fn build_petite_cellule_prend_tout() {
        let occ = vec![(pid(1), 1.0, 1.0), (pid(2), 2.0, 2.0), (pid(3), 3.0, 3.0)];
        let s = build_cell_summary((5, 5), pid(2), &occ, 0);
        assert_eq!(s.count, 3);
        assert_eq!(s.samples, occ);
    }

    /// Le moindre octet du CORPS modifié casse le sceau → `sig_ok` faux (anti-falsification, comme
    /// l'état signé). Ici un relais malveillant trafique le `count` pour effacer une région.
    #[test]
    fn corps_altere_casse_le_sceau() {
        let identity = Identity::generate();
        let mut s = build_cell_summary((1, 1), identity.id(), &[(pid(1), 1.0, 2.0)], 1);
        s.count = 500;
        sign_summary(&mut s, &identity);
        let mut bytes = encode_cell_summary(&s);
        bytes[OFF_COUNT] ^= 0xFF; // on triture le count (effacement de foule)
        let recu = decode_cell_summary(&bytes).expect("se décode encore"); // count trafiqué, sig d'origine
        assert!(!summary_sig_ok(&recu)); // mais le sceau ne colle plus → rejet
    }

    /// USURPATION : un attaquant embarque la clé de la VICTIME dans `host`, mais signe avec SA
    /// propre clé. Le sceau, vérifié contre la clé embarquée (la victime), ne colle pas → rejet.
    #[test]
    fn usurpation_d_hote_est_rejetee() {
        let victime = Identity::generate();
        let attaquant = Identity::generate();
        let mut s = build_cell_summary((0, 0), victime.id(), &[(pid(1), 0.0, 0.0)], 1); // je PRÉTENDS être la victime…
        sign_summary(&mut s, &attaquant); // … mais je signe avec MA clé
        let recu = decode_cell_summary(&encode_cell_summary(&s)).expect("se décode");
        assert!(!summary_sig_ok(&recu)); // sceau vérifié contre la clé victime → ne colle pas
    }

    /// Un paquet tronqué (forme non canonique) est rejeté à la fois par le décodage et le sceau —
    /// on n'accepte que la taille EXACTE (sinon un relais re-sérialiserait un corps différent).
    #[test]
    fn taille_non_canonique_est_rejetee() {
        let identity = Identity::generate();
        let mut s = build_cell_summary((1, 1), identity.id(), &[(pid(1), 1.0, 2.0), (pid(2), 3.0, 4.0)], 0);
        sign_summary(&mut s, &identity);
        let mut bytes = encode_cell_summary(&s);
        bytes.truncate(bytes.len() - 3); // coupe la fin → taille non canonique
        assert_eq!(decode_cell_summary(&bytes), None); // rejet net au décodage
    }

    /// Un échantillon NaN/Inf fait rejeter le résumé ENTIER (forme préservée, jamais mutée).
    #[test]
    fn decode_rejette_position_non_finie() {
        let identity = Identity::generate();
        let mut s = build_cell_summary((0, 0), identity.id(), &[(pid(1), f32::NAN, 0.0), (pid(2), 1.0, 2.0)], 0);
        sign_summary(&mut s, &identity);
        assert_eq!(decode_cell_summary(&encode_cell_summary(&s)), None);
    }

    /// Mauvais type/version → rejeté proprement.
    #[test]
    fn decode_rejette_mauvais_entete() {
        let identity = Identity::generate();
        let mut s = build_cell_summary((0, 0), identity.id(), &[], 0);
        sign_summary(&mut s, &identity);
        let mut bytes = encode_cell_summary(&s);
        bytes[0] = 0xFF; // mauvais KIND
        assert_eq!(decode_cell_summary(&bytes), None);
    }
}
