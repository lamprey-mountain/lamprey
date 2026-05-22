use core::fmt;
use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
};

use http::HeaderValue;
use ipnet::IpNet;
use serde::{Deserialize, Serialize};
use strum::{EnumIter, IntoEnumIterator};
use tracing::error;
use url::Url;

use crate::{Error, Result};

use common::v1::types::federation::ServerKeyAlgorithm;
use common::v1::types::redex::EvalLimits;
use common::v1::types::util::Time;

/// a server's signing key for internal use (includes private key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerKeyInternal {
    /// the key algorithm
    pub alg: ServerKeyAlgorithm,

    /// public key (base64 url safe unpadded)
    pub pubkey: String,

    /// private key (base64 url safe unpadded)
    pub privkey: String,

    /// when this key expires
    pub expires_at: Time,
}

// TODO: derive(Clone, Serialize)
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

    /// public hostname for federation
    pub hostname: Option<String>,

    /// for media/file uploads
    #[serde(alias = "s3")]
    pub blobs: ConfigBlobs,

    pub oauth_provider: HashMap<String, ConfigOauthProvider>,

    pub otel_trace_endpoint: Option<String>,

    // TODO: make optional
    #[serde(default)]
    pub http: ConfigHttp,

    // TODO: make optional
    pub smtp: ConfigSmtp,

    #[serde(default)]
    pub url_preview: ConfigUrlPreview,

    #[serde(default = "default_max_user_emails")]
    pub max_user_emails: usize,

    #[serde(default = "default_email_queue_workers")]
    pub email_queue_workers: usize,

    #[serde(default = "default_require_server_invite")]
    pub require_server_invite: bool,

    #[serde(default = "default_listen")]
    pub listen: Vec<ListenConfig>,

    /// whether to enable admin tokens
    ///
    /// this stores a token in the database that allows full access to the
    /// server. this must be enabled to use the cli interface.
    #[serde(default = "default_true")]
    pub enable_admin_token: bool,

    /// configuration for nats
    ///
    /// if None, use in memory channels for events
    pub nats: Option<ConfigNats>,

    /// static admin token override
    pub admin_token: Option<String>,

    #[serde(default)]
    pub media: ConfigMedia,

    /// voice config, if None disables voice
    pub voice: Option<ConfigVoice>,

    #[serde(default)]
    pub scripts: ConfigScripts,

    /// experimental features to enable
    #[serde(default)]
    pub experiments: ConfigExperiments,
}

fn default_require_server_invite() -> bool {
    true
}

fn default_true() -> bool {
    true
}

fn default_max_user_emails() -> usize {
    50
}

fn default_email_queue_workers() -> usize {
    5
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ConfigBlobs {
    S3(ConfigS3),
    Fs(ConfigFs),
}

#[derive(Debug, Deserialize)]
pub struct ConfigFs {
    pub data_dir: PathBuf,
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

#[derive(Debug, Default, Deserialize)]
pub struct ConfigUrlPreview {
    // does this need anything?
}

#[derive(Debug, Deserialize)]
pub struct ConfigHttp {
    /// contact information for webmasters; gner
    pub contact: Option<String>,

    /// override the user agent string
    pub user_agent: Option<String>,

    /// deny access to these ip addresses

    #[serde(default = "default_deny_list")]
    pub deny: Vec<IpNet>,

    /// the maximum number of parallel requests
    #[serde(default = "default_max_parallel_jobs")]
    pub max_parallel_jobs: usize,
}

impl Default for ConfigHttp {
    fn default() -> Self {
        Self {
            contact: None,
            user_agent: None,
            deny: default_deny_list(),
            max_parallel_jobs: default_max_parallel_jobs(),
        }
    }
}

fn default_deny_list() -> Vec<IpNet> {
    vec![
        "127.0.0.1/8"
            .parse()
            .expect("Invalid default IPv4 loopback"),
        "10.0.0.0/8".parse().expect("Invalid default RFC1918 range"),
        "172.16.0.0/12"
            .parse()
            .expect("Invalid default RFC1918 range"),
        "192.168.0.0/16"
            .parse()
            .expect("Invalid default RFC1918 range"),
        "100.64.0.0/10"
            .parse()
            .expect("Invalid default CGNAT range"),
        "169.254.0.0/16"
            .parse()
            .expect("Invalid default link-local range"),
        "::1/128".parse().expect("Invalid default IPv6 loopback"),
        "fe80::/64"
            .parse()
            .expect("Invalid default IPv6 link-local"),
        "fc00::/7".parse().expect("Invalid default IPv6 ULA range"),
    ]
}

fn default_max_parallel_jobs() -> usize {
    8
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigSmtp {
    pub username: String,
    pub password: String,
    pub host: String,
    pub from: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigScripts {
    /// whether to enable the script system
    pub enabled: bool,

    /// the domain suffix for http handlers
    ///
    /// setting to `example.com` will cause `random-uuid-here.example.com` domains to be handed to http scripts
    pub suffix: Option<String>,

    /// default limits for scripts
    #[serde(default = "EvalLimits::strict")]
    pub limits: EvalLimits,
}

/// config for the media server
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigMedia {
    #[serde(default = "default_cache_media")]
    pub cache_media: u64,

    #[serde(default = "default_cache_emoji")]
    pub cache_emoji: u64,

    #[serde(default = "default_thumb_sizes")]
    pub thumb_sizes: Vec<u32>,

    /// the maximum size of media in bytes (default 8MiB)
    #[serde(default = "default_max_media_size")]
    pub max_size: u64,

    /// media scanners
    #[serde(default)]
    pub scanners: Vec<ConfigMediaScanner>,
}

fn default_cache_media() -> u64 {
    10_000
}

fn default_cache_emoji() -> u64 {
    1_000_000
}

fn default_thumb_sizes() -> Vec<u32> {
    vec![64, 320, 640]
}

fn default_max_media_size() -> u64 {
    8 * 1024 * 1024 // 8 MiB
}

impl Default for ConfigMedia {
    fn default() -> Self {
        ConfigMedia {
            cache_media: default_cache_media(),
            cache_emoji: default_cache_emoji(),
            thumb_sizes: default_thumb_sizes(),
            max_size: default_max_media_size(),
            scanners: Vec::new(),
        }
    }
}

/// config for the voice server
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigVoice {
    /// the token for the voice servers to connect via
    pub token: String,

    /// override the ipv4 address to listen on
    pub host_ipv4: Option<String>,

    /// override the ipv6 address to listen on
    pub host_ipv6: Option<String>,

    /// the number of worker threads to spawn
    ///
    /// defaults to the number of cpu cores
    pub workers: Option<u8>,

    /// the udp port to use for media traffic
    ///
    /// defaults to a random port
    pub udp_port: u16,
}

impl Default for ConfigScripts {
    fn default() -> Self {
        Self {
            enabled: Default::default(),
            suffix: Default::default(),
            limits: EvalLimits::strict(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ConfigNats {
    /// the address of the nats server
    #[serde(default = "default_nats_addr")]
    pub addr: String,

    /// path to a nats credential file, if authentication is required
    pub credentials: Option<PathBuf>,
}

fn default_nats_addr() -> String {
    "localhost:4222".to_string()
}

#[derive(Debug, Default, Deserialize)]
pub struct ConfigExperiments {}

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
    /// the main rest api server, websocket sync, and
    Api,

    // TODO: merge media serving here
    // /// the media proxy server
    // Media,
    /// metrics for this service
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

/// Configuration for an external media scanning service.
///
/// Media scanners are external services that analyze uploaded media files
/// and return confidence scores for various categories (e.g., NSFW content, malware).
#[derive(Debug, Clone, Deserialize)]
pub struct ConfigMediaScanner {
    /// The URL to POST scan requests to.
    pub scan_url: Url,

    /// The URL to GET for health checks.
    pub health_url: Url,

    /// The unique name of the media scanner (e.g., `nsfw`, `malware`).
    pub key: String,

    /// The current version of this scanner.
    ///
    /// This version is stored alongside scan results to track which scanner
    /// version was used for each scan.
    pub version: u16,
}

/// internal config that is saved in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigInternal {
    /// web push api vapid public key
    pub vapid_private_key: String,

    /// web push api vapid private key
    pub vapid_public_key: String,

    /// openid connect key
    pub oidc_jwk_key: String,

    /// a token that can be used to do administrative operations on this server
    ///
    /// - DO NOT LEAK THIS TOKEN!
    /// - if this is None, there is no valid token
    /// - this gets rotated every 5 minutes
    /// - cli tools will fetch this token from the db, then do admin tasks through the http api
    pub admin_token: Option<String>,

    /// federation signing keys
    #[serde(default)]
    pub federation_keys: Vec<ServerKeyInternal>,
}

// TODO: use this for loading secrets
// do i use this type in config structs, or just use this to decode?
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
                error!("environment variable {env_var} not set");
                Error::Internal(format!("environment variable {env_var} not set"))
            }),
        }
    }
}

const VERSION: &str = env!("CARGO_PKG_VERSION");

impl Config {
    pub fn hostname(&self) -> Result<&str> {
        self.hostname
            .as_deref()
            .ok_or_else(|| Error::Internal("federation hostname not configured".to_owned()))
    }

    /// get user agent string
    pub fn user_agent(&self) -> String {
        if let Some(ua) = &self.http.user_agent {
            return ua.to_string();
        }

        let host = self.hostname.as_deref().unwrap_or("secluded");
        let contact = self.http.contact.as_deref().unwrap_or("anonymous");

        format!("Lamprey/v{VERSION} ({contact}; {host}")
    }

    /// get user agent string
    pub fn user_agent_header_value(&self) -> Result<HeaderValue> {
        Ok(HeaderValue::from_str(&self.user_agent())?)
    }
}
