/// TLS certificate management for QUIC transport.
use std::fmt::Write as _;
use std::path::Path;
use std::sync::Arc;

use quinn::crypto::rustls::QuicServerConfig;
use rcgen::CertifiedKey;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName};
use snafu::ResultExt;

use crate::error::{self, CertGenSnafu, SyndesisError};

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
pub fn save_identity(
    cert_path: &Path,
    key_path: &Path,
    certs: &[CertificateDer<'_>],
    key: &PrivateKeyDer<'_>,
) -> Result<(), SyndesisError> {
    use std::fs;

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
    fs::write(key_path, key.secret_der()).context(error::IoSnafu)?;

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

/// TOFU certificate verifier that accepts any certificate on first contact.
#[derive(Debug)]
struct TofuVerifier;

impl rustls::client::danger::ServerCertVerifier for TofuVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Build a quinn ClientConfig that trusts any server certificate (TOFU model).
pub fn build_client_config() -> Result<quinn::ClientConfig, SyndesisError> {
    let mut tls_config = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(TofuVerifier))
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

    #[test]
    fn builds_client_config() {
        let config = build_client_config();
        assert!(config.is_ok());
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

    #[test]
    fn save_and_load_identity_round_trip() {
        let dir = std::env::temp_dir().join("syndesis_tls_test");
        let cert_path = dir.join("cert.der");
        let key_path = dir.join("key.der");

        let (certs, key) = generate_self_signed(&["localhost".to_string()]).unwrap();
        save_identity(&cert_path, &key_path, &certs, &key).unwrap();
        let (loaded_certs, _loaded_key) = load_identity(&cert_path, &key_path).unwrap();

        assert_eq!(certs[0].as_ref(), loaded_certs[0].as_ref());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
