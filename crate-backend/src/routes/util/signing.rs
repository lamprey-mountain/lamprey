use common::v1::types::{federation::Hostname, util::Time};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use http::{HeaderMap, HeaderName, HeaderValue};
use sha2::{Digest, Sha512_256};

use crate::{error::Result, services::federation::LocalSigningKey, Error};

/// how long a signed request is valid for
pub const SIGNATURE_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(30);

/// header names for federation signing
pub const HEADER_ORIGIN: &str = "x-origin";
pub const HEADER_TIMESTAMP: &str = "x-timestamp";
pub const HEADER_SIGNATURE: &str = "x-signature";
pub const HEADER_PUBKEY: &str = "x-pubkey";
pub const HEADER_HOST: &str = "x-host";

/// base64 engine used for encoding/decoding signature headers
const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// headers attached to a signed federation request
#[derive(Debug, Clone)]
pub struct SigningHeaders {
    /// the server this request *says* it came from
    pub origin: Hostname,
    /// the host this request was sent to
    pub host: String,
    /// unix timestamp as a decimal string
    pub timestamp: String,
    /// raw ed25519 signature bytes
    pub signature: Vec<u8>,
    /// raw ed25519 public key bytes
    pub pubkey: Vec<u8>,
}

impl SigningHeaders {
    /// parse signing headers from an incoming HTTP request
    pub fn decode(headers: &HeaderMap) -> Result<Self> {
        let origin = headers
            .get(HEADER_ORIGIN)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::BadHeader)?
            .to_string();

        let host = headers
            .get(HEADER_HOST)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::BadHeader)?
            .to_string();

        let timestamp = headers
            .get(HEADER_TIMESTAMP)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::BadHeader)?
            .to_string();

        let signature = headers
            .get(HEADER_SIGNATURE)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::BadHeader)?;

        let signature = base64::Engine::decode(&B64, signature)
            .map_err(|_| Error::BadStatic("invalid signature encoding"))?;

        let pubkey = headers
            .get(HEADER_PUBKEY)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| Error::BadHeader)?;

        let pubkey = base64::Engine::decode(&B64, pubkey)
            .map_err(|_| Error::BadStatic("invalid pubkey encoding"))?;

        Ok(Self {
            origin: Hostname(origin),
            host,
            timestamp,
            signature,
            pubkey,
        })
    }

    /// write signing headers into an outgoing HTTP request
    pub fn encode(&self, headers: &mut HeaderMap) {
        headers.insert(
            HeaderName::from_static(HEADER_ORIGIN),
            HeaderValue::from_str(&self.origin.0).unwrap(),
        );
        headers.insert(
            HeaderName::from_static(HEADER_HOST),
            HeaderValue::from_str(&self.host).unwrap(),
        );
        headers.insert(
            HeaderName::from_static(HEADER_TIMESTAMP),
            HeaderValue::from_str(&self.timestamp).unwrap(),
        );
        headers.insert(
            HeaderName::from_static(HEADER_SIGNATURE),
            HeaderValue::from_str(&base64::Engine::encode(&B64, &self.signature)).unwrap(),
        );
        headers.insert(
            HeaderName::from_static(HEADER_PUBKEY),
            HeaderValue::from_str(&base64::Engine::encode(&B64, &self.pubkey)).unwrap(),
        );
    }
}

/// compute the canonical hash input for a federation request
///
/// format: `method\npath\norigin\nhost\ntimestamp\nbody`
pub fn compute_hash(
    method: &str,
    path: &str,
    origin: &str,
    host: &str,
    timestamp: &str,
    body: &[u8],
) -> Vec<u8> {
    let mut hasher = Sha512_256::new();
    hasher.update(method.as_bytes());
    hasher.update(b"\n");
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    hasher.update(origin.as_bytes());
    hasher.update(b"\n");
    hasher.update(host.as_bytes());
    hasher.update(b"\n");
    hasher.update(timestamp.as_bytes());
    hasher.update(b"\n");
    hasher.update(body);
    hasher.finalize().to_vec()
}

/// an outgoing request that needs to be signed
pub struct OutgoingRequest<'a> {
    pub origin: Hostname,
    pub host: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub body: &'a [u8],
}

impl OutgoingRequest<'_> {
    /// sign this request with the given local key, producing headers to attach
    pub fn sign(&self, key: &LocalSigningKey) -> Result<SigningHeaders> {
        let timestamp = Time::now_utc().to_string();

        let hash = compute_hash(
            self.method,
            self.path,
            &self.origin.0,
            self.host,
            &timestamp,
            self.body,
        );

        let sig: ed25519_dalek::Signature = key.signing_key.sign(&hash);

        Ok(SigningHeaders {
            origin: self.origin.clone(),
            host: self.host.to_string(),
            timestamp,
            signature: sig.to_bytes().to_vec(),
            pubkey: key.pubkey.to_bytes().to_vec(),
        })
    }
}

/// an incoming request that needs to be verified
pub struct IncomingRequest<'a> {
    pub origin: Hostname,
    pub host: &'a str,
    pub method: &'a str,
    pub path: &'a str,
    pub body: &'a [u8],
    pub headers: &'a SigningHeaders,
}

impl IncomingRequest<'_> {
    /// verify the ed25519 signature
    pub fn verify_signature(&self, verifying_key: &VerifyingKey) -> Result<()> {
        let sig = Signature::from_slice(&self.headers.signature)
            .map_err(|_| Error::BadStatic("invalid signature encoding"))?;

        let hash = compute_hash(
            self.method,
            self.path,
            &self.origin.0,
            self.host,
            &self.headers.timestamp,
            self.body,
        );

        verifying_key
            .verify(&hash, &sig)
            .map_err(|_| Error::BadStatic("signature verification failed"))
    }

    /// verify the signature and check the timestamp isn't expired
    pub fn verify(&self, verifying_key: &VerifyingKey) -> Result<()> {
        let ts: i64 = self
            .headers
            .timestamp
            .parse()
            .map_err(|_| Error::BadStatic("invalid timestamp"))?;

        let req_time = time::OffsetDateTime::from_unix_timestamp(ts)
            .map_err(|_| Error::BadStatic("timestamp out of range"))?;

        let now = time::OffsetDateTime::now_utc();
        let diff = if now >= req_time {
            now - req_time
        } else {
            req_time - now
        };

        let max_age = time::Duration::seconds(SIGNATURE_MAX_AGE.as_secs() as i64);
        if diff > max_age {
            return Err(Error::BadStatic("request expired"));
        }

        self.verify_signature(verifying_key)
    }
}
