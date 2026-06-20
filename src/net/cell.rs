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
//! # Consultatif, PAS autoritaire (le point qui SIMPLIFIE)
//! Contrairement à l'orbe, un résumé n'a aucune AUTORITÉ : il ne fait que décrire une région. Si
//! deux hôtes résument la même cellule (désaccord transitoire de migration en grande foule), on a
//! juste **deux flux redondants** — un peu de gaspillage, AUCUNE corruption. Donc 8.3 n'a PAS
//! besoin que la migration soit durcie d'abord (D11). *(Trust : un hôte peut MENTIR sur sa cellule
//! — cacher/inventer des gens, D5/D9 ; corroboration multi-informateurs = 8.8, plus tard.)*
//!
//! 8.3c : l'émission par l'hôte (épidémique, fanout borné), le relais et l'ingestion sont câblés
//! dans `bot.rs` (et `NetLink::build_my_cell_summary` / `ingest_summary`). Preuve d'échelle = 8.3d.

use super::wire::{KIND_CELL_SUMMARY, PROTO_VERSION};

/// Nombre MAX de positions représentatives dans un résumé. 16 × 8 o + 15 o d'en-tête = 143 o,
/// bien sous un datagramme. Borne le coût d'un résumé quelle que soit la taille de la foule.
pub(crate) const MAX_CELL_SAMPLES: usize = 16;

/// En-tête : KIND + VERSION + cell.0 (i32) + cell.1 (i32) + count (u32) + ts (u64) + n_samples (u8) = 23 o.
const HEADER: usize = 2 + 4 + 4 + 4 + 8 + 1;
/// Taille d'un échantillon : x (4) + z (4) = 8 octets.
const SAMPLE_SIZE: usize = 8;

/// Le RÉSUMÉ d'une cellule, produit par son hôte (chap. 8.3) : QUI EST LÀ (combien) et OÙ
/// (quelques positions représentatives), à bas débit. Inonde un observateur d'UNE info de foule
/// au lieu de N états individuels.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct CellSummary {
    pub(crate) cell: (i32, i32),
    /// Occupants ESTIMÉS de la cellule (≥ le nombre d'échantillons : la foule peut dépasser 16).
    pub(crate) count: u32,
    /// HORODATAGE d'émission (ms, chap. 8.3d) : sert à l'ingestion à ne garder que le résumé le
    /// PLUS FRAIS par cellule. Les relais le portent VERBATIM (ils ne le restampent pas) → une
    /// vieille copie partielle qui circule encore ne peut plus écraser la fraîche (le bug du 8.3c).
    /// *(Horloge murale partagée sur une machine ; en réseau réel, biais borné = D13, consultatif.)*
    pub(crate) ts: u64,
    /// Positions REPRÉSENTATIVES (≤ `MAX_CELL_SAMPLES`) — un échantillon réparti de la foule.
    pub(crate) samples: Vec<(f32, f32)>,
}

/// Construit le résumé d'une cellule à partir des positions de ses occupants connus (chap. 8.3).
/// `count` = nombre réel d'occupants ; `samples` = un échantillon RÉPARTI (pas les 16 premiers : on
/// prend un pas régulier dans la liste → représentatif de toute la foule) ; `ts` = horodatage
/// d'émission (fourni par l'appelant → fonction PURE/testable, l'horloge reste hors d'ici).
pub(crate) fn build_cell_summary(cell: (i32, i32), occupants: &[(f32, f32)], ts: u64) -> CellSummary {
    let count = occupants.len() as u32;
    let samples: Vec<(f32, f32)> = if occupants.len() <= MAX_CELL_SAMPLES {
        occupants.to_vec()
    } else {
        // Pas régulier : on couvre toute la liste (échantillon réparti, pas les premiers).
        let stride = occupants.len() / MAX_CELL_SAMPLES;
        (0..MAX_CELL_SAMPLES).map(|k| occupants[k * stride]).collect()
    };
    CellSummary { cell, count, ts, samples }
}

/// Sérialise un résumé : en-tête + positions. Tronque à `MAX_CELL_SAMPLES` (borne de coût).
pub(crate) fn encode_cell_summary(s: &CellSummary) -> Vec<u8> {
    let n = s.samples.len().min(MAX_CELL_SAMPLES);
    let mut buf = Vec::with_capacity(HEADER + n * SAMPLE_SIZE);
    buf.push(KIND_CELL_SUMMARY);
    buf.push(PROTO_VERSION);
    buf.extend_from_slice(&s.cell.0.to_le_bytes());
    buf.extend_from_slice(&s.cell.1.to_le_bytes());
    buf.extend_from_slice(&s.count.to_le_bytes());
    buf.extend_from_slice(&s.ts.to_le_bytes());
    buf.push(n as u8);
    for &(x, z) in s.samples.iter().take(n) {
        buf.extend_from_slice(&x.to_le_bytes());
        buf.extend_from_slice(&z.to_le_bytes());
    }
    buf
}

/// Désérialise un résumé. `None` si type/version/taille invalides. Tronqué → on garde ce qu'on a
/// lu (comme `decode_gossip`). Une position non finie (NaN/Inf) fait rejeter l'échantillon
/// (jamais de poison numérique dans l'AoI).
pub(crate) fn decode_cell_summary(buf: &[u8]) -> Option<CellSummary> {
    if buf.len() < HEADER || buf[0] != KIND_CELL_SUMMARY || buf[1] != PROTO_VERSION {
        return None;
    }
    let cx = i32::from_le_bytes([buf[2], buf[3], buf[4], buf[5]]);
    let cz = i32::from_le_bytes([buf[6], buf[7], buf[8], buf[9]]);
    let count = u32::from_le_bytes([buf[10], buf[11], buf[12], buf[13]]);
    let ts = u64::from_le_bytes([
        buf[14], buf[15], buf[16], buf[17], buf[18], buf[19], buf[20], buf[21],
    ]);
    let n = buf[22] as usize;
    let mut samples = Vec::with_capacity(n.min(MAX_CELL_SAMPLES));
    let mut o = HEADER;
    for _ in 0..n {
        if o + SAMPLE_SIZE > buf.len() {
            break; // tronqué : on garde les échantillons complets
        }
        let x = f32::from_le_bytes([buf[o], buf[o + 1], buf[o + 2], buf[o + 3]]);
        let z = f32::from_le_bytes([buf[o + 4], buf[o + 5], buf[o + 6], buf[o + 7]]);
        o += SAMPLE_SIZE;
        if x.is_finite() && z.is_finite() {
            samples.push((x, z));
        }
    }
    Some(CellSummary { cell: (cx, cz), count, ts, samples })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Aller-retour : un résumé encodé puis décodé revient identique.
    #[test]
    fn resume_survit_a_l_aller_retour() {
        let s = CellSummary { cell: (-3, 7), count: 42, ts: 123_456, samples: vec![(1.0, -2.0), (3.5, 4.0)] };
        assert_eq!(decode_cell_summary(&encode_cell_summary(&s)), Some(s));
    }

    /// La construction : `count` = vrai total, l'échantillon est borné ET RÉPARTI (couvre toute la
    /// foule, pas seulement le début).
    #[test]
    fn build_compte_tout_mais_echantillonne_reparti() {
        // 100 occupants alignés en x = 0,1,2,…99. Le résumé dit count=100 et n'en porte que 16.
        let occ: Vec<(f32, f32)> = (0..100).map(|i| (i as f32, 0.0)).collect();
        let s = build_cell_summary((0, 0), &occ, 0);
        assert_eq!(s.count, 100);
        assert_eq!(s.samples.len(), MAX_CELL_SAMPLES);
        // Réparti : le 1er échantillon est au début, le dernier loin dans la foule (pas tous au début).
        assert_eq!(s.samples[0], (0.0, 0.0));
        assert!(s.samples.last().unwrap().0 >= 80.0); // couvre le fond de la foule
    }

    /// Petite cellule (≤ 16) : tout le monde est dans l'échantillon, count = taille.
    #[test]
    fn build_petite_cellule_prend_tout() {
        let occ = vec![(1.0, 1.0), (2.0, 2.0), (3.0, 3.0)];
        let s = build_cell_summary((5, 5), &occ, 0);
        assert_eq!(s.count, 3);
        assert_eq!(s.samples, occ);
    }

    /// Un paquet tronqué en plein milieu ne plante pas : on garde les échantillons complets.
    #[test]
    fn decode_tronque_ne_plante_pas() {
        let s = CellSummary { cell: (1, 1), count: 2, ts: 0, samples: vec![(1.0, 2.0), (3.0, 4.0)] };
        let mut bytes = encode_cell_summary(&s);
        bytes.truncate(bytes.len() - 3); // coupe le 2e échantillon
        let d = decode_cell_summary(&bytes).expect("doit se décoder");
        assert_eq!(d.cell, (1, 1));
        assert_eq!(d.count, 2); // le count annoncé est conservé
        assert_eq!(d.samples, vec![(1.0, 2.0)]); // seul le 1er échantillon complet survit
    }

    /// Un échantillon NaN/Inf est ignoré (jamais de flottant non fini dans l'AoI).
    #[test]
    fn decode_rejette_position_non_finie() {
        let s = CellSummary { cell: (0, 0), count: 2, ts: 0, samples: vec![(f32::NAN, 0.0), (1.0, 2.0)] };
        let d = decode_cell_summary(&encode_cell_summary(&s)).expect("doit se décoder");
        assert_eq!(d.samples, vec![(1.0, 2.0)]); // la NaN sautée, la saine gardée
    }

    /// Mauvais type/version → rejeté proprement.
    #[test]
    fn decode_rejette_mauvais_entete() {
        let s = CellSummary { cell: (0, 0), count: 0, ts: 0, samples: vec![] };
        let mut bytes = encode_cell_summary(&s);
        bytes[0] = 0xFF; // mauvais KIND
        assert_eq!(decode_cell_summary(&bytes), None);
    }
}
