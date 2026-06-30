//! ÉTUDE DU SPECTRE — un spectrogramme PNG fait main (std-only) pour DONNER DES YEUX à Claude (idée utilisateur,
//! 1er juillet 2026 : « je n'ai pas d'oreilles → un mini-logiciel d'étude du spectre pour que j'aie au moins une
//! vision »). On écrit une vraie image PNG (que je peux ensuite LIRE) : temps en X, fréquence en Y (grave en bas),
//! intensité = niveau (dB). On VOIT alors directement ce que la séparation retire (les raies du ventilo qui
//! disparaissent, les stries verticales des clics, etc.) au lieu de juger à l'oreille.
//!
//! Tout est white-box et sans dépendance : FFT/STFT maison (`fft.rs`), PNG = signature + IHDR + IDAT (flux zlib en
//! blocs DEFLATE NON COMPRESSÉS, donc pas de compresseur à écrire) + IEND, avec CRC32 et Adler32 faits main.

use super::fft::{hann, stft};
use std::io::Write;

// ---- PNG minimal (std-only) --------------------------------------------------------------------------------------

/// CRC32 (polynôme PNG 0xEDB88320), calculé à la volée (pas de table opaque).
fn crc32(data: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0xEDB8_8320 } else { crc >> 1 };
        }
    }
    crc ^ 0xFFFF_FFFF
}

/// Adler32 (la somme de contrôle du flux zlib).
fn adler32(data: &[u8]) -> u32 {
    let (mut a, mut b) = (1u32, 0u32);
    for &x in data {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    (b << 16) | a
}

/// Un chunk PNG : longueur (BE) + type + data + CRC32(type+data) (BE).
fn chunk(out: &mut Vec<u8>, typ: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(typ);
    out.extend_from_slice(data);
    let mut crc_in = Vec::with_capacity(4 + data.len());
    crc_in.extend_from_slice(typ);
    crc_in.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_in).to_be_bytes());
}

/// Flux zlib à blocs DEFLATE NON COMPRESSÉS (BTYPE=00) — valide, et zéro compresseur à écrire.
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut z = vec![0x78, 0x01]; // en-tête zlib (CMF, FLG)
    let mut i = 0;
    loop {
        let len = (raw.len() - i).min(65535);
        let dernier = i + len >= raw.len();
        z.push(if dernier { 1 } else { 0 }); // BFINAL + BTYPE=00
        z.extend_from_slice(&(len as u16).to_le_bytes());
        z.extend_from_slice(&(!(len as u16)).to_le_bytes()); // NLEN = complément
        z.extend_from_slice(&raw[i..i + len]);
        i += len;
        if dernier {
            break;
        }
    }
    z.extend_from_slice(&adler32(raw).to_be_bytes());
    z
}

/// Écrit une image RGB (8 bits) en PNG. `rgb` = `h` lignes de `w·3` octets.
fn ecrire_png(chemin: &str, w: usize, h: usize, rgb: &[u8]) -> std::io::Result<()> {
    // Données brutes filtrées : chaque ligne préfixée d'un octet de filtre 0 (« None »).
    let mut raw = Vec::with_capacity(h * (1 + w * 3));
    for y in 0..h {
        raw.push(0);
        raw.extend_from_slice(&rgb[y * w * 3..(y + 1) * w * 3]);
    }
    let mut out: Vec<u8> = vec![137, 80, 78, 71, 13, 10, 26, 10]; // signature PNG
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]); // 8 bits/canal, type 2 = RGB, méthodes 0
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    chunk(&mut out, b"IEND", &[]);
    std::fs::File::create(chemin)?.write_all(&out)
}

// ---- Lecture WAV (PCM 16-bit, std-only) — le pont vers de VRAIS échantillons -------------------------------------

/// Lit un WAV PCM 16-bit (mono ou multi-canal → on garde le canal 0). En-tête canonique 44 octets (celui qu'écrit
/// `separate::ecrire_wav`). Renvoie (échantillons normalisés [-1,1], fréquence d'échantillonnage).
fn lire_wav(chemin: &str) -> std::io::Result<(Vec<f32>, f64)> {
    let o = std::fs::read(chemin)?;
    if o.len() < 44 {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "WAV trop court"));
    }
    let u16le = |i: usize| u16::from_le_bytes([o[i], o[i + 1]]) as usize;
    let u32le = |i: usize| u32::from_le_bytes([o[i], o[i + 1], o[i + 2], o[i + 3]]) as f64;
    let canaux = u16le(22).max(1);
    let sr = u32le(24);
    let mut samples = Vec::new();
    let mut i = 44;
    while i + 2 <= o.len() {
        samples.push(i16::from_le_bytes([o[i], o[i + 1]]) as f32 / 32768.0);
        i += 2 * canaux; // on échantillonne le canal 0
    }
    Ok((samples, sr))
}

// ---- Spectrogramme -----------------------------------------------------------------------------------------------

/// Plage dynamique (dB) d'un signal : (plancher = pic − 70 dB, pic). Sert d'échelle COMMUNE pour comparer des images.
fn plage_db(signal: &[f32], n: usize, hop: usize) -> (f64, f64) {
    let win = hann(n);
    let mut vmax = -300.0_f64;
    for fx in stft(signal, n, hop, &win) {
        for k in 0..n / 2 + 1 {
            let p = fx[k].re * fx[k].re + fx[k].im * fx[k].im;
            vmax = vmax.max(10.0 * (p + 1e-12).log10());
        }
    }
    (vmax - 70.0, vmax)
}

/// Palette « chaleur » lisible (sombre → bleu → magenta → orange → blanc) pour `v ∈ [0,1]`.
fn chaleur(v: f64) -> (u8, u8, u8) {
    let v = v.clamp(0.0, 1.0);
    let r = (255.0 * (1.5 * v).min(1.0)).round() as u8;
    let g = (255.0 * ((v - 0.35) / 0.65).clamp(0.0, 1.0)).round() as u8;
    let b = (255.0 * (1.0 - (1.6 * v - 0.2).clamp(0.0, 1.0)) + 255.0 * ((v - 0.8) / 0.2).clamp(0.0, 1.0)).min(255.0).round() as u8;
    (r, g, b)
}

/// Rend le spectrogramme de `signal` en PNG (X = temps, Y = fréquence grave→aigu de bas en haut), bornes dB données.
/// On ne montre que 0..`HZ_MAX` (là où vit la voix + nos bruits) → image plus lisible.
fn rendre(signal: &[f32], n: usize, hop: usize, sr: f64, vmin: f64, vmax: f64, chemin: &str) -> std::io::Result<()> {
    const HZ_MAX: f64 = 4000.0;
    const SX: usize = 4; // chaque trame → 4 px de large
    const SY: usize = 3; // chaque bin → 3 px de haut
    let win = hann(n);
    let sp = stft(signal, n, hop, &win);
    let frames = sp.len().max(1);
    let kmax = ((HZ_MAX * n as f64 / sr).round() as usize).min(n / 2);
    let kbins = kmax + 1;
    let (w, h) = (frames * SX, kbins * SY);
    let mut rgb = vec![0u8; w * h * 3];
    let span = (vmax - vmin).max(1e-9);
    for (t, fx) in sp.iter().enumerate() {
        for k in 0..kbins {
            let p = fx[k].re * fx[k].re + fx[k].im * fx[k].im;
            let db = 10.0 * (p + 1e-12).log10();
            let (cr, cg, cb) = chaleur((db - vmin) / span);
            for dy in 0..SY {
                for dx in 0..SX {
                    let px = t * SX + dx;
                    let py = h - 1 - (k * SY + dy); // grave en BAS
                    let idx = (py * w + px) * 3;
                    rgb[idx] = cr;
                    rgb[idx + 1] = cg;
                    rgb[idx + 2] = cb;
                }
            }
        }
    }
    ecrire_png(chemin, w, h, &rgb)
}

/// Point d'entrée `jeu spectre` : `jeu spectre <fichier.wav>` → un PNG ; `jeu spectre` (sans arg) → un PNG par WAV
/// du dossier `voix_wav/` (échelle dB COMMUNE = celle du mélange, pour comparer ce qui est retiré).
pub fn run_spectre(arg: &str) {
    let (n, hop) = (512usize, 256usize);
    println!("🖼️  ÉTUDE DU SPECTRE — spectrogramme PNG fait main (X=temps, Y=fréquence grave→aigu, couleur=niveau dB)");

    if arg.ends_with(".wav") {
        match lire_wav(arg) {
            Ok((sig, sr)) => {
                let (vmin, vmax) = plage_db(&sig, n, hop);
                let out = format!("{}.png", arg.trim_end_matches(".wav"));
                match rendre(&sig, n, hop, sr, vmin, vmax, &out) {
                    Ok(()) => println!("    {} → {}", arg, out),
                    Err(e) => println!("    ⚠ échec rendu {} : {}", out, e),
                }
            }
            Err(e) => println!("    ⚠ lecture {} impossible : {}", arg, e),
        }
        return;
    }

    let (dir, outdir) = ("voix_wav", "voix_spectres");
    let mel = format!("{dir}/00_melange.wav");
    let (vmin, vmax) = match lire_wav(&mel) {
        Ok((sig, _)) => plage_db(&sig, n, hop),
        Err(_) => {
            println!("    ⚠ {} introuvable — lance d'abord `jeu separe wav`.", mel);
            return;
        }
    };
    let _ = std::fs::create_dir_all(outdir);
    let mut fichiers: Vec<String> = match std::fs::read_dir(dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|f| f.ends_with(".wav"))
            .collect(),
        Err(e) => {
            println!("    ⚠ {} illisible : {}", dir, e);
            return;
        }
    };
    fichiers.sort();
    for f in &fichiers {
        if let Ok((sig, sr)) = lire_wav(&format!("{dir}/{f}")) {
            let out = format!("{outdir}/{}.png", f.trim_end_matches(".wav"));
            match rendre(&sig, n, hop, sr, vmin, vmax, &out) {
                Ok(()) => println!("    {} → {}", f, out),
                Err(e) => println!("    ⚠ {} : {}", out, e),
            }
        }
    }
    println!("\n📌 Échelle dB COMMUNE (celle du mélange) → comparer `melange` vs `sans_B*` MONTRE ce qui a été retiré.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_aller_retour_entete_et_sommes() {
        // Un PNG minimal 2×2 : on vérifie la signature, et que CRC/Adler sont cohérents (relecture des longueurs).
        let rgb = vec![255u8, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255]; // 2×2 RGB
        let chemin = std::env::temp_dir().join("web3_test_spectro.png");
        let p = chemin.to_string_lossy().into_owned();
        ecrire_png(&p, 2, 2, &rgb).unwrap();
        let octets = std::fs::read(&p).unwrap();
        assert_eq!(&octets[0..8], &[137, 80, 78, 71, 13, 10, 26, 10], "signature PNG");
        // IHDR doit annoncer 2×2.
        assert_eq!(u32::from_be_bytes([octets[16], octets[17], octets[18], octets[19]]), 2);
        assert_eq!(u32::from_be_bytes([octets[20], octets[21], octets[22], octets[23]]), 2);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn adler_et_crc_valeurs_connues() {
        // Valeurs de référence classiques.
        assert_eq!(adler32(b"abc"), 0x024d0127);
        assert_eq!(crc32(b"abc"), 0x352441c2);
    }
}
