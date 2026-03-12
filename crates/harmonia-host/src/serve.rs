use std::sync::Arc;

use snafu::ResultExt;
use tokio::signal::unix::SignalKind;
use tracing::info;

use epignosis::{EpignosisService, resolver::ProviderCredentials};
use exousia::ExousiaServiceImpl;
use harmonia_common::create_event_bus;
use harmonia_db::init_pools;
use horismos::ConfigManager;
use komide::{KomideService, scheduler::FeedScheduler};
use kritike::DefaultCurationService;
use paroche::state::{AppState, DynCurationService, DynMetadataResolver};
use taxis::ScannerManager;

use crate::cli::ServeArgs;
use crate::error::{
    ConfigSnafu, DatabaseSnafu, FeedSchedulerSnafu, HostError, ScannerSnafu, ServerSnafu,
};
use crate::shutdown::shutdown_signal;
use crate::startup::{ensure_admin_user, init_tracing};

struct NullCuration;
impl DynCurationService for NullCuration {}

struct NullMetadata;
impl DynMetadataResolver for NullMetadata {}

pub async fn run_serve(args: ServeArgs) -> Result<(), HostError> {
    // 1. Load config
    let (mut config, warnings) =
        horismos::load_config(Some(args.config.as_path())).context(ConfigSnafu)?;

    for w in &warnings {
        eprintln!("config warning: [{}] {}", w.field, w.message);
    }

    // Apply CLI overrides
    if let Some(ref listen) = args.listen {
        config.paroche.listen_addr = listen.clone();
    }
    if let Some(port) = args.port {
        config.paroche.port = port;
    }

    // 2. Initialize tracing
    init_tracing(&config)?;

    for w in &warnings {
        tracing::warn!(field = %w.field, "{}", w.message);
    }

    // 3. Set up ConfigManager for hot-reload
    let config_path = args.config.clone();
    let (config_manager, _config_handle) = ConfigManager::new(config.clone(), config_path);

    // SIGHUP handler for config reload
    let manager_for_reload = config_manager.clone();
    tokio::spawn(async move {
        let mut sighup = tokio::signal::unix::signal(SignalKind::hangup())
            .expect("failed to register SIGHUP handler");
        loop {
            sighup.recv().await;
            tracing::info!("SIGHUP received — reloading configuration");
            match manager_for_reload.reload() {
                Ok(reload_warnings) => {
                    for w in reload_warnings {
                        tracing::warn!(field = %w.field, "config reload: {}", w.message);
                    }
                    tracing::info!("configuration reloaded");
                }
                Err(e) => {
                    tracing::error!("config reload failed: {e} — keeping current config");
                }
            }
        }
    });

    let config = Arc::new(config);

    // 4. Create database pools
    let db_path = config.database.db_path.to_string_lossy();
    let db = Arc::new(init_pools(&db_path).await.context(DatabaseSnafu)?);

    // 5. Create Aggelia event bus
    let (event_tx, _event_rx) = create_event_bus(config.aggelia.buffer_size);

    // 6. Create auth service
    let auth = Arc::new(ExousiaServiceImpl::new(db.clone(), config.exousia.clone()));

    // 7. First-run admin setup
    ensure_admin_user(&auth, &db).await?;

    // 8. Create metadata resolver
    let _metadata_service =
        EpignosisService::new(config.epignosis.clone(), ProviderCredentials::default());

    // 9. Create curation service
    let _curation_service = DefaultCurationService::new(db.read.clone(), event_tx.clone());

    // 10. Start scanner — background task
    let scanner = ScannerManager::start(&config.taxis, event_tx.clone())
        .await
        .context(ScannerSnafu)?;

    // 11. Start feed scheduler — background task
    let komide_service = Arc::new(KomideService::new(
        harmonia_db::DbPools {
            read: db.read.clone(),
            write: db.write.clone(),
        },
        event_tx.clone(),
        reqwest::Client::new(),
        config.komide.clone(),
    ));
    let feed_scheduler = FeedScheduler::start(
        komide_service,
        config.komide.clone(),
        harmonia_db::DbPools {
            read: db.read.clone(),
            write: db.write.clone(),
        },
    )
    .await
    .context(FeedSchedulerSnafu)?;

    // 12. Build import service adapter for paroche
    let import = paroche::state::make_import_service(|| async { Ok(vec![]) });

    // 13. Build HTTP router
    let state = AppState {
        db,
        config: config.clone(),
        event_tx,
        auth,
        import,
        metadata: Arc::new(NullMetadata),
        curation: Arc::new(NullCuration),
    };
    let router = paroche::build_router(state);

    // 14. Bind + serve
    let addr = format!("{}:{}", config.paroche.listen_addr, config.paroche.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context(ServerSnafu)?;
    info!("Harmonia serving on {addr}");

    // 15. Graceful shutdown
    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context(ServerSnafu)?;

    // 16. Cleanup — reverse order
    info!("shutting down subsystems");
    feed_scheduler.shutdown();
    scanner.shutdown().await;

    info!("shutdown complete");
    Ok(())
}
