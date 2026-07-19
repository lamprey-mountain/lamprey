//! global server state

use crate::globals::messaging::{Messaging, Transport};
use crate::prelude::*;

use std::sync::Weak;
use std::time::Duration;

use lamprey_backend_core::config::{Config, ConfigBlobs};
use lamprey_backend_data_postgres::data::postgres::PostgresPool;
use lamprey_backend_data_postgres::data::{AnyData, Database};
use opendal::layers::LoggingLayer;
use sqlx::postgres::PgPoolOptions;
use tokio::runtime::Handle as TokioHandle;
use tracing::info;

pub mod messaging;

/// owned handle for the server's global state
#[derive(Clone)]
pub struct GlobalsOwned {
    inner: Arc<GlobalsInner>,
    services: Arc<Services>,
}

/// global state for the server
#[derive(Clone)]
pub struct Globals {
    inner: Arc<GlobalsInner>,
    services: Weak<Services>,
}

struct GlobalsInner {
    /// config for this server
    config: Box<Config>,

    /// reference to the database for persistent data
    database: Box<dyn Database>,

    // TEMP: compat
    database_compat: Box<PostgresPool>,

    /// storage for large blobs
    blobs: opendal::Operator,

    /// send and receive messages
    messaging: Messaging,
}

impl GlobalsOwned {
    /// get a handle to the `Globals` itself
    pub fn handle(&self) -> Globals {
        Globals {
            inner: Arc::clone(&self.inner),
            services: Arc::downgrade(&self.services),
        }
    }
}

impl Globals {
    pub async fn init_from_config(config: Config) -> Result<GlobalsOwned> {
        // lint config
        for issue in config.lint() {
            issue.log();
        }

        // setup the database connection
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect_lazy(&config.database_url)?;
        let database = Box::new(PostgresPool::new(pool));

        // setup the object storage connection
        let blobs = match &config.blobs {
            ConfigBlobs::S3(s3) => {
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
            ConfigBlobs::Fs(fs) => {
                let builder = opendal::services::Fs::default().root(fs.data_dir.to_str().unwrap());
                opendal::Operator::new(builder)?
                    .layer(LoggingLayer::default())
                    .finish()
            }
        };

        // set up messaging
        let transport = if let Some(nats_config) = &config.nats {
            info!("using NATS for messaging");
            let mut nats_options = async_nats::ConnectOptions::new();
            if let Some(credentials_path) = &nats_config.credentials {
                nats_options = nats_options
                    .credentials_file(credentials_path)
                    .await
                    .map_err(|e| Error::Internal(format!("NATS credentials file failed: {}", e)))?;
            }
            let nats = async_nats::connect_with_options(&nats_config.addr, nats_options)
                .await
                .map_err(|e| Error::Internal(format!("NATS connect failed: {}", e)))?;
            Transport::nats(nats)
        } else {
            info!("using in-memory messaging");
            Transport::memory()
        };
        let messaging = Messaging::new(transport);

        let inner = Arc::new(GlobalsInner {
            config: Box::new(config),
            database: database.clone(),
            database_compat: database,
            blobs,
            messaging,
        });

        // create services and tie up the arc cycle
        let srv = Arc::new_cyclic(|weak_services| {
            let globals = Globals {
                inner: Arc::clone(&inner),
                services: weak_services.clone(),
            };
            Services::new(globals)
        });

        // initialize server
        inner.database.migrate().await?;
        inner.blobs.check().await?; // TODO: remove
        srv.start_background_tasks().await;

        // TODO: add these
        // srv.notifications.init_vapid_keys().await?; -> setup_vapid_keys(&state).await?;
        // srv.rooms.init_server_rooms().await?;-> setup_server_room(&state).await?;

        Ok(GlobalsOwned {
            inner,
            services: srv,
        })
    }

    // TEMP: compat
    pub fn temp_database_compat(&self) -> Box<PostgresPool> {
        self.inner.database_compat.clone()
    }

    // TEMP: compat
    pub fn temp_services_raw(&self) -> Weak<Services> {
        self.services.clone()
    }

    // TEMP: maybe i'll have a reference to the Database instead
    pub async fn temp_test_database(&self) -> bool {
        self.inner
            .database
            .check_database()
            .await
            .unwrap_or_default()
    }

    // TODO: use this?
    // TODO: what happens if this is called inside another with_data?
    // let a = self.state.with_data(|txn| async move {txn.reaction_put(user_id, channel_id, message_id, key).await?; Ok(123)}).await?;
    pub async fn with_data<T, F, Fut>(&self, f: F) -> Result<T>
    where
        F: FnOnce(&mut AnyData) -> Fut,
        Fut: Future<Output = Result<T>>,
    {
        let mut txn = self.inner.database.begin().await?;
        match f(&mut txn).await {
            Ok(res) => {
                txn.commit().await?;
                Ok(res)
            }
            Err(err) => {
                // NOTE: is this redundant?
                let _ = txn.rollback().await;
                Err(err)
            }
        }
    }

    /// begin a database transaction
    ///
    /// use this for writes and for reads that need consistency
    pub async fn begin(&self) -> Result<AnyData> {
        self.inner.database.begin().await
    }

    /// begin a database session without a transaction
    ///
    /// use this for isolated single reads
    pub async fn begin_read(&self) -> Result<AnyData> {
        self.inner.database.begin_read().await
    }

    pub fn services(&self) -> Arc<Services> {
        self.services.upgrade().expect("Services should exist")
    }

    pub fn messaging(&self) -> &Messaging {
        &self.inner.messaging
    }

    pub fn config(&self) -> &Config {
        &self.inner.config
    }

    // TEMP: i should write a wrapper for opendal
    pub fn blobs(&self) -> &opendal::Operator {
        &self.inner.blobs
    }
}
