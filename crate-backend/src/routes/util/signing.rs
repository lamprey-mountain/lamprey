use base64::Engine;
use common::v1::types::{
    federation::{Hostname, ServerKey, ServerKeyAlgorithm},
    util::Time,
};
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey};
use http::{HeaderMap, HeaderName, HeaderValue};

use crate::{error::Result, services::federation::signing::LocalSigningKey, Error};

/// how long a signed request is valid for
pub const SIGNATURE_MAX_AGE: std::time::Duration = std::time::Duration::from_secs(30);

// TODO: use common::v1::types::headers

/// federation signing header: the hostname of the server thats sending this request
pub const HEADER_ORIGIN: &str = "x-origin";

/// federation signing header: the timestamp of this request
pub const HEADER_TIMESTAMP: &str = "x-timestamp";

/// federation signing header: the signature of this request
pub const HEADER_SIGNATURE: &str = "x-signature";

/// federation signing header: the public key that was used to sign this request
pub const HEADER_PUBKEY: &str = "x-pubkey";

/// standard http header: the target host of this request
pub const HEADER_HOST: &str = "host";

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

// FIXME: ensure that none of these fields contain newlines
/// compute the canonical payload to sign for a federation request
///
/// format: `method\npath\norigin\nhost\ntimestamp\nbody`
pub fn compute_payload(
    method: &str,
    path: &str,
    origin: &str,
    host: &str,
    timestamp: &str,
    body: &[u8],
) -> Result<Vec<u8>> {
    let has_newlines = [method, path, origin, host, timestamp]
        .iter()
        .any(|field| field.contains('\n'));

    if has_newlines {
        return Err(Error::BadStatic(
            "newlines are not permitted in signing headers",
        ));
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

        let payload = compute_payload(
            self.method,
            self.path,
            &self.origin.0,
            self.host,
            &timestamp,
            self.body,
        )?;

        let sig: ed25519_dalek::Signature = key.signing_key.sign(&payload);

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

        let hash = compute_payload(
            self.method,
            self.path,
            &self.origin.0,
            self.host,
            &self.headers.timestamp,
            self.body,
        )?;

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

/// create an api `ServerKey` from a `LocalSigningKey`
pub fn sign_server_key(local_key: &LocalSigningKey, hostname: &str) -> ServerKey {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let pubkey_b64 = b64.encode(local_key.pubkey.to_bytes());

    let mut nonce = [0u8; 32];
    rand::fill(&mut nonce);
    let nonce_b64 = b64.encode(nonce);

    let mut message = Vec::new();
    message.extend_from_slice(&nonce);
    message.extend_from_slice(&local_key.pubkey.to_bytes());
    message.extend_from_slice(hostname.as_bytes());

    let sig: ed25519_dalek::Signature = local_key.signing_key.sign(&message);
    let signature_b64 = b64.encode(sig.to_bytes());

    ServerKey {
        alg: ServerKeyAlgorithm::Ed25519,
        pubkey: pubkey_b64,
        nonce: nonce_b64,
        signature: signature_b64,
        expires_at: local_key.expires_at,
    }
}

/// verify an api `ServerKey`'s signature
pub fn verify_server_key(key: &ServerKey, hostname: &str) -> bool {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let pubkey_bytes = match b64.decode(&key.pubkey) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let nonce = match b64.decode(&key.nonce) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let signature_bytes = match b64.decode(&key.signature) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let pubkey_array: [u8; 32] = match pubkey_bytes.clone().try_into() {
        Ok(v) => v,
        Err(_) => return false,
    };

    let verifying_key = match VerifyingKey::from_bytes(&pubkey_array) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let sig = match Signature::from_slice(&signature_bytes) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let mut message = Vec::new();
    message.extend_from_slice(&nonce);
    message.extend_from_slice(&pubkey_bytes);
    message.extend_from_slice(hostname.as_bytes());

    verifying_key.verify(&message, &sig).is_ok()
}
