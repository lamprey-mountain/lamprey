use std::{str::FromStr, sync::Arc, time::Duration};

use crate::{
    config,
    serve::{self, serve_transport},
    Result, ServerState,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use common::v1::types::{util::Time, RoomType};
use lamprey_backend_core::{
    config::{Config, ListenComponent},
    types::admin::{AdminCollectGarbage, AdminCollectGarbageMode, AdminCollectGarbageTarget},
    Error,
};
use lamprey_backend_data_postgres::{
    data::Data2, DbRoomCreate, DbUserCreate, RoomCreate, SERVER_ROOM_ID, SERVER_USER_ID,
};
use opendal::layers::LoggingLayer;
use opentelemetry_otlp::WithExportConfig;
use sqlx::postgres::PgPoolOptions;
use tokio::task::JoinSet;
use tracing::{error, info};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

/// the api server
pub struct Server {
    state: Arc<ServerState>,
    router: axum::Router,
}

impl Server {
    /// setup a server
    pub async fn init_from_config(config: Config) -> Result<Self> {
        let state = create_server_state(config).await?;
        let server = Self::init(state).await;
        Ok(server)
    }

    pub async fn init(state: ServerState) -> Self {
        let state = Arc::new(state);
        state.services.start_background_tasks().await;
        let router = serve::create_router(Arc::clone(&state));
        Self { state, router }
    }

    pub fn state(&self) -> Arc<ServerState> {
        Arc::clone(&self.state)
    }

    pub async fn serve(&self) -> Result<()> {
        info!("starting server");

        let mut set = JoinSet::new();

        for config in &self.state.config.listen {
            if config.components.contains(&ListenComponent::Api) {
                let router = self.router.clone();
                let transport = config.transport.clone();
                info!("api listening on {}", transport);
                set.spawn(async move { serve_transport(transport, router).await });
            }
        }

        if set.is_empty() {
            error!("no components enabled for any listeners");
            return Err(Error::BadStatic("no components enabled for any listeners"));
        }

        while let Some(res) = set.join_next().await {
            res.unwrap()?;
        }

        Ok(())
    }
}

async fn create_server_state(config: Config) -> Result<ServerState> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await?;

    let blobs = match &config.blobs {
        config::ConfigBlobs::S3(s3) => {
            let builder = opendal::services::S3::default()
                .bucket(&s3.bucket)
                .endpoint(s3.endpoint.as_str())
                .region(&s3.region)
                .access_key_id(&s3.access_key_id)
                .secret_access_key(s3.secret_access_key.load()?.as_ref());
            opendal::Operator::new(builder)?
                .layer(LoggingLayer::default())
                .finish()
        }
        config::ConfigBlobs::Fs(fs) => {
            let builder = opendal::services::Fs::default().root(fs.data_dir.to_str().unwrap());
            opendal::Operator::new(builder)?
                .layer(LoggingLayer::default())
                .finish()
        }
    };
    blobs.check().await?;

    let nats = if let Some(nats_config) = &config.nats {
        let mut nats_options = async_nats::ConnectOptions::new();
        if let Some(credentials_path) = &nats_config.credentials {
            nats_options = nats_options
                .credentials_file(credentials_path)
                .await
                .map_err(|e| Error::Internal(format!("NATS credentials file failed: {}", e)))?;
        }
        Some(
            async_nats::connect_with_options(&nats_config.addr, nats_options)
                .await
                .map_err(|e| Error::Internal(format!("NATS connect failed: {}", e)))?,
        )
    } else {
        None
    };

    let state = ServerState::init(config, pool, blobs, nats).await;
    state.database.migrate().await?;
    setup_vapid_keys(&state).await?;
    setup_server_room(&state).await?;
    Ok(state)
}

/// create new vapid keys if they dont exist
async fn setup_vapid_keys(state: &ServerState) -> Result<()> {
    let mut txn = state.acquire_data().await?;
    if txn.config_get().await?.is_none() {
        info!("initializing internal config");
        let (keypair, _) = ece::generate_keypair_and_auth_secret()
            .map_err(|e| Error::Internal(format!("VAPID key generation failed: {}", e)))?;
        let vapid_public_key = URL_SAFE_NO_PAD.encode(
            keypair
                .pub_as_raw()
                .map_err(|e| Error::Internal(format!("VAPID key encoding failed: {}", e)))?,
        );
        let vapid_private_key = URL_SAFE_NO_PAD.encode(
            keypair
                .raw_components()
                .map_err(|e| Error::Internal(format!("VAPID key encoding failed: {}", e)))?
                .private_key(),
        );

        let mut jwk = jsonwebkey::JsonWebKey::new(jsonwebkey::Key::generate_p256());
        jwk.set_algorithm(jsonwebkey::Algorithm::ES256).unwrap();
        jwk.key_id = Some(nanoid::nanoid!());
        jwk.key_use = Some(jsonwebkey::KeyUse::Signing);

        txn.config_put(config::ConfigInternal {
            vapid_private_key,
            vapid_public_key,
            oidc_jwk_key: serde_json::to_string(&jwk)?,
            admin_token: None,
            federation_keys: vec![],
        })
        .await?;
    }
    txn.commit().await?;

    Ok(())
}

/// create the server room if it doesnt exist
async fn setup_server_room(state: &ServerState) -> Result<()> {
    let srv = state.services();
    let mut txn = state.acquire_data().await?;
    if txn.user_get(SERVER_USER_ID).await.is_err() {
        txn.user_create(DbUserCreate {
            id: Some(SERVER_USER_ID),
            parent_id: None,
            name: "root".to_string(),
            description: None,
            puppet: None,
            registered_at: Some(Time::now_utc()),
            system: true,
            remote: None,
        })
        .await?;
    }
    if txn.room_get(SERVER_ROOM_ID).await.is_err() {
        srv.rooms
            .create_system(
                RoomCreate {
                    name: "server".to_string(),
                    description: None,
                    icon: None,
                    banner: None,
                    public: Some(false),
                },
                SERVER_USER_ID,
                DbRoomCreate {
                    id: Some(SERVER_ROOM_ID),
                    ty: RoomType::Server,
                    welcome_channel_id: None,
                },
            )
            .await?;
    }
    txn.commit().await?;

    Ok(())
}

pub fn setup_otel(config: &Config) -> Result<()> {
    if let Some(endpoint) = &config.otel_trace_endpoint {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build()?;
        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder()
            .with_batch_exporter(exporter)
            .build();
        use opentelemetry::trace::TracerProvider;
        let tracer = provider.tracer("lamprey-api");
        opentelemetry::global::set_tracer_provider(provider);
        let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        let subscriber = Registry::default()
            .with(EnvFilter::from_str(&config.rust_log)?)
            .with(tracing_subscriber::fmt::layer())
            .with(telemetry_layer);
        tracing::subscriber::set_global_default(subscriber)?;
    } else {
        let subscriber = Registry::default()
            .with(EnvFilter::from_str(&config.rust_log)?)
            .with(tracing_subscriber::fmt::layer());
        tracing::subscriber::set_global_default(subscriber)?;
    }

    Ok(())
}

pub async fn gc(state: Arc<ServerState>, targets: &[AdminCollectGarbageTarget]) -> Result<()> {
    let srv = state.services();

    info!("starting garbage collection");

    let res = srv
        .admin
        .collect_garbage(AdminCollectGarbage {
            targets: targets.to_vec(),
            mode: AdminCollectGarbageMode::Mark,
            async_mode: false,
        })
        .await?;

    for s in res.stats {
        info!(
            "marked {} items of type {:?} for deletion",
            s.rows_deleted, s.target
        );
    }

    info!("deleting...");

    srv.admin
        .collect_garbage(AdminCollectGarbage {
            targets: targets.to_vec(),
            mode: AdminCollectGarbageMode::Sweep,
            async_mode: false,
        })
        .await?;

    info!("done!");

    Ok(())
}
