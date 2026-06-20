//! LA CRYPTO : signatures à clé publique (Ed25519) — la frontière de confiance.
//!
//! # Pourquoi ce fichier est spécial
//! C'est le SEUL endroit du projet qui s'appuie sur une bibliothèque externe
//! (`ed25519-dalek`). Partout ailleurs on code tout à la main pour comprendre
//! chaque octet ; ici, NON, et c'est volontaire : on ne code JAMAIS sa propre
//! crypto. L'arithmétique sur courbe elliptique est un nid à failles subtiles
//! (canaux auxiliaires, valeurs dégénérées…) que même des experts ratent. On
//! délègue donc le CALCUL de la signature, et on garde la main sur tout le reste
//! (le format de l'enveloppe, la distribution des clés, la règle de vérification).
//!
//! # L'idée de la signature à clé publique (le cœur du chapitre 5)
//!   - chaque joueur possède une PAIRE de clés : une privée (secrète, gardée pour
//!     soi) et une publique (partagée, c'est son IDENTITÉ) ;
//!   - SIGNER un message avec la clé privée produit une « signature » de 64 octets
//!     que SEUL le détenteur de la clé privée pouvait calculer ;
//!   - VÉRIFIER cette signature avec la clé publique prouve deux choses d'un coup :
//!       1) le message vient bien du détenteur de la clé (authenticité) ;
//!       2) il n'a pas été modifié d'un seul octet (intégrité).
//! N'importe qui peut vérifier ; personne ne peut forger sans la clé privée.
//! C'est exactement l'« enveloppe scellée » qu'on voulait : un relais peut porter
//! l'enveloppe, mais pas en changer le contenu sans casser le sceau.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Taille d'une clé publique Ed25519 (octets) : c'est l'identité d'un joueur.
pub(crate) const PUBKEY_LEN: usize = 32;
/// Taille d'une signature Ed25519 (octets) : le « sceau » apposé sur un paquet.
pub(crate) const SIG_LEN: usize = 64;

/// DIFFICULTÉ anti-Sybil par DÉFAUT (chap. 6.2, montée au 9.1) : nombre de bits de tête à
/// ZÉRO qu'une clé publique doit avoir pour être une identité VALIDE. Comme une clé Ed25519
/// est ~aléatoire, en trouver une qui satisfait ça exige d'en essayer ~2^bits (du « minage »,
/// façon Hashcash). Vérifier, en revanche, est gratuit. Conséquence : **créer une identité
/// COÛTE** → un banni ne se reconnecte plus gratuitement, et fabriquer une FOULE de Sybils
/// devient cher (anti-Sybil de masse). Le 16 d'origine était un « jouet » (D6).
///
/// **Pourquoi 18 et pas plus (choix MESURÉ, 9.1) :** courbe relevée sur ce PC (×4 par +2 bits) —
/// 16 bits ≈ 0,3 s, **18 ≈ 3 s**, 20 ≈ 14 s, 22 ≈ 55 s *par identité*. (1) **Inclusivité** (pilier) :
/// 18 bits ≈ ~25-30 s sur un vieux téléphone = un coût d'entrée UNIQUE acceptable (comme générer une
/// clé SSH) ; 20+ l'exclurait. (2) **Depuis 9.2, la PoW n'a plus à être punitive** : le framing est
/// fermé par la CRÉDIBILITÉ des témoins, pas par le prix de l'identité ; la PoW ne sert plus qu'à
/// rendre une identité NON GRATUITE (anti reconnexion-spam, anti inondation de table). 4× le jouet
/// suffit. La vraie défense DYNAMIQUE sous attaque sera la couche (b) ADAPTATIVE (carrefour 9.1).
const DEFAULT_POW_BITS: u32 = 18;

/// Borne de sûreté : au-delà, le minage devient interminable (2³² ≈ minutes/heures) — on
/// refuse une difficulté absurde venue de l'environnement plutôt que de geler au démarrage.
const MAX_POW_BITS: u32 = 28;

/// DIFFICULTÉ anti-Sybil EFFECTIVE (chap. 9.1) — désormais **RÉGLABLE**, plus une constante
/// figée. C'est un paramètre de PROTOCOLE *réseau-large* : tous les nœuds d'une même instance
/// doivent exiger ET miner la MÊME difficulté (sinon ils se rejettent mutuellement). Surchargée
/// par la variable d'environnement `POW_BITS` — pour DURCIR un déploiement réel, ou l'ABAISSER
/// en simu/tests (le minage de centaines de bots à pleine difficulté serait lent). Résolue UNE
/// SEULE fois par processus (cache) : pas de relecture d'env à chaque paquet. (La couche (b)
/// *adaptative* du carrefour 9.1 — chaque nœud relève sa propre barre sous pression — viendra
/// au-dessus de ce socle réglable, plus tard.)
pub(crate) fn pow_bits() -> u32 {
    static BITS: OnceLock<u32> = OnceLock::new();
    *BITS.get_or_init(|| {
        match std::env::var("POW_BITS").ok().and_then(|s| s.parse::<u32>().ok()) {
            Some(b) if b <= MAX_POW_BITS => b,
            Some(b) => {
                eprintln!("POW_BITS={b} dépasse le plafond {MAX_POW_BITS} ; on retombe sur le défaut {DEFAULT_POW_BITS}.");
                DEFAULT_POW_BITS
            }
            None => DEFAULT_POW_BITS,
        }
    })
}

/// Compte les bits de tête à zéro d'une suite d'octets (gros-boutiste).
fn leading_zero_bits(bytes: &[u8]) -> u32 {
    let mut total = 0;
    for &b in bytes {
        if b == 0 {
            total += 8;
        } else {
            total += b.leading_zeros();
            break;
        }
    }
    total
}

/// L'IDENTITÉ d'un joueur sur le réseau = sa clé publique Ed25519 (32 octets).
///
/// # Auto-certifiante (le keystone du chapitre 6.1)
/// Chaque paquet signé porte cette clé ; vérifier le sceau AVEC cette clé prouve
/// l'identité sans demander à personne. Aucun serveur ne peut donc mentir sur
/// « telle clé = tel joueur » — l'identité EST la clé. Fini le numéro `u8` attribué
/// par le rendez-vous (plafond 255, collisions, et surtout racine de confiance
/// déléguée à un tiers). L'espace d'identité passe à 2²⁵⁶ : illimité en pratique.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) struct PeerId(pub(crate) [u8; PUBKEY_LEN]);

impl PeerId {
    /// Les 32 octets bruts (la clé publique elle-même).
    pub(crate) fn bytes(&self) -> &[u8; PUBKEY_LEN] {
        &self.0
    }
    /// Reconstruit un `PeerId` depuis 32 octets reçus du réseau.
    pub(crate) fn from_bytes(b: [u8; PUBKEY_LEN]) -> PeerId {
        PeerId(b)
    }
    /// Empreinte courte et lisible (4 premiers octets en hexa) pour les logs/affichage.
    pub(crate) fn short(&self) -> String {
        let b = &self.0;
        format!("{:02x}{:02x}{:02x}{:02x}", b[0], b[1], b[2], b[3])
    }
    /// Le `PeerId` « nul » (32 zéros) : sentinelle « pas de parent / personne ».
    /// Une vraie clé Ed25519 n'est jamais nulle, donc aucune ambiguïté.
    pub(crate) fn none() -> PeerId {
        PeerId([0u8; PUBKEY_LEN])
    }
    /// Est-ce la sentinelle nulle ?
    pub(crate) fn is_none(&self) -> bool {
        self.0 == [0u8; PUBKEY_LEN]
    }

    /// Cette identité porte-t-elle la PREUVE DE TRAVAIL exigée (chap. 6.2) ? Vrai si
    /// sa clé a au moins `bits` bits de tête à zéro. Vérification gratuite (O(1)).
    pub(crate) fn has_pow(&self, bits: u32) -> bool {
        leading_zero_bits(&self.0) >= bits
    }
}

impl std::fmt::Debug for PeerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}…", self.short())
    }
}

/// L'identité cryptographique d'UNE session : sa paire de clés. La privée ne
/// quitte JAMAIS la mémoire de ce processus ; on ne publie que la publique.
pub(crate) struct Identity {
    signing: SigningKey,
}

impl Identity {
    /// Tire une paire de clés au hasard, SANS preuve de travail (rapide). Réservé aux
    /// TESTS : en vrai, une identité doit être minée (`generate_pow`) pour être
    /// acceptée par les pairs et le rendez-vous (chap. 6.2).
    #[cfg(test)]
    pub(crate) fn generate() -> Identity {
        let seed = os_random_seed();
        Identity { signing: SigningKey::from_bytes(&seed) }
    }

    /// Tire une identité qui SATISFAIT la preuve de travail (`bits` bits de tête à
    /// zéro sur la clé publique) — chap. 6.2. On part d'une graine aléatoire et on
    /// l'incrémente comme un compteur jusqu'à tomber sur une clé conforme (« minage »).
    /// On lit `/dev/urandom` UNE seule fois (pas à chaque essai) : c'est l'incrément
    /// qui balaie l'espace, pas des relectures coûteuses. Coût ≈ 2^bits essais.
    pub(crate) fn generate_pow(bits: u32) -> Identity {
        let mut seed = os_random_seed();
        loop {
            let signing = SigningKey::from_bytes(&seed);
            if leading_zero_bits(&signing.verifying_key().to_bytes()) >= bits {
                return Identity { signing };
            }
            // Incrémente la graine (grand compteur little-endian) pour l'essai suivant.
            for byte in seed.iter_mut() {
                *byte = byte.wrapping_add(1);
                if *byte != 0 {
                    break;
                }
            }
        }
    }

    /// Notre clé PUBLIQUE (notre identité), prête à être envoyée sur le réseau.
    pub(crate) fn public(&self) -> [u8; PUBKEY_LEN] {
        self.signing.verifying_key().to_bytes()
    }

    /// Notre identité réseau (= notre clé publique, enveloppée en `PeerId`).
    pub(crate) fn id(&self) -> PeerId {
        PeerId(self.public())
    }

    /// Reconstruit une identité depuis sa graine PRIVÉE (32 octets) relue du disque (chap. 10.1) :
    /// c'est ce qui permet de RECHARGER son identité au lancement au lieu d'en miner une neuve.
    pub(crate) fn from_secret(secret: [u8; 32]) -> Identity {
        Identity { signing: SigningKey::from_bytes(&secret) }
    }

    /// La graine PRIVÉE (32 octets) — à écrire dans un fichier local protégé (perms 600), comme une
    /// clé SSH. Ne quitte JAMAIS la machine : le réseau ne voit que `public()`. Sert à la persistance.
    pub(crate) fn secret(&self) -> [u8; 32] {
        self.signing.to_bytes()
    }

    /// Appose notre sceau sur un message : 64 octets que seul le détenteur de
    /// notre clé privée pouvait produire pour CES octets précis.
    pub(crate) fn sign(&self, message: &[u8]) -> [u8; SIG_LEN] {
        self.signing.sign(message).to_bytes()
    }
}

/// Vérifie qu'une signature correspond bien à (ce message, cette clé publique).
/// Renvoie `false` au moindre doute : clé publique invalide, sceau qui ne colle
/// pas, message altéré. On ne fait JAMAIS confiance par défaut.
pub(crate) fn verify(message: &[u8], sig: &[u8; SIG_LEN], pubkey: &[u8; PUBKEY_LEN]) -> bool {
    let Ok(verifying_key) = VerifyingKey::from_bytes(pubkey) else {
        return false; // clé publique mal formée → on rejette
    };
    let signature = Signature::from_bytes(sig);
    verifying_key.verify(message, &signature).is_ok()
}

/// Lit 32 octets d'aléa cryptographique depuis le système (`/dev/urandom`).
/// On évite ainsi une dépendance de plus (`rand`) : la seule chose qu'on demande
/// au monde extérieur, c'est de l'entropie — et l'OS est fait pour ça.
fn os_random_seed() -> [u8; 32] {
    let mut seed = [0u8; 32];
    std::fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut seed))
        .expect("impossible de lire /dev/urandom pour générer les clés");
    seed
}

// ─────────────────────────── Identité PERSISTANTE (chapitre 10.1) ───────────────────────────
// On garde la MÊME identité entre deux sessions : on mine la clé UNE fois, on la sauve sur disque
// (comme une clé SSH), on la recharge ensuite. Sans ça, `NetLink::new` mine une identité neuve à
// chaque lancement → pas de « compte », réputation qui ne s'accumule pas (D14). Réservé au VRAI
// jeu : la simu/les bots gardent l'identité éphémère (`new`), pas de fichiers de clés à la pelle.

/// Dossier du coffre d'identité : `$WEB3GAME_DIR` si défini, sinon `$HOME/.web3game`. Local par
/// utilisateur, jamais réseau — comme `~/.ssh`.
fn identity_dir() -> PathBuf {
    if let Some(d) = std::env::var_os("WEB3GAME_DIR") {
        return PathBuf::from(d);
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
    home.join(".web3game")
}

/// Restreint un fichier de clé à son seul propriétaire (perms 600 sous Unix), comme `~/.ssh/id_*`.
#[cfg(unix)]
fn restrict_perms(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
}
#[cfg(not(unix))]
fn restrict_perms(_path: &Path) {}

/// CHARGE l'identité du `profile` depuis `dir`, ou en MINE une neuve et la SAUVE. Renvoie
/// `(identité, neuve?)`. Une clé dont la PoW ne satisfait plus `bits` (ex. `POW_BITS` relevé) est
/// re-minée + ré-écrite (sinon les pairs la rejetteraient). `dir` est un PARAMÈTRE → fonction
/// testable avec un dossier temporaire (l'environnement reste hors d'ici).
pub(crate) fn load_or_create_in(dir: &Path, profile: &str, bits: u32) -> std::io::Result<(Identity, bool)> {
    let path = dir.join(format!("{profile}.key"));
    if let Ok(bytes) = std::fs::read(&path) {
        if let Ok(seed) = <[u8; 32]>::try_from(bytes.as_slice()) {
            let id = Identity::from_secret(seed);
            if id.id().has_pow(bits) {
                return Ok((id, false)); // relue du disque, toujours valide
            }
            // PoW devenue insuffisante → on re-mine (sinon rejet par les pairs).
        }
        // fichier corrompu / mauvaise taille → on re-mine proprement.
    }
    let id = Identity::generate_pow(bits);
    std::fs::create_dir_all(dir)?;
    std::fs::write(&path, id.secret())?;
    restrict_perms(&path);
    Ok((id, true))
}

/// Comme `load_or_create_in`, mais résout le dossier depuis l'environnement (`identity_dir`) et
/// renvoie aussi le CHEMIN du fichier (pour l'afficher au joueur). Le point d'entrée du vrai jeu.
pub(crate) fn load_or_create_identity(profile: &str, bits: u32) -> std::io::Result<(Identity, PathBuf, bool)> {
    let dir = identity_dir();
    let (id, fresh) = load_or_create_in(&dir, profile, bits)?;
    Ok((id, dir.join(format!("{profile}.key")), fresh))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signature_valide_est_acceptee() {
        let id = Identity::generate();
        let pubkey = id.public();
        let msg = b"position du joueur 7";
        let sig = id.sign(msg);
        assert!(verify(msg, &sig, &pubkey));
    }

    #[test]
    fn message_altere_est_rejete() {
        let id = Identity::generate();
        let pubkey = id.public();
        let sig = id.sign(b"je suis a la position A");
        // Le moindre octet changé → le sceau ne colle plus.
        assert!(!verify(b"je suis a la position B", &sig, &pubkey));
    }

    #[test]
    fn pow_se_verifie() {
        // Une identité minée à 8 bits satisfait has_pow(8) (rapide : ~256 essais).
        let id = Identity::generate_pow(8);
        assert!(id.id().has_pow(8));
    }

    #[test]
    fn pow_rejette_une_cle_sans_travail() {
        // Clé toute en 0xFF : aucun bit de tête à zéro → échoue dès la difficulté 1.
        assert!(!PeerId::from_bytes([0xFF; PUBKEY_LEN]).has_pow(1));
    }

    #[test]
    fn signature_d_un_autre_est_rejetee() {
        let moi = Identity::generate();
        let autre = Identity::generate();
        let msg = b"transfert de l'orbe";
        let sig = autre.sign(msg); // signé par QUELQU'UN D'AUTRE
        // Vérifiée contre MA clé publique → refusée : pas d'usurpation possible.
        assert!(!verify(msg, &sig, &moi.public()));
    }

    /// 10.1 — la graine PRIVÉE se sérialise sans perte : graine → Identity → graine donne la MÊME
    /// identité, et elle peut toujours signer. C'est ce qui rend l'identité sauvegardable/rechargeable.
    #[test]
    fn identite_survit_a_l_aller_retour_disque() {
        let id = Identity::generate_pow(8);
        let back = Identity::from_secret(id.secret());
        assert_eq!(id.id(), back.id());
        let msg = b"je suis le meme joueur qu'hier";
        assert!(verify(msg, &back.sign(msg), &id.public())); // recharge → signe encore
    }

    /// 10.1 — LE CŒUR : un 2e « lancement » RECHARGE la même identité du disque (pas une neuve), et
    /// deux profils distincts (ex. fenêtres `a` et `b`) ne collisionnent pas. Dossier temporaire →
    /// pas de pollution de `~/.web3game`, pas de variable d'environnement (édition 2024).
    #[test]
    fn identite_persiste_entre_deux_lancements() {
        let dir = std::env::temp_dir().join(format!("web3game-id-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir); // repart propre
        let (id1, fresh1) = load_or_create_in(&dir, "joueur", 8).expect("création");
        assert!(fresh1, "1er lancement : identité NEUVE minée et sauvée");
        let (id2, fresh2) = load_or_create_in(&dir, "joueur", 8).expect("rechargement");
        assert!(!fresh2, "2e lancement : identité RELUE du disque");
        assert_eq!(id1.id(), id2.id(), "persistance : MEME identite entre deux lancements");
        let (autre, _) = load_or_create_in(&dir, "autre", 8).expect("autre profil");
        assert_ne!(id1.id(), autre.id(), "profils distincts (a/b) = identites distinctes");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
