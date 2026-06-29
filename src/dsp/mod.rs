//! DSP fait-main, white-box, std-only — le SOCLE SPECTRAL partagé.
//!
//! Décidé le 30 juin 2026 : le codec voix est **fait main** (pas Opus, pas de boîte noire — fidèle au README
//! « le cœur est du code, fait main, sans boîte noire ; la seule dépendance = la crypto »). Le même front-end
//! spectral sert DEUX usages (esprit « estime corroborée » : une mécanique, plusieurs besoins) :
//!   - le **codec transform-domain** (FFT fenêtrée → quantification perceptuelle → reconstruction) — agnostique au
//!     signal, donc chuchotement/chant/beatbox survivent (≠ vocoder LPC qui détruit le non-parole) ;
//!   - **« l'étude du micro »** (débruitage white-box contrôlé par l'utilisateur, cf. `prive/PLAN_TEST_VOIX.md` §1.8).
//!
//! Zéro dépendance externe : un FFT radix-2 fait main. La preuve qui ne ment pas = le **round-trip exact**
//! (FFT→IFFT et analyse→synthèse reconstruisent le signal à l'epsilon flottant près).

pub mod adaptive;
pub mod chain;
pub mod codec;
pub mod denoise;
pub mod fft;
pub mod optim;
pub mod psycho;
pub mod separate;
pub mod stoi;

pub use adaptive::run_adaptatif;
pub use chain::run_son;
pub use codec::run_codec;
pub use denoise::run_micro;
pub use optim::run_optim;
pub use separate::run_separe;
