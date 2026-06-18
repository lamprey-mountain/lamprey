use std::collections::HashMap;

use serde::Deserialize;
use url::Url;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    pub otel_trace_endpoint: Option<String>,
    pub rust_log: String,

    /// external platforms to connect to
    ///
    /// the key is the name. don't change it once set.
    pub platform: HashMap<String, ConfigPlatform>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ConfigPlatform {
    Lamprey(LampreyConfig),
    Discord(DiscordConfig),
}

#[derive(Debug, Clone, Deserialize)]
pub struct LampreyConfig {
    pub token: Secret,
    pub api_url: Url,
    pub ws_url: Option<Url>,
    pub cdn_url: Option<Url>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscordConfig {
    pub token: Secret,
}

// copied from crate-backend-core/src/config/secret.rs
pub use secret::Secret;
pub mod secret {
    use core::fmt;
    use std::{
        env, fs,
        path::PathBuf,
        sync::{Arc, RwLock},
    };

    use crate::prelude::*;

    use serde::{Deserialize, Serialize};

    /// a secret value that can be read
    ///
    /// deserializes with [`SecretSource`]
    #[derive(Deserialize)]
    #[serde(from = "SecretSource")]
    pub struct Secret {
        source: SecretSource,
        value: RwLock<Option<Arc<str>>>,
    }

    /// the source to load a secret from
    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(untagged)]
    pub enum SecretSource {
        /// a secret that is included directly in the config file. avoid in production.
        Inline(String),

        /// load this secret from a file. trailing newlines are removed.
        File { file_path: PathBuf },

        /// load this secret from an environment variable
        Env { env_var: String },
    }

    impl From<SecretSource> for Secret {
        fn from(source: SecretSource) -> Self {
            Secret::new(source)
        }
    }

    impl Serialize for Secret {
        fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            self.source.serialize(serializer)
        }
    }

    impl fmt::Display for Secret {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match &self.source {
                SecretSource::Inline(_) => write!(f, "<secret inline>"),
                SecretSource::File { file_path } => {
                    write!(f, "<secret file={}>", file_path.display())
                }
                SecretSource::Env { env_var } => write!(f, "<secret env={env_var}>"),
            }
        }
    }

    impl fmt::Debug for Secret {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            // reuse Display so the secret value is never accidentally exposed via {:?}
            fmt::Display::fmt(self, f)
        }
    }

    impl Clone for Secret {
        fn clone(&self) -> Self {
            // PERF: maybe make value an Arc<RwLock<_>> for cloning?
            Self::new(self.source.clone())
        }
    }

    // i don't use tokio since the files are small and will seldom be read
    impl Secret {
        pub fn new(source: SecretSource) -> Self {
            Self {
                source,
                value: RwLock::new(None),
            }
        }

        /// load this secret
        ///
        /// returns cached value if it exists
        pub fn load(&self) -> Result<Arc<str>> {
            {
                let guard = self.value.read().unwrap();
                if let Some(val) = guard.as_ref() {
                    return Ok(val.clone());
                }
            }
            self.reload()
        }

        /// reload this secret
        pub fn reload(&self) -> Result<Arc<str>> {
            let new_value = self.load_uncached()?;
            let mut guard = self.value.write().unwrap();
            let arc_value: Arc<str> = Arc::from(new_value);
            *guard = Some(arc_value.clone());
            Ok(arc_value)
        }

        fn load_uncached(&self) -> Result<String> {
            match &self.source {
                SecretSource::Inline(val) => Ok(val.clone()),
                SecretSource::File { file_path } => {
                    let content = fs::read_to_string(file_path)?;
                    Ok(content.trim().to_string())
                }
                // TODO: more specific error
                SecretSource::Env { env_var } => env::var(env_var)
                    .map_err(|_| anyhow::anyhow!("environment variable {} not set", env_var)),
            }
        }
    }
}
