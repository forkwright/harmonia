// Session authentication middleware for incoming renderer connections
use apotheke::repo::renderer::Renderer;
use sqlx::SqlitePool;

use crate::error::SyndesisError;
use crate::pairing::handshake::{
    PairingOutcome, PairingRequest, authenticate_renderer, complete_pairing,
};
use crate::protocol::session_frame::{
    PairingChallenge, PairingComplete, SessionInit as SessionInitMsg,
};

/// Outcome of processing a `SessionInit` frame.
#[non_exhaustive]
pub enum SessionOutcome {
    /// Renderer authenticated with an existing API key.
    Authenticated(Renderer),
    /// First connection: pairing completed, key should be sent to renderer.
    Paired(PairingOutcome),
}

/// Admission policy for the pairing flow: an on/off switch plus a global
/// fixed-window rate limit on attempts.
///
/// WHY: `is_new` pairing mints an API key for an unauthenticated peer —
/// without a gate any network peer can enroll unlimited renderers.
pub struct PairingGate {
    enabled: bool,
    max_attempts: u32,
    window: std::time::Duration,
    state: std::sync::Mutex<PairingWindow>,
}

struct PairingWindow {
    started: std::time::Instant,
    attempts: u32,
}

impl PairingGate {
    /// Build the gate from server config (`pairing_enabled`,
    /// `pairing_max_attempts_per_min`).
    #[must_use]
    pub fn from_config(config: &crate::config::ServerConfig) -> Self {
        Self {
            enabled: config.pairing_enabled,
            max_attempts: config.pairing_max_attempts_per_min,
            window: std::time::Duration::from_secs(60),
            state: std::sync::Mutex::new(PairingWindow {
                started: std::time::Instant::now(),
                attempts: 0,
            }),
        }
    }

    /// Admit or reject one pairing attempt.
    pub fn admit(&self) -> Result<(), SyndesisError> {
        if !self.enabled {
            return Err(SyndesisError::PairingDisabled {
                location: snafu::location!(),
            });
        }
        // WHY: poisoning is impossible to act on here — the guarded state is
        // two plain integers, safe to reuse after a panicked writer.
        let mut window = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if window.started.elapsed() >= self.window {
            window.started = std::time::Instant::now();
            window.attempts = 0;
        }
        if window.attempts >= self.max_attempts {
            return Err(SyndesisError::PairingRateLimited {
                location: snafu::location!(),
            });
        }
        window.attempts += 1;
        Ok(())
    }
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
    pairing_gate: &PairingGate,
) -> Result<SessionOutcome, SyndesisError> {
    if init.is_new {
        pairing_gate.admit()?;
        let req = PairingRequest {
            renderer_name: &init.renderer_name,
            renderer_id: &init.renderer_id.0,
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
    use apotheke::migrate::MIGRATOR;
    use sqlx::SqlitePool;

    use super::*;
    use crate::protocol::session_frame::{RendererSyncId, SessionInit as SessionInitMsg};

    async fn setup() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        MIGRATOR.run(&pool).await.unwrap();
        pool
    }

    fn open_gate() -> PairingGate {
        PairingGate::from_config(&crate::config::ServerConfig::default())
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
            renderer_id: RendererSyncId(id.clone()),
            api_key: None,
            is_new: true,
        };

        let outcome = handle_session_init(&pool, &pool, &init, "aabbcc", &open_gate())
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
            renderer_id: RendererSyncId(id.clone()),
            api_key: None,
            is_new: true,
        };

        let api_key =
            match handle_session_init(&pool, &pool, &init, "fingerprint_renderer", &open_gate())
                .await
                .unwrap()
            {
                SessionOutcome::Paired(o) => o.api_key,
                _ => panic!("expected paired"),
            };

        let auth_init = SessionInitMsg {
            renderer_name: "Test Renderer".to_string(),
            renderer_id: RendererSyncId(id.clone()),
            api_key: Some(api_key.clone()),
            is_new: false,
        };

        let result = handle_session_init(
            &pool,
            &pool,
            &auth_init,
            "fingerprint_renderer",
            &open_gate(),
        )
        .await;

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
            renderer_id: RendererSyncId(id),
            api_key: None,
            is_new: true,
        };

        handle_session_init(&pool, &pool, &init, "fp", &open_gate())
            .await
            .unwrap();

        let auth_init = SessionInitMsg {
            renderer_name: "Test Renderer".to_string(),
            renderer_id: RendererSyncId(uuid::Uuid::now_v7().to_string()),
            api_key: Some("wrong-key-value-here".to_string()),
            is_new: false,
        };

        let result = handle_session_init(&pool, &pool, &auth_init, "fp", &open_gate()).await;

        assert!(matches!(result, Err(SyndesisError::InvalidApiKey { .. })));
    }

    #[tokio::test]
    async fn disabled_renderer_rejected() {
        use apotheke::repo::renderer;

        let pool = setup().await;
        let id = renderer_id();

        let init = SessionInitMsg {
            renderer_name: "Disabled Renderer".to_string(),
            renderer_id: RendererSyncId(id.clone()),
            api_key: None,
            is_new: true,
        };

        let api_key = match handle_session_init(&pool, &pool, &init, "fp", &open_gate())
            .await
            .unwrap()
        {
            SessionOutcome::Paired(o) => o.api_key,
            _ => panic!("expected paired"),
        };

        renderer::set_enabled(&pool, &id, false).await.unwrap();

        let auth_init = SessionInitMsg {
            renderer_name: "Disabled Renderer".to_string(),
            renderer_id: RendererSyncId(id),
            api_key: Some(api_key),
            is_new: false,
        };

        let result = handle_session_init(&pool, &pool, &auth_init, "fp", &open_gate()).await;

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
            renderer_id: RendererSyncId(id.clone()),
            api_key: None,
            is_new: true,
        };

        let api_key =
            match handle_session_init(&pool, &pool, &init, "original-fingerprint", &open_gate())
                .await
                .unwrap()
            {
                SessionOutcome::Paired(o) => o.api_key,
                _ => panic!("expected paired"),
            };

        let auth_init = SessionInitMsg {
            renderer_name: "Test Renderer".to_string(),
            renderer_id: RendererSyncId(id),
            api_key: Some(api_key),
            is_new: false,
        };

        let result = handle_session_init(
            &pool,
            &pool,
            &auth_init,
            "different-fingerprint",
            &open_gate(),
        )
        .await;

        assert!(matches!(
            result,
            Err(SyndesisError::FingerprintMismatch { .. })
        ));
    }

    #[tokio::test]
    async fn pairing_rejected_when_disabled() {
        let pool = setup().await;
        let gate = PairingGate::from_config(&crate::config::ServerConfig {
            pairing_enabled: false,
            ..crate::config::ServerConfig::default()
        });

        let init = SessionInitMsg {
            renderer_name: "Test Renderer".to_string(),
            renderer_id: RendererSyncId(renderer_id()),
            api_key: None,
            is_new: true,
        };

        let result = handle_session_init(&pool, &pool, &init, "fp", &gate).await;
        assert!(matches!(result, Err(SyndesisError::PairingDisabled { .. })));
    }

    #[tokio::test]
    async fn pairing_rate_limited_after_n_attempts() {
        let pool = setup().await;
        let gate = PairingGate::from_config(&crate::config::ServerConfig {
            pairing_max_attempts_per_min: 2,
            ..crate::config::ServerConfig::default()
        });

        for i in 0..2 {
            let init = SessionInitMsg {
                renderer_name: format!("Renderer {i}"),
                renderer_id: RendererSyncId(renderer_id()),
                api_key: None,
                is_new: true,
            };
            let result = handle_session_init(&pool, &pool, &init, "fp", &gate).await;
            assert!(result.is_ok(), "attempt {i} within the budget must pass");
        }

        let init = SessionInitMsg {
            renderer_name: "Renderer over budget".to_string(),
            renderer_id: RendererSyncId(renderer_id()),
            api_key: None,
            is_new: true,
        };
        let result = handle_session_init(&pool, &pool, &init, "fp", &gate).await;
        assert!(matches!(
            result,
            Err(SyndesisError::PairingRateLimited { .. })
        ));
    }
}
