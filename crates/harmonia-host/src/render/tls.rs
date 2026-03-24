// TLS certificate management for QUIC transport.

use std::path::Path;
use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, SignatureScheme};
use snafu::ResultExt;

use super::error::{IoSnafu, RenderError};

pub fn load_or_generate_server_config(cert_dir: &Path) -> Result<quinn::ServerConfig, RenderError> {
    let cert_path = cert_dir.join("server.der");
    let key_path = cert_dir.join("server.key.der");

    let (cert_der, key_der) = if cert_path.exists() && key_path.exists() {
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
        let key_der = PrivatePkcs8KeyDer::from(certified.key_pair.serialize_der());
        std::fs::write(&cert_path, cert_der.as_ref()).context(IoSnafu)?;
        std::fs::write(&key_path, key_der.secret_pkcs8_der()).context(IoSnafu)?;
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

pub fn build_client_config() -> Result<quinn::ClientConfig, RenderError> {
    // WHY: For development and LAN use, skip server certificate verification.
    // Prompt 124 implements proper mDNS-based pairing with certificate pinning.
    let crypto = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(InsecureVerifier))
        .with_no_client_auth();

    let quic_config = quinn::crypto::rustls::QuicClientConfig::try_from(crypto).map_err(|e| {
        RenderError::Tls {
            message: e.to_string(),
            location: snafu::location!(),
        }
    })?;

    Ok(quinn::ClientConfig::new(Arc::new(quic_config)))
}

// WARNING: Accepts any server certificate. Replaced by certificate pinning in prompt 124.
#[derive(Debug)]
struct InsecureVerifier;

impl ServerCertVerifier for InsecureVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ED25519,
        ]
    }
}
