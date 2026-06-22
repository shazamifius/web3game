//! LE NETCODE : tout ce qui rattrape la latence et la perte de paquets pour que
//! l'avatar distant bouge de façon fluide, malgré un réseau imparfait.
//!
//! # Le pipeline (dans l'ordre où ça s'enchaîne chaque image)
//!   - `state`       : les instantanés, la file par joueur, et les RÉGLAGES
//!   - `send`        : émettre NOTRE état (débit limité)
//!   - `receive`     : ranger les paquets reçus (et créer l'avatar au 1er paquet)
//!   - `interpolate` : animer chaque image (horloge adaptative + réconciliation)
//!   - `predict`     : calculer l'état voulu (interpolation OU prédiction)
//!   - `smooth`      : le ressort amorti (SmoothDamp) + helpers d'angles
//!
//! # Tester un vrai mauvais réseau
//!   sudo tc qdisc add dev lo root netem delay 80ms 40ms loss 10%   # dégrade
//!   sudo tc qdisc del dev lo root                                  # remet normal

mod interpolate;
mod nameplates;
mod predict;
mod receive;
mod send;
mod smooth;
mod state;

pub use interpolate::net_interpolate;
pub use nameplates::{update_nameplates, Nameplates};
pub use receive::net_receive;
pub use send::net_send;
pub(crate) use send::relay_fallback_enabled; // 12.3-G : réutilisé par orb_send (source unique du drapeau)
pub use state::RemoteAvatars;
