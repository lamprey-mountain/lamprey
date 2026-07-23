use crate::error::{Error, Result};
use crate::services::federation::signing::{ValidatedKey, ValidatedKeyAlgo};
use crate::services::federation::{ServerInfo, ServiceFederation};
use common::v1::types::federation::signing::OutgoingRequest;
use common::v1::types::federation::{Hostname, ServerKeys, ServerPingResponse, WellKnown};
use common::v1::types::util::Time;
use ed25519_dalek::VerifyingKey;
use url::Url;

// TODO: deduplicate code (eg. for signing, for http requests)
impl ServiceFederation {
    /// ping a remote server
    pub async fn ping(&self, hostname: Hostname) -> Result<bool> {
        let info = self.fetch_server_info(&hostname).await?;
        let ping_url = info
            .api_url
            .join(&format!("/api/v1/server/{}/ping", hostname.0))?;

        let key = self
            .get_local_keys()
            .await
            .into_iter()
            .next()
            .ok_or_else(|| Error::BadStatic("no local signing keys"))?;

        let req = OutgoingRequest {
            origin: &self.state.config().hostname2()?,
            host: &hostname,
            method: "POST",
            path: ping_url.path(),
            body: &[],
        };

        let res = self
            .state
            .services()
            .http
            .client
            .post(ping_url.clone())
            .headers(req.sign(&key)?)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(Error::BadStatic("ping failed"));
        }

        let body: ServerPingResponse = res.json().await?;
        Ok(body.federated)
    }

    /// connect to a remote server to start syncing
    pub async fn connect(&self, hostname: Hostname) -> Result<()> {
        let info = self.fetch_server_info(&hostname).await?;
        let connect_url = info
            .api_url
            .join(&format!("/api/v1/server/{}/connect", hostname.0))?;

        let key = self
            .get_local_keys()
            .await
            .into_iter()
            .next()
            .ok_or_else(|| Error::BadStatic("no local signing keys"))?;

        let req = OutgoingRequest {
            origin: &self.state.config().hostname2()?,
            host: &hostname,
            method: "POST",
            path: connect_url.path(),
            body: &[],
        };

        let res = self
            .state
            .services()
            .http
            .client
            .post(connect_url.clone())
            .headers(req.sign(&key)?)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(Error::BadStatic("connection request failed"));
        }

        Ok(())
    }

    /// lookup the server info for this hostname
    pub async fn fetch_server_info(&self, hostname: &Hostname) -> Result<ServerInfo> {
        if let Some(info) = self.cache.get(hostname).await {
            return Ok(info);
        }

        let well_known_url = Url::parse(&format!(
            "https://{}/.well-known/lamprey-mountain",
            hostname.0
        ))?;

        let res = self
            .state
            .services()
            .http
            .client
            .get(well_known_url)
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Error::BadStatic("failed to fetch well-known"));
        }

        let well_known: WellKnown = res.json().await?;

        // TODO: use strongly typed request structs like `common::v1::routes::federation::server_keys_get::Request` instead of manually building urls
        let keys_url = well_known
            .api_url
            .join(&format!("/api/v1/server/{}/keys", &hostname.0))?;

        let res = self
            .state
            .services()
            .http
            .client
            .get(keys_url)
            .send()
            .await?;
        if !res.status().is_success() {
            return Err(Error::BadStatic("failed to fetch server keys"));
        }

        let server_keys: ServerKeys = res.json().await?;

        let now = Time::now_utc();
        let validated: Vec<ValidatedKey> = server_keys
            .keys
            .into_iter()
            .filter(|k| k.expires_at > now)
            .map(|k| {
                let pubkey_bytes: [u8; 32] = k
                    .pubkey
                    .as_ref()
                    .try_into()
                    .map_err(|_| Error::BadStatic("invalid pubkey length"))?;

                let verifying_key = VerifyingKey::from_bytes(&pubkey_bytes)
                    .map_err(|_| Error::BadStatic("invalid public key"))?;

                Ok(ValidatedKey {
                    alg: ValidatedKeyAlgo::Ed25519(verifying_key),
                    expires_at: k.expires_at,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let info = ServerInfo {
            api_url: well_known.api_url,
            cdn_url: well_known.cdn_url,
            keys: validated,
        };

        self.cache.insert(hostname.to_owned(), info.clone()).await;
        Ok(info)
    }

    /// fetch the signing keys for this hostname
    pub async fn fetch_keys(&self, hostname: &Hostname) -> Result<Vec<ValidatedKey>> {
        let info = self.fetch_server_info(hostname).await?;
        Ok(info.keys)
    }
}
