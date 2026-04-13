// Session authentication middleware for incoming renderer connections
use sqlx::SqlitePool;

use apotheke::repo::renderer::Renderer;

use crate::error::SyndesisError;
use crate::pairing::handshake::{
    PairingOutcome, PairingRequest, authenticate_renderer, complete_pairing,
};
use crate::protocol::session_frame::{
    PairingChallenge, PairingComplete, SessionInit as SessionInitMsg,
};

/// Outcome of processing a `SessionInit` frame.
pub enum SessionOutcome {
    /// Renderer authenticated with an existing API key.
    Authenticated(Renderer),
    /// First connection: pairing completed, key should be sent to renderer.
    Paired(PairingOutcome),
}

/// Process a `SessionInit` frame from a connecting renderer.
///
/// - `is_new: true` -> run the pairing flow (generate + store API key).
/// - `api_key: Some(key)` -> verify against the renderer registry.
/// - Neither -> reject with `InvalidApiKey`.
// WHY: peer_cert_fingerprint is the renderer's TLS cert fingerprint stored for TOFU.
// The caller is responsible for building and sending PairingChallenge (with the server's
// own cert fingerprint) after receiving a Paired outcome from this function.
pub async fn handle_session_init(
    read_pool: &SqlitePool,
    write_pool: &SqlitePool,
    init: &SessionInitMsg,
    peer_cert_fingerprint: &str,
) -> Result<SessionOutcome, SyndesisError> {
    if init.is_new {
        let req = PairingRequest {
            renderer_name: &init.renderer_name,
            renderer_id: &init.renderer_id,
            cert_fingerprint: peer_cert_fingerprint,
        };
        let outcome = complete_pairing(write_pool, req).await?;
        return Ok(SessionOutcome::Paired(outcome));
    }

    match &init.api_key {
        Some(key) => {
            let renderer =
                authenticate_renderer(read_pool, write_pool, key, peer_cert_fingerprint).await?;
            Ok(SessionOutcome::Authenticated(renderer))
        }
        None => Err(SyndesisError::InvalidApiKey {
            location: snafu::location!(),
        }),
    }
}

/// Build the `PairingChallenge` frame the server sends during pairing.
pub fn build_pairing_challenge(
    server_name: &str,
    server_cert_fingerprint: &str,
) -> PairingChallenge {
    PairingChallenge {
        server_name: server_name.to_string(),
        cert_fingerprint: server_cert_fingerprint.to_string(),
    }
}

/// Build the `PairingComplete` frame from a `PairingOutcome`.
pub fn build_pairing_complete(outcome: &PairingOutcome) -> PairingComplete {
    PairingComplete {
        api_key: outcome.api_key.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::session_frame::SessionInit as SessionInitMsg;
    use apotheke::migrate::MIGRATOR;
    use sqlx::SqlitePool;

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn renderer_id() -> String {
        uuid::Uuid::now_v7().to_string()
    }

    #[tokio::test]
    async fn pairing_flow_completes_and_key_verifiable() {
        let pool = setup().await;
        let id = renderer_id();

        let init = SessionInitMsg {
            renderer_name: "Test Renderer".to_string(),
            renderer_id: id.clone(),
            api_key: None,
            is_new: true,
        };

        let outcome = handle_session_init(&pool, &pool, &init, "aabbcc")
            .await
            .unwrap();

        let api_key = match outcome {
            SessionOutcome::Paired(o) => o.api_key,
            SessionOutcome::Authenticated(_) => panic!("expected paired"),
        };
        assert!(!api_key.is_empty());
    }

    #[tokio::test]
    async fn authenticate_after_pairing_succeeds() {
        let pool = setup().await;
        let id = renderer_id();

        let init = SessionInitMsg {
            renderer_name: "Test Renderer".to_string(),
            renderer_id: id.clone(),
            api_key: None,
            is_new: true,
        };

        let api_key = match handle_session_init(&pool, &pool, &init, "fingerprint_renderer")
            .await
            .unwrap()
        {
            SessionOutcome::Paired(o) => o.api_key,
            _ => panic!("expected paired"),
        };

        let auth_init = SessionInitMsg {
            renderer_name: "Test Renderer".to_string(),
            renderer_id: id.clone(),
            api_key: Some(api_key.clone()),
            is_new: false,
        };

        let result = handle_session_init(&pool, &pool, &auth_init, "fingerprint_renderer").await;

        assert!(result.is_ok());
        match result.unwrap() {
            SessionOutcome::Authenticated(r) => assert_eq!(r.id, id),
            _ => panic!("expected authenticated"),
        }
    }

    #[tokio::test]
    async fn invalid_api_key_rejected() {
        let pool = setup().await;
        let id = renderer_id();

        let init = SessionInitMsg {
            renderer_name: "Test Renderer".to_string(),
            renderer_id: id,
            api_key: None,
            is_new: true,
        };

        handle_session_init(&pool, &pool, &init, "fp")
            .await
            .unwrap();

        let auth_init = SessionInitMsg {
            renderer_name: "Test Renderer".to_string(),
            renderer_id: uuid::Uuid::now_v7().to_string(),
            api_key: Some("wrong-key-value-here".to_string()),
            is_new: false,
        };

        let result = handle_session_init(&pool, &pool, &auth_init, "fp").await;

        assert!(matches!(result, Err(SyndesisError::InvalidApiKey { .. })));
    }

    #[tokio::test]
    async fn disabled_renderer_rejected() {
        use apotheke::repo::renderer;

        let pool = setup().await;
        let id = renderer_id();

        let init = SessionInitMsg {
            renderer_name: "Disabled Renderer".to_string(),
            renderer_id: id.clone(),
            api_key: None,
            is_new: true,
        };

        let api_key = match handle_session_init(&pool, &pool, &init, "fp")
            .await
            .unwrap()
        {
            SessionOutcome::Paired(o) => o.api_key,
            _ => panic!("expected paired"),
        };

        renderer::set_enabled(&pool, &id, false).await.unwrap();

        let auth_init = SessionInitMsg {
            renderer_name: "Disabled Renderer".to_string(),
            renderer_id: id,
            api_key: Some(api_key),
            is_new: false,
        };

        let result = handle_session_init(&pool, &pool, &auth_init, "fp").await;

        assert!(matches!(
            result,
            Err(SyndesisError::RendererDisabled { .. })
        ));
    }

    #[tokio::test]
    async fn fingerprint_mismatch_rejected() {
        let pool = setup().await;
        let id = renderer_id();

        let init = SessionInitMsg {
            renderer_name: "Test Renderer".to_string(),
            renderer_id: id.clone(),
            api_key: None,
            is_new: true,
        };

        let api_key = match handle_session_init(&pool, &pool, &init, "original-fingerprint")
            .await
            .unwrap()
        {
            SessionOutcome::Paired(o) => o.api_key,
            _ => panic!("expected paired"),
        };

        let auth_init = SessionInitMsg {
            renderer_name: "Test Renderer".to_string(),
            renderer_id: id,
            api_key: Some(api_key),
            is_new: false,
        };

        let result = handle_session_init(&pool, &pool, &auth_init, "different-fingerprint").await;

        assert!(matches!(
            result,
            Err(SyndesisError::FingerprintMismatch { .. })
        ));
    }
}
