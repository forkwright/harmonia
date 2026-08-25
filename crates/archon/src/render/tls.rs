// TLS certificate management for QUIC transport.

use std::path::Path;
use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::crypto::WebPkiSupportedAlgorithms;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use sha2::{Digest, Sha256};
use snafu::ResultExt;
use subtle::ConstantTimeEq;

use super::error::{FingerprintSnafu, IoSnafu, RenderError};

pub fn load_or_generate_server_config(cert_dir: &Path) -> Result<quinn::ServerConfig, RenderError> {
    let cert_path = cert_dir.join("server.der");
    let key_path = cert_dir.join("server.key.der");

    let (cert_der, key_der) = if cert_path.exists() && key_path.exists() {
        let repaired = super::secret::ensure_secret_file_mode(&key_path).context(IoSnafu)?;
        if repaired {
            tracing::warn!(
                path = %key_path.display(),
                "TLS key file had group/other-readable permissions; tightened to 0600"
            );
        }
        let cert = std::fs::read(&cert_path).context(IoSnafu)?;
        let key = std::fs::read(&key_path).context(IoSnafu)?;
        (CertificateDer::from(cert), PrivatePkcs8KeyDer::from(key))
    } else {
        std::fs::create_dir_all(cert_dir).context(IoSnafu)?;
        let certified =
            rcgen::generate_simple_self_signed(vec!["harmonia".into()]).map_err(|e| {
                RenderError::Tls {
                    message: e.to_string(),
                    location: snafu::location!(),
                }
            })?;
        let cert_der = certified.cert.der().clone();
        let key_der = PrivatePkcs8KeyDer::from(certified.signing_key.serialize_der());
        std::fs::write(&cert_path, cert_der.as_ref()).context(IoSnafu)?;
        super::secret::write_secret_file(&key_path, key_der.secret_pkcs8_der()).context(IoSnafu)?;
        tracing::info!(cert_dir = %cert_dir.display(), "generated self-signed TLS certificate");
        (cert_der, key_der)
    };

    quinn::ServerConfig::with_single_cert(vec![cert_der], PrivateKeyDer::Pkcs8(key_der)).map_err(
        |e| RenderError::Tls {
            message: e.to_string(),
            location: snafu::location!(),
        },
    )
}

/// Build a QUIC client config that trusts exactly one server certificate:
/// the one whose SHA-256 fingerprint matches `server_fingerprint` (64 hex chars,
/// pinned at pairing time in `credentials.toml`).
///
/// A malformed or empty fingerprint fails closed here rather than falling back
/// to any weaker trust model.
pub fn build_client_config(server_fingerprint: &str) -> Result<quinn::ClientConfig, RenderError> {
    let expected = parse_fingerprint_hex(server_fingerprint).ok_or_else(|| {
        FingerprintSnafu {
            message: format!(
                "pinned server fingerprint must be 64 hex chars, got {} chars",
                server_fingerprint.len()
            ),
        }
        .build()
    })?;

    // WHY: .dangerous() only swaps the WebPKI verifier for the pinning verifier
    // below; the pin plus real handshake-signature checks carry the trust.
    //
    // WHY builder_with_provider, not the plain builder(): the plain builder()
    // asks rustls to auto-select a CryptoProvider from crate features, valid
    // only when EXACTLY ONE of "ring"/"aws-lc-rs" is active for the whole
    // binary. Nothing pulls aws-lc-rs into this workspace's build today, so
    // the plain builder() happens to resolve unambiguously right now — but
    // that is an accident of the current dependency graph, not a guarantee:
    // librqbit 9 (landing next) forwards its `default` feature to
    // `reqwest/default-tls`, which in reqwest 0.13 means `rustls`, which
    // pulls `__rustls-aws-lc-rs` — the exact combination that made this same
    // call panic on the librqbit-9 branch. This crate's own fleet convention
    // (see main.rs's install_default call) is explicit provider selection,
    // never implicit — so pin `ring` here explicitly rather than let this
    // call keep working by chance until the next dependency bump breaks it.
    let crypto = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .expect("ring's default provider supports rustls's default TLS versions")
    .dangerous()
    .with_custom_certificate_verifier(Arc::new(PinnedFingerprintVerifier::new(expected)))
    .with_no_client_auth();

    let quic_config = quinn::crypto::rustls::QuicClientConfig::try_from(crypto).map_err(|e| {
        RenderError::Tls {
            message: e.to_string(),
            location: snafu::location!(),
        }
    })?;

    Ok(quinn::ClientConfig::new(Arc::new(quic_config)))
}

fn parse_fingerprint_hex(hex: &str) -> Option<[u8; 32]> {
    if hex.len() != 64 || !hex.is_ascii() {
        return None;
    }
    let mut out = [0u8; 32];
    for (byte, chunk) in out.iter_mut().zip(hex.as_bytes().chunks_exact(2)) {
        let hi = char::from(chunk[0]).to_digit(16)?;
        let lo = char::from(chunk[1]).to_digit(16)?;
        *byte = u8::try_from(hi * 16 + lo).ok()?;
    }
    Some(out)
}

/// Certificate verifier that accepts only the server certificate whose
/// SHA-256 digest matches the pinned fingerprint.
///
/// Handshake signatures still go through real cryptographic verification
/// (`rustls::crypto::verify_tls1x_signature`), so presenting the pinned
/// certificate without holding its private key fails the handshake.
#[derive(Debug)]
struct PinnedFingerprintVerifier {
    expected: [u8; 32],
    supported: WebPkiSupportedAlgorithms,
}

impl PinnedFingerprintVerifier {
    fn new(expected: [u8; 32]) -> Self {
        Self {
            expected,
            supported: rustls::crypto::ring::default_provider().signature_verification_algorithms,
        }
    }
}

impl ServerCertVerifier for PinnedFingerprintVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let actual = Sha256::digest(end_entity.as_ref());
        if bool::from(actual.as_slice().ct_eq(&self.expected)) {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(message, cert, dss, &self.supported)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(message, cert, dss, &self.supported)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.supported.supported_schemes()
    }
}

#[cfg(test)]
mod tests {
    use std::os::unix::fs::PermissionsExt;

    use tempfile::TempDir;

    use super::*;

    fn self_signed_cert() -> CertificateDer<'static> {
        rcgen::generate_simple_self_signed(vec!["harmonia".into()])
            .expect("generate cert")
            .cert
            .der()
            .clone()
    }

    fn fingerprint_of(cert: &CertificateDer<'_>) -> [u8; 32] {
        Sha256::digest(cert.as_ref()).into()
    }

    fn verify(
        verifier: &PinnedFingerprintVerifier,
        cert: &CertificateDer<'_>,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let server_name = ServerName::try_from("harmonia").expect("server name");
        verifier.verify_server_cert(cert, &[], &server_name, &[], UnixTime::now())
    }

    #[test]
    fn pinned_verifier_accepts_matching_fingerprint() {
        let cert = self_signed_cert();
        let verifier = PinnedFingerprintVerifier::new(fingerprint_of(&cert));

        assert!(verify(&verifier, &cert).is_ok());
    }

    #[test]
    fn pinned_verifier_rejects_mismatched_fingerprint() {
        let cert = self_signed_cert();
        let verifier = PinnedFingerprintVerifier::new([0u8; 32]);

        let result = verify(&verifier, &cert);

        assert!(matches!(
            result,
            Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure
            ))
        ));
    }

    #[test]
    fn pinned_verifier_rejects_other_cert_with_same_name() {
        let cert_a = self_signed_cert();
        let cert_b = self_signed_cert();
        let verifier = PinnedFingerprintVerifier::new(fingerprint_of(&cert_a));

        assert!(verify(&verifier, &cert_b).is_err());
    }

    #[test]
    fn build_client_config_accepts_valid_fingerprint() {
        let hex = "ab".repeat(32);
        assert!(build_client_config(&hex).is_ok());
    }

    #[test]
    fn build_client_config_rejects_malformed_fingerprint() {
        assert!(build_client_config("").is_err());
        assert!(build_client_config("abcd").is_err());
        assert!(build_client_config(&"zz".repeat(32)).is_err());
    }

    #[test]
    fn parse_fingerprint_hex_round_trips() {
        let cert = self_signed_cert();
        let expected = fingerprint_of(&cert);
        let hex: String = expected.iter().fold(String::new(), |mut s, b| {
            use std::fmt::Write as _;
            // WHY: fmt::Write on String is infallible; ok() avoids unused-result warning
            write!(s, "{b:02x}").ok();
            s
        });

        assert_eq!(parse_fingerprint_hex(&hex), Some(expected));
    }

    #[test]
    fn generated_server_key_has_mode_0600() {
        let dir = TempDir::new().expect("tempdir");

        load_or_generate_server_config(dir.path()).expect("generate");

        let mode = std::fs::metadata(dir.path().join("server.key.der"))
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn load_repairs_loose_key_permissions() {
        let dir = TempDir::new().expect("tempdir");
        load_or_generate_server_config(dir.path()).expect("generate");
        let key_path = dir.path().join("server.key.der");
        std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o644)).expect("chmod");

        load_or_generate_server_config(dir.path()).expect("reload");

        let mode = std::fs::metadata(&key_path)
            .expect("metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }
}
