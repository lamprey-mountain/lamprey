use core::fmt;
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};

use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};
use url::Url;

use crate::Result;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub rust_log: String,
    pub database_url: String,

    /// public api url
    pub api_url: Url,

    /// public url where media is served from
    pub cdn_url: Url,

    /// public url for the web ui
    pub html_url: Url,

    /// for media/file uploads
    pub s3: ConfigS3,
    pub oauth_provider: HashMap<String, ConfigOauthProvider>,
    pub url_preview: ConfigUrlPreview,
    pub http: ConfigHttp,
    pub media_max_size: u64,
    pub smtp: ConfigSmtp,
    pub otel_trace_endpoint: Option<String>,
    pub sfu_token: String,

    #[serde(default = "default_max_user_emails")]
    pub max_user_emails: usize,

    #[serde(default = "default_email_queue_workers")]
    pub email_queue_workers: usize,

    #[serde(default = "default_require_server_invite")]
    pub require_server_invite: bool,

    #[serde(default = "default_listen")]
    pub listen: Vec<ListenConfig>,

    #[serde(default)]
    pub media_scanners: Vec<ConfigMediaScanner>,
}

fn default_require_server_invite() -> bool {
    true
}

fn default_max_user_emails() -> usize {
    50
}

fn default_email_queue_workers() -> usize {
    5
}

#[derive(Debug, Deserialize)]
pub struct ConfigS3 {
    pub bucket: String,
    pub endpoint: Url,
    pub region: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    // /// alternative host instead of going though the reverse proxy
    // pub local_endpoint: Option<Url>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigOauthProvider {
    pub client_id: String,
    pub client_secret: String,
    pub authorization_url: String,
    pub token_url: String,
    pub revocation_url: String,

    /// automatically mark users as registered if they create an account or link their account with this provider
    #[serde(default)]
    pub autoregister: bool,
}

#[derive(Debug, Deserialize)]
pub struct ConfigUrlPreview {
    // does this need anything?
}

#[derive(Debug, Deserialize)]
pub struct ConfigHttp {
    pub user_agent: String,
    pub deny: Vec<IpNet>,
    pub max_parallel_jobs: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigSmtp {
    pub username: String,
    pub password: String,
    pub host: String,
    pub from: String,
}

#[derive(Clone, Debug, Deserialize)]
// Incompatible with deny_unknown_fields due to serde(flatten).
pub struct ListenConfig {
    #[serde(default = "ListenComponent::all_components")]
    pub components: HashSet<ListenComponent>,
    #[serde(flatten)]
    pub transport: ListenTransport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize, EnumIter, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum ListenComponent {
    Api,
    Metrics,
}

impl ListenComponent {
    fn all_components() -> HashSet<Self> {
        Self::iter().collect()
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
#[serde(deny_unknown_fields)]
pub enum ListenTransport {
    Tcp {
        #[serde(default = "default_address")]
        address: IpAddr,
        #[serde(default = "default_port")]
        port: u16,
    },
    Unix {
        path: PathBuf,
    },
}

impl Display for ListenTransport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ListenTransport::Tcp { address, port } => {
                write!(f, "http://{address}:{port}")
            }
            ListenTransport::Unix { path } => {
                write!(f, "http+unix://{}", path.display())
            }
        }
    }
}

fn default_listen() -> Vec<ListenConfig> {
    vec![ListenConfig {
        components: ListenComponent::all_components(),
        transport: ListenTransport::Tcp {
            address: default_address(),
            port: default_port(),
        },
    }]
}

fn default_address() -> IpAddr {
    Ipv4Addr::LOCALHOST.into()
}

fn default_port() -> u16 {
    4000
}

/// A method to scan media
#[derive(Debug, Deserialize)]
pub struct ConfigMediaScanner {
    /// The url to post media to
    pub scan_url: Url,

    /// The unique name of the media scanner (eg. `nsfw`, `malware`)
    pub key: String,

    /// The current version of this scanner.
    pub version: u16,
}

/// internal config that is saved in the database
#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigInternal {
    pub vapid_key: String,
    pub oidc_jwk_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Secret {
    /// a secret that is included directly in the config file. avoid in production.
    Inline(String),

    /// load this secret from a file. trailing newlines are removed.
    File { file_path: PathBuf },

    /// load this secret from an environment variable
    Env { env_var: String },
}

impl Secret {
    /// load this secret
    pub fn load(&self) -> Result<String> {
        match self {
            Secret::Inline(s) => Ok(s.to_owned()),
            Secret::File { file_path } => {
                let s = std::fs::read_to_string(file_path)?;
                Ok(s.trim_end().to_owned())
            }
            Secret::Env { env_var } => std::env::var(env_var).map_err(|_| {
                crate::Error::BadRequest(format!("environment variable {env_var} not set"))
            }),
        }
    }
}

/*
media scanning notes

```
POST /media/{media_id}/rescan
```

```
POST scan_url
Content-Length: blob_length

<blob bytes>
```

```
200 OK
Content-Type: application/json

{"score":0.5}
```
*/

// #[derive(Debug, Clone, Serialize, Deserialize)]
// pub struct MediaScanResponse {
//     pub score: f32,

//     /// note for the sysadmin
//     pub note: Option<String>,
// }
