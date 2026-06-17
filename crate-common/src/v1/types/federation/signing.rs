//! request signing types and logic

use crate::v1::types::{
    federation::{
        Hostname,
        consts::{KEY_EXPIRY, SIGNATURE_MAX_AGE},
    },
    headers::{HEADER_ORIGIN, HEADER_PUBKEY, HEADER_SIGNATURE, HEADER_TIMESTAMP},
    misc::{Time, binary::Binary},
};

use bytes::Bytes;
use ed25519_dalek::{Signature, SigningKey, VerifyingKey, ed25519::signature::Signer};
use http::{HeaderMap, HeaderValue};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use thiserror::Error;
#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

/// base64 engine used for encoding/decoding signature headers
const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// a server's signing key
// NOTE: a lot of binary MAX_SIZEs are larger than needed, maybe i should shrink it
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ServerKey {
    /// the key algorithm
    pub alg: ServerKeyAlgorithm,

    /// public key
    pub pubkey: Binary<128>,

    /// random data to sign
    pub nonce: Binary<128>,

    /// the signature
    ///
    /// the bytes that were signed: nonce || pubkey || hostname
    pub signature: Binary<128>,

    /// when this key expires
    ///
    /// maximum Date + 72h, should be Date + 48h and rotated every 24h
    // NOTE: should i require more frequent rotation?
    pub expires_at: Time,
}

/// a server's key signing key with its public key pre-parsed
#[derive(Debug, Clone)]
pub struct ServerKeySecret {
    pub pubkey: VerifyingKey,
    pub signing_key: SigningKey,
    pub expires_at: Time,
}

/// the algorithm to sign requests with
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(rename_all = "lowercase")
)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ServerKeyAlgorithm {
    Ed25519,
}

/// an error that occured during the signing process
#[derive(Debug, Clone, Error)]
pub enum SigningError {
    /// header is missing or unable to be deserialized
    #[error("invalid header")]
    InvalidHeader,

    /// failed to encode data into base64
    #[error("encoding failed")]
    Encoding,

    /// header(s) contains newlines
    #[error("headers contain newlines")]
    ContainsNewlines,

    /// the timestamp is invalid
    #[error("invalid timestamp")]
    InvalidTimestamp,

    /// the request is expired
    #[error("request expired")]
    RequestExpired,

    /// the signature is invalid
    #[error("invalid signature")]
    InvalidSignature,
}

/// utility to extract/encode headers for a signed federation request
#[derive(Debug, Clone)]
struct SigningHeaders {
    /// the server this request *says* it came from
    origin: Hostname,

    /// the host this request was sent to
    host: Hostname,

    /// unix timestamp as a decimal string
    timestamp: String,

    /// raw ed25519 signature bytes
    signature: Vec<u8>,

    /// raw ed25519 public key bytes
    pubkey: Vec<u8>,
}

impl SigningHeaders {
    /// parse signing headers from an incoming HTTP request
    fn decode(headers: &HeaderMap) -> Result<Self, SigningError> {
        let origin = headers
            .get(HEADER_ORIGIN)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| SigningError::InvalidHeader)?
            .to_string();

        let host = headers
            .get(http::header::HOST)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| SigningError::InvalidHeader)?
            .to_string();

        let timestamp = headers
            .get(HEADER_TIMESTAMP)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| SigningError::InvalidHeader)?
            .to_string();

        let signature = headers
            .get(HEADER_SIGNATURE)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| SigningError::InvalidHeader)?;

        let signature =
            base64::Engine::decode(&B64, signature).map_err(|_| SigningError::InvalidHeader)?;

        let pubkey = headers
            .get(HEADER_PUBKEY)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| SigningError::InvalidHeader)?;

        let pubkey =
            base64::Engine::decode(&B64, pubkey).map_err(|_| SigningError::InvalidHeader)?;

        Ok(Self {
            origin: Hostname(origin),
            host: Hostname(host),
            timestamp,
            signature,
            pubkey,
        })
    }

    /// write signing headers into an outgoing HTTP request
    fn encode(&self, headers: &mut HeaderMap) -> Result<(), SigningError> {
        headers.insert(
            HEADER_ORIGIN,
            HeaderValue::from_str(&self.origin.0).map_err(|_| SigningError::Encoding)?,
        );
        headers.insert(
            http::header::HOST,
            HeaderValue::from_str(&self.host.0).map_err(|_| SigningError::Encoding)?,
        );
        headers.insert(
            HEADER_TIMESTAMP,
            HeaderValue::from_str(&self.timestamp).map_err(|_| SigningError::Encoding)?,
        );
        headers.insert(
            HEADER_SIGNATURE,
            HeaderValue::from_str(&base64::Engine::encode(&B64, &self.signature))
                .map_err(|_| SigningError::Encoding)?,
        );
        headers.insert(
            HEADER_PUBKEY,
            HeaderValue::from_str(&base64::Engine::encode(&B64, &self.pubkey))
                .map_err(|_| SigningError::Encoding)?,
        );
        Ok(())
    }
}

/// an outgoing request that needs to be signed
pub struct OutgoingRequest<'a> {
    pub origin: &'a Hostname,
    pub host: &'a Hostname,
    pub method: &'a str,
    pub path: &'a str,
    pub body: &'a [u8],
}

impl OutgoingRequest<'_> {
    /// sign this request with the given local key, producing headers to attach
    pub fn sign(&self, key: &ServerKeySecret) -> Result<HeaderMap, SigningError> {
        let timestamp = Time::now_utc().to_string();

        let payload = compute_request_payload(
            self.method,
            self.path,
            &self.origin.0,
            self.host,
            &timestamp,
            self.body,
        )?;

        let sig: ed25519_dalek::Signature = key.signing_key.sign(&payload);

        let headers = SigningHeaders {
            origin: self.origin.clone(),
            host: self.host.clone(),
            timestamp,
            signature: sig.to_bytes().to_vec(),
            pubkey: key.pubkey.to_bytes().to_vec(),
        };

        let mut map = HeaderMap::new();
        headers.encode(&mut map)?;
        Ok(map)
    }
}

/// an incoming request that needs to be verified
pub struct IncomingRequest<'a> {
    pub origin: &'a Hostname,
    pub host: &'a Hostname,
    pub method: &'a str,
    pub path: &'a str,
    pub body: &'a [u8],
    pub headers: &'a HeaderMap,
}

impl IncomingRequest<'_> {
    // TODO: don't deserialize over and over again
    fn headers(&self) -> Result<SigningHeaders, SigningError> {
        SigningHeaders::decode(&self.headers)
    }

    /// verify the ed25519 signature
    pub fn verify_signature(&self, verifying_key: &VerifyingKey) -> Result<(), SigningError> {
        let headers = self.headers()?;
        let sig = Signature::from_slice(&headers.signature).map_err(|_| SigningError::Encoding)?;

        let hash = compute_request_payload(
            self.method,
            self.path,
            &self.origin.0,
            self.host,
            &headers.timestamp,
            self.body,
        )?;

        verifying_key
            .verify_strict(&hash, &sig)
            .map_err(|_| SigningError::InvalidSignature)
    }

    /// verify the signature and check the timestamp isn't expired
    pub fn verify(&self, verifying_key: &VerifyingKey) -> Result<(), SigningError> {
        let ts: i64 = self
            .headers()?
            .timestamp
            .parse()
            .map_err(|_| SigningError::InvalidTimestamp)?;

        let req_time = time::OffsetDateTime::from_unix_timestamp(ts)
            .map_err(|_| SigningError::InvalidTimestamp)?;

        let now = time::OffsetDateTime::now_utc();
        let diff = if now >= req_time {
            now - req_time
        } else {
            req_time - now
        };

        let max_age = time::Duration::seconds(SIGNATURE_MAX_AGE.as_secs() as i64);
        if diff > max_age {
            return Err(SigningError::RequestExpired);
        }

        self.verify_signature(verifying_key)
    }
}

impl ServerKeySecret {
    /// generate a new random signing key
    pub fn generate_new() -> Self {
        let mut bytes = [0u8; 32];
        rand::fill(&mut bytes);
        let signing_key = SigningKey::from_bytes(&bytes);
        let pubkey = signing_key.verifying_key();
        let expires_at = Time::now_utc() + KEY_EXPIRY;

        Self {
            pubkey,
            signing_key,
            expires_at,
        }
    }

    /// sign this key
    pub fn sign(&self, hostname: &Hostname) -> ServerKey {
        let mut nonce = [0u8; 32];
        rand::fill(&mut nonce);

        let bytes = compute_key_payload(&nonce, &self.pubkey.to_bytes(), hostname);
        let signature = self.signing_key.sign(&bytes);

        ServerKey {
            alg: ServerKeyAlgorithm::Ed25519,
            pubkey: Binary(Bytes::copy_from_slice(&self.pubkey.to_bytes())),
            nonce: Binary(Bytes::copy_from_slice(&nonce)),
            signature: Binary(Bytes::copy_from_slice(&signature.to_bytes())),
            expires_at: self.expires_at.clone(),
        }
    }
}

impl ServerKey {
    /// verify this key's signature
    pub fn verify(&self, hostname: &Hostname) -> bool {
        let bytes = compute_key_payload(&self.nonce, &self.pubkey, hostname);

        let pubkey_bytes: [u8; 32] = match self.pubkey.0.as_ref().try_into() {
            Ok(v) => v,
            Err(_) => return false,
        };

        let verifying_key = match VerifyingKey::from_bytes(&pubkey_bytes) {
            Ok(v) => v,
            Err(_) => return false,
        };

        let sig = match Signature::from_slice(&self.signature) {
            Ok(v) => v,
            Err(_) => return false,
        };

        verifying_key.verify_strict(&bytes, &sig).is_ok()
    }
}

/// compute the canonical payload to sign for a federation request
///
/// format: `nonce bytes  || pubkey_bytes || hostname`
fn compute_request_payload(
    method: &str,
    path: &str,
    origin: &str,
    host: &str,
    timestamp: &str,
    body: &[u8],
) -> Result<Vec<u8>, SigningError> {
    let has_newlines = [method, path, origin, host, timestamp]
        .iter()
        .any(|field| field.contains('\n'));

    if has_newlines {
        return Err(SigningError::ContainsNewlines);
    }

    let mut bytes = vec![];
    bytes.extend(method.as_bytes());
    bytes.extend(b"\n");
    bytes.extend(path.as_bytes());
    bytes.extend(b"\n");
    bytes.extend(origin.as_bytes());
    bytes.extend(b"\n");
    bytes.extend(host.as_bytes());
    bytes.extend(b"\n");
    bytes.extend(timestamp.as_bytes());
    bytes.extend(b"\n");
    bytes.extend(body);
    Ok(bytes)
}

/// compute the canonical payload to sign for a server key
///
/// format: `method || "\n" || path || "\n" || origin || "\n" || host || "\n" || timestamp || "\n" || body` (where `||` means concatenate)
fn compute_key_payload(nonce: &[u8], pubkey: &[u8], hostname: &Hostname) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&nonce);
    bytes.extend_from_slice(&pubkey);
    bytes.extend_from_slice(hostname.as_bytes());
    bytes
}
