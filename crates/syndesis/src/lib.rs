// syndesis -- QUIC transport, TLS pairing, and renderer authentication
pub mod client;
pub mod clock;
pub mod error;
pub mod pairing;
pub mod protocol;
pub mod server;
pub mod tls;

pub use error::SyndesisError;
pub use pairing::{
    PairingOutcome, PairingRequest, complete_pairing, generate_api_key, hash_api_key,
    verify_api_key,
};
pub use protocol::session_frame::{
    Frame as SessionFrame, PairingChallenge, PairingComplete, SessionAccepted,
    SessionInit as SessionInitMsg, SessionRejected,
};
pub use server::auth::{
    SessionOutcome, build_pairing_challenge, build_pairing_complete, handle_session_init,
};
pub use tls::{SelfSignedCert, compute_fingerprint, generate_self_signed_simple};
