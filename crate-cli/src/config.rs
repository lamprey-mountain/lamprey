use std::fmt;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use common::v1::types::{ApplicationId, SessionToken};
use serde::{Deserialize, Serialize};
use url::Url;

/// configuration for the `ly` command
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct LyConfig {
    /// where this config was loaded from
    #[serde(skip)]
    path: PathBuf,

    #[serde(default)]
    pub logins: Vec<Login>,

    /// dynamically registered oauth clients, cached to avoid re-registering
    #[serde(default)]
    pub clients: Vec<CachedClient>,
}

/// a dynamically registered oauth client
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CachedClient {
    pub api_url: Url,
    pub client_id: ApplicationId,
    pub client_name: String,
    pub redirect_uris: Vec<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Login {
    pub name: String,
    pub api_url: Url,
    pub token: SessionToken,
    #[serde(default)]
    pub default: bool,

    /// refresh token from an oauth login, if any
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,

    /// oauth client that issued this login, if any
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<ApplicationId>,
}

impl fmt::Debug for Login {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Login")
            .field("name", &self.name)
            .field("api_url", &self.api_url)
            .field("token", &"<redacted>")
            .field("default", &self.default)
            .field(
                "refresh_token",
                &self.refresh_token.as_ref().map(|_| "<redacted>"),
            )
            .field("client_id", &self.client_id)
            .finish()
    }
}

impl Login {
    pub fn new(name: String, api_url: Url, token: SessionToken) -> Self {
        Self {
            name,
            api_url,
            token,
            default: false,
            refresh_token: None,
            client_id: None,
        }
    }
}

impl LyConfig {
    /// the default config file path
    pub fn default_path() -> Result<PathBuf> {
        let config_dir = dirs::config_dir().context("config dir not found")?;
        Ok(config_dir.join("ly").join("config.toml"))
    }

    /// open the config at this path (or the default path), returning an empty
    /// config if the file doesn't exist
    pub fn open(path: Option<PathBuf>) -> Result<Self> {
        let path = match path {
            Some(path) => path,
            None => Self::default_path()?,
        };

        let mut config: Self = match fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text)
                .with_context(|| format!("failed to parse config at {}", path.display()))?,
            Err(err) if err.kind() == ErrorKind::NotFound => Self::default(),
            Err(err) => {
                return Err(err)
                    .with_context(|| format!("failed to read config at {}", path.display()));
            }
        };
        config.path = path;

        Ok(config)
    }

    /// atomically save the config, creating the parent directory if needed
    pub fn save(&self) -> Result<()> {
        let parent = self.path.parent().context("config path has no parent")?;
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir {}", parent.display()))?;

        // the config holds session tokens, so write it to a private temp file first
        let tmp_path = self.path.with_extension("toml.new");
        let toml = toml::to_string_pretty(self).context("failed to serialize config")?;
        write_private(&tmp_path, &toml)?;

        fs::rename(&tmp_path, &self.path)
            .with_context(|| format!("failed to save config at {}", self.path.display()))?;
        Ok(())
    }

    /// the path this config was loaded from
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// get a login by name, or the default login if no name is given
    pub fn login(&self, name: Option<&str>) -> Result<&Login> {
        Ok(&self.logins[self.position(name)?])
    }

    /// set the default login by name
    pub fn set_default(&mut self, name: &str) -> Result<()> {
        let index = self.position(Some(name))?;
        for (position, login) in self.logins.iter_mut().enumerate() {
            login.default = position == index;
        }

        Ok(())
    }

    /// add a login, replacing an existing login with the same name
    ///
    /// the config always has a default login if it has any logins at all
    pub fn add(&mut self, mut login: Login) {
        if let Some(existing) = self.logins.iter_mut().find(|l| l.name == login.name) {
            login.default = existing.default;
            *existing = login;
            return;
        }

        if !self.logins.iter().any(|l| l.default) {
            login.default = true;
        }

        self.logins.push(login);
    }

    /// get the cached dynamic client for this api url
    pub fn client(&self, api_url: &Url) -> Option<&CachedClient> {
        self.clients
            .iter()
            .find(|client| &client.api_url == api_url)
    }

    /// add or replace the cached dynamic client for this api url
    pub fn set_client(&mut self, client: CachedClient) {
        match self
            .clients
            .iter_mut()
            .find(|existing| existing.api_url == client.api_url)
        {
            Some(existing) => *existing = client,
            None => self.clients.push(client),
        }
    }

    /// remove a login by name (or the default login), promoting the first
    /// remaining login to default
    pub fn remove(&mut self, name: Option<&str>) -> Result<Login> {
        let index = self.position(name)?;
        let removed = self.logins.remove(index);
        if removed.default
            && let Some(first) = self.logins.first_mut()
        {
            first.default = true;
        }

        Ok(removed)
    }

    fn position(&self, name: Option<&str>) -> Result<usize> {
        match name {
            Some(name) => self
                .logins
                .iter()
                .position(|login| login.name == name)
                .with_context(|| format!("no login named {name}")),
            None => self
                .logins
                .iter()
                .position(|login| login.default)
                .context("no default login"),
        }
    }
}

fn write_private(path: &Path, contents: &str) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options
        .open(path)
        .with_context(|| format!("failed to write config at {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .with_context(|| format!("failed to set permissions on {}", path.display()))?;
    }

    file.write_all(contents.as_bytes())
        .with_context(|| format!("failed to write config at {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to write config at {}", path.display()))?;

    Ok(())
}
