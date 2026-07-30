/// TLS certificate management for QUIC transport.
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

use quinn::crypto::rustls::QuicServerConfig;
use rcgen::CertifiedKey;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use snafu::ResultExt;

use crate::error::{self, CertGenSnafu, SyndesisError};

// WHY: pure data — self-signed TLS certificate bundle.
/// A generated self-signed TLS certificate with its DER-encoded bytes and private key.
#[derive(Clone)]
pub struct SelfSignedCert {
    /// DER-encoded certificate bytes.
    pub cert_der: Vec<u8>,
    /// DER-encoded PKCS#8 private key bytes.
    pub key_der: Vec<u8>,
    /// Hex-encoded SHA-256 fingerprint of the certificate.
    pub fingerprint: String,
}

/// Compute the hex-encoded SHA-256 fingerprint of a DER-encoded certificate.
pub fn compute_fingerprint(cert_der: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(cert_der);
    hash.iter().fold(String::with_capacity(64), |mut acc, b| {
        if let Err(e) = write!(acc, "{b:02x}") {
            tracing::warn!(error = %e, "operation failed");
        }
        acc
    })
}

/// Generate a simple self-signed certificate returning `SelfSignedCert`.
pub fn generate_self_signed_simple(san: Vec<String>) -> Result<SelfSignedCert, SyndesisError> {
    let CertifiedKey { cert, signing_key } =
        rcgen::generate_simple_self_signed(san).context(CertGenSnafu)?;

    let cert_der = cert.der().to_vec();
    let key_der = signing_key.serialize_der();
    let fingerprint = compute_fingerprint(&cert_der);

    Ok(SelfSignedCert {
        cert_der,
        key_der,
        fingerprint,
    })
}

/// Generate a self-signed certificate and private key for QUIC transport.
pub fn generate_self_signed(
    subject_alt_names: &[String],
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), SyndesisError> {
    let mut params = rcgen::CertificateParams::new(subject_alt_names.to_vec()).map_err(|e| {
        error::TlsSnafu {
            reason: e.to_string(),
        }
        .build()
    })?;
    params.distinguished_name = rcgen::DistinguishedName::new();
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "syndesis");

    let key_pair = rcgen::KeyPair::generate().map_err(|e| {
        error::TlsSnafu {
            reason: e.to_string(),
        }
        .build()
    })?;
    let cert = params.self_signed(&key_pair).map_err(|e| {
        error::TlsSnafu {
            reason: e.to_string(),
        }
        .build()
    })?;

    let cert_der = CertificateDer::from(cert.der().to_vec());
    let key_der = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_pair.serialize_der()));

    Ok((vec![cert_der], key_der))
}

/// Save certificate and key to disk in DER format.
///
/// The private-key file is written with mode 0600; a key file that already
/// exists with a looser mode is tightened to 0600 as well.
pub fn save_identity(
    cert_path: &Path,
    key_path: &Path,
    certs: &[CertificateDer<'_>],
    key: &PrivateKeyDer<'_>,
) -> Result<(), SyndesisError> {
    use std::fs;
    use std::io::Write as _;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

    if let Some(parent) = cert_path.parent() {
        fs::create_dir_all(parent).context(error::IoSnafu)?;
    }
    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent).context(error::IoSnafu)?;
    }

    let mut cert_bytes = Vec::new();
    for cert in certs {
        cert_bytes.extend_from_slice(cert.as_ref());
    }
    fs::write(cert_path, &cert_bytes).context(error::IoSnafu)?;

    // WHY: mode(0o600) applies only at creation; the explicit set_permissions
    // afterwards repairs files that already existed with looser modes.
    let mut key_file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(key_path)
        .context(error::IoSnafu)?;
    key_file
        .write_all(key.secret_der())
        .context(error::IoSnafu)?;
    fs::set_permissions(key_path, fs::Permissions::from_mode(0o600)).context(error::IoSnafu)?;

    Ok(())
}

/// Load certificate and key FROM disk.
pub fn load_identity(
    cert_path: &Path,
    key_path: &Path,
) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>), SyndesisError> {
    use std::fs;

    let cert_bytes = fs::read(cert_path).context(error::IoSnafu)?;
    let key_bytes = fs::read(key_path).context(error::IoSnafu)?;

    let cert = CertificateDer::from(cert_bytes);
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(key_bytes));

    Ok((vec![cert], key))
}

/// Build a quinn ServerConfig with self-signed certs.
pub fn build_server_config(
    certs: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> Result<quinn::ServerConfig, SyndesisError> {
    let mut tls_config = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| {
            error::TlsSnafu {
                reason: e.to_string(),
            }
            .build()
        })?;

    tls_config.alpn_protocols = vec![b"syndesis/1".to_vec()];

    let quic_server_config = QuicServerConfig::try_from(tls_config).map_err(|e| {
        error::TlsSnafu {
            reason: e.to_string(),
        }
        .build()
    })?;

    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(quic_server_config));

    let mut transport = quinn::TransportConfig::default();
    transport.datagram_receive_buffer_size(Some(65536));
    transport.datagram_send_buffer_size(65536);
    server_config.transport_config(Arc::new(transport));

    Ok(server_config)
}

/// Fingerprint observed during a first-contact pairing handshake.
///
/// The pairing flow reads the value after connecting and MUST persist it;
/// every subsequent connection pins it via [`build_client_config`].
#[derive(Debug, Default)]
pub struct ObservedFingerprint {
    cell: std::sync::OnceLock<String>,
}

impl ObservedFingerprint {
    /// The hex-encoded SHA-256 fingerprint of the server certificate seen
    /// during pairing, once a handshake has completed.
    #[must_use]
    pub fn get(&self) -> Option<String> {
        self.cell.get().cloned()
    }
}

#[derive(Debug)]
enum VerifierMode {
    /// Reject any leaf whose SHA-256 fingerprint differs from the pin.
    Pinned(String),
    /// Explicit first-contact pairing: lock onto the first leaf observed and
    /// reject any later change for the verifier's lifetime.
    Pairing(Arc<ObservedFingerprint>),
}

/// TOFU certificate verifier: pins the server leaf certificate by SHA-256
/// fingerprint. Trust is established once (explicit pairing) and enforced on
/// every subsequent connection.
#[derive(Debug)]
struct TofuVerifier {
    mode: VerifierMode,
    algorithms: rustls::crypto::WebPkiSupportedAlgorithms,
}

impl TofuVerifier {
    fn pinned(fingerprint: String) -> Self {
        Self {
            mode: VerifierMode::Pinned(fingerprint),
            algorithms: rustls::crypto::ring::default_provider().signature_verification_algorithms,
        }
    }

    fn pairing(observed: Arc<ObservedFingerprint>) -> Self {
        Self {
            mode: VerifierMode::Pairing(observed),
            algorithms: rustls::crypto::ring::default_provider().signature_verification_algorithms,
        }
    }
}

impl rustls::client::danger::ServerCertVerifier for TofuVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        let presented = compute_fingerprint(end_entity.as_ref());
        let expected = match &self.mode {
            VerifierMode::Pinned(pin) => pin.clone(),
            // INVARIANT: get_or_init pins the first observed leaf, so a
            // mid-pairing certificate swap is rejected below.
            VerifierMode::Pairing(observed) => {
                observed.cell.get_or_init(|| presented.clone()).clone()
            }
        };
        if presented != expected {
            tracing::warn!(
                %presented,
                %expected,
                "server certificate fingerprint mismatch (TOFU violation)"
            );
            return Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ));
        }
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.algorithms)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.algorithms)
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        self.algorithms.supported_schemes()
    }
}

// WHY: fail closed — a malformed pin must never degrade into accept-any.
fn normalize_pin(pinned_fingerprint: &str) -> Result<String, SyndesisError> {
    let pin = pinned_fingerprint.trim().to_ascii_lowercase();
    if pin.len() != 64 || !pin.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(error::TlsSnafu {
            reason: "pinned fingerprint must be 64 hex chars (SHA-256)".to_string(),
        }
        .build());
    }
    Ok(pin)
}

fn client_config_with_verifier(
    verifier: Arc<TofuVerifier>,
) -> Result<quinn::ClientConfig, SyndesisError> {
    let mut tls_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_no_client_auth();

    tls_config.alpn_protocols = vec![b"syndesis/1".to_vec()];

    let mut client_config = quinn::ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls_config).map_err(|e| {
            error::TlsSnafu {
                reason: e.to_string(),
            }
            .build()
        })?,
    ));

    let mut transport = quinn::TransportConfig::default();
    transport.datagram_receive_buffer_size(Some(65536));
    transport.datagram_send_buffer_size(65536);
    client_config.transport_config(Arc::new(transport));

    Ok(client_config)
}

/// Build a quinn ClientConfig pinned to a known server-certificate fingerprint.
///
/// `pinned_fingerprint` is the hex-encoded SHA-256 fingerprint persisted during
/// pairing (`PairingChallenge.cert_fingerprint` or renderer-local storage).
/// Connections to a server presenting any other leaf certificate fail the
/// handshake. A missing pin has no accept-any fallback; first contact goes
/// through [`build_pairing_client_config`] instead.
pub fn build_client_config(pinned_fingerprint: &str) -> Result<quinn::ClientConfig, SyndesisError> {
    let pin = normalize_pin(pinned_fingerprint)?;
    client_config_with_verifier(Arc::new(TofuVerifier::pinned(pin)))
}

/// Build a quinn ClientConfig for the explicit first-contact pairing step.
///
/// The verifier locks onto the first server leaf observed and rejects any
/// change for its lifetime. The caller MUST read the fingerprint from the
/// returned [`ObservedFingerprint`] after the handshake, persist it, and use
/// [`build_client_config`] for every subsequent connection.
pub fn build_pairing_client_config()
-> Result<(quinn::ClientConfig, Arc<ObservedFingerprint>), SyndesisError> {
    let observed = Arc::new(ObservedFingerprint::default());
    let config =
        client_config_with_verifier(Arc::new(TofuVerifier::pairing(Arc::clone(&observed))))?;
    Ok((config, observed))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_self_signed_cert() {
        let (certs, _key) = generate_self_signed(&["localhost".to_string()]).unwrap();
        assert_eq!(certs.len(), 1);
        assert!(!certs[0].as_ref().is_empty());
    }

    #[test]
    fn builds_server_config_from_generated_cert() {
        let (certs, key) = generate_self_signed(&["localhost".to_string()]).unwrap();
        let config = build_server_config(certs, key);
        assert!(config.is_ok());
    }

    fn verify_leaf(
        verifier: &TofuVerifier,
        cert_der: &[u8],
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        use rustls::client::danger::ServerCertVerifier as _;
        let leaf = CertificateDer::from(cert_der.to_vec());
        let name = ServerName::try_from("localhost").expect("valid server name");
        verifier.verify_server_cert(&leaf, &[], &name, &[], rustls::pki_types::UnixTime::now())
    }

    #[test]
    fn builds_client_config_with_valid_pin() {
        let cert = generate_self_signed_simple(vec!["localhost".to_string()]).unwrap();
        let config = build_client_config(&cert.fingerprint);
        assert!(config.is_ok());
    }

    #[test]
    fn build_client_config_rejects_malformed_pin() {
        assert!(build_client_config("").is_err(), "empty pin must fail");
        assert!(
            build_client_config("not-hex").is_err(),
            "non-hex pin must fail"
        );
        assert!(
            build_client_config("abc123").is_err(),
            "short pin must fail"
        );
        assert!(
            build_client_config(&"g".repeat(64)).is_err(),
            "64 non-hex chars must fail"
        );
    }

    #[test]
    fn pinned_verifier_accepts_matching_fingerprint() {
        let cert = generate_self_signed_simple(vec!["localhost".to_string()]).unwrap();
        let verifier = TofuVerifier::pinned(cert.fingerprint.clone());
        assert!(verify_leaf(&verifier, &cert.cert_der).is_ok());
    }

    #[test]
    fn pinned_verifier_rejects_mismatched_fingerprint() {
        let pinned = generate_self_signed_simple(vec!["localhost".to_string()]).unwrap();
        let imposter = generate_self_signed_simple(vec!["localhost".to_string()]).unwrap();
        let verifier = TofuVerifier::pinned(pinned.fingerprint.clone());
        let result = verify_leaf(&verifier, &imposter.cert_der);
        assert!(
            matches!(result, Err(rustls::Error::InvalidCertificate(_))),
            "mismatched leaf must be rejected, got {result:?}"
        );
    }

    #[test]
    fn pairing_verifier_locks_first_leaf_and_rejects_change() {
        let first = generate_self_signed_simple(vec!["localhost".to_string()]).unwrap();
        let second = generate_self_signed_simple(vec!["localhost".to_string()]).unwrap();

        let observed = Arc::new(ObservedFingerprint::default());
        let verifier = TofuVerifier::pairing(Arc::clone(&observed));

        assert!(observed.get().is_none(), "no fingerprint before handshake");
        assert!(verify_leaf(&verifier, &first.cert_der).is_ok());
        assert_eq!(
            observed.get().as_deref(),
            Some(first.fingerprint.as_str()),
            "observed cell must expose the first leaf's fingerprint"
        );

        let swap = verify_leaf(&verifier, &second.cert_der);
        assert!(
            matches!(swap, Err(rustls::Error::InvalidCertificate(_))),
            "mid-pairing certificate swap must be rejected"
        );
        // Re-presenting the locked leaf still verifies.
        assert!(verify_leaf(&verifier, &first.cert_der).is_ok());
    }

    #[test]
    fn generate_simple_produces_fingerprint() {
        let cert = generate_self_signed_simple(vec!["harmonia.local".to_string()]).unwrap();
        assert!(!cert.cert_der.is_empty());
        assert!(!cert.key_der.is_empty());
        assert_eq!(cert.fingerprint.len(), 64);
        assert!(cert.fingerprint.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn fingerprint_is_deterministic() {
        let cert = generate_self_signed_simple(vec!["harmonia.local".to_string()]).unwrap();
        let fp2 = compute_fingerprint(&cert.cert_der);
        assert_eq!(cert.fingerprint, fp2);
    }

    #[test]
    fn different_certs_have_different_fingerprints() {
        let c1 = generate_self_signed_simple(vec!["a.local".to_string()]).unwrap();
        let c2 = generate_self_signed_simple(vec!["b.local".to_string()]).unwrap();
        assert_ne!(c1.fingerprint, c2.fingerprint);
    }

    // WARNING: these tests write TLS private key material, so each one must
    // own its directory. A fixed `env::temp_dir().join(...)` path is shared
    // by every account on the box — whichever user creates it first owns it
    // and every other user's run fails on permissions, and two concurrent
    // runs delete each other's key files through the trailing cleanup.
    #[test]
    fn save_and_load_identity_round_trip() {
        let dir = tempfile::TempDir::new().unwrap();
        let cert_path = dir.path().join("cert.der");
        let key_path = dir.path().join("key.der");

        let (certs, key) = generate_self_signed(&["localhost".to_string()]).unwrap();
        save_identity(&cert_path, &key_path, &certs, &key).unwrap();
        let (loaded_certs, _loaded_key) = load_identity(&cert_path, &key_path).unwrap();

        assert_eq!(certs[0].as_ref(), loaded_certs[0].as_ref());
    }

    #[test]
    fn save_identity_sets_key_file_permissions_0600() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::TempDir::new().unwrap();
        let cert_path = dir.path().join("cert.der");
        let key_path = dir.path().join("key.der");

        let (certs, key) = generate_self_signed(&["localhost".to_string()]).unwrap();
        save_identity(&cert_path, &key_path, &certs, &key).unwrap();

        let mode = std::fs::metadata(&key_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "fresh key file must be 0600, got {mode:o}");
    }

    #[test]
    fn save_identity_repairs_loose_key_file_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::TempDir::new().unwrap();
        let cert_path = dir.path().join("cert.der");
        let key_path = dir.path().join("key.der");
        std::fs::write(&key_path, b"stale").unwrap();
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o644)).unwrap();

        let (certs, key) = generate_self_signed(&["localhost".to_string()]).unwrap();
        save_identity(&cert_path, &key_path, &certs, &key).unwrap();

        let mode = std::fs::metadata(&key_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "loose key file must be repaired to 0600");
    }
}
