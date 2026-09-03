use crate::prelude::*;
use common::v1::types::oauth::OidcClaims;
use jsonwebkey::JsonWebKey;
use jsonwebtoken::{Algorithm, EncodingKey, Header};

pub struct ServiceOauthProvider {
    globals: Globals,
}

impl ServiceOauthProvider {
    pub fn new(globals: Globals) -> Self {
        Self { globals }
    }

    pub async fn sign_oidc_claims(&self, claims: &OidcClaims) -> Result<String> {
        let srv = self.globals.services();
        let jwk = srv.config.load_oidc_jwk().await?;
        jwk.sign(claims)
    }
}

pub struct LoadedOidcJwt {
    header: Header,
    encoding_key: EncodingKey,
}

impl LoadedOidcJwt {
    /// parse a serialized json web key
    pub fn parse(s: &str) -> Result<Self> {
        let jwk: JsonWebKey = serde_json::from_str(&s)?;
        let pem = jwk.key.to_pem();

        let (encoding_key, alg) = match jwk.algorithm {
            Some(jsonwebkey::Algorithm::ES256) => {
                (EncodingKey::from_ec_pem(pem.as_bytes())?, Algorithm::ES256)
            }
            Some(jsonwebkey::Algorithm::RS256) => {
                (EncodingKey::from_rsa_pem(pem.as_bytes())?, Algorithm::RS256)
            }

            // TODO: better error
            _ => return Err(Error::Internal("unsupported signing alg".into())),
        };

        let header = Header {
            alg,
            kid: jwk.key_id.clone(),
            ..Default::default()
        };

        Ok(LoadedOidcJwt {
            header,
            encoding_key,
        })
    }

    /// sign a set of claims
    pub fn sign(&self, claims: &OidcClaims) -> Result<String> {
        let token = jsonwebtoken::encode(&self.header, claims, &self.encoding_key)?;
        Ok(token)
    }
}
