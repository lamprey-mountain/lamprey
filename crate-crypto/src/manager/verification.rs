use std::collections::HashMap;

use ed25519_dalek::VerifyingKey;
use lamprey::v2::types::{SessionId, UserId};

// cache of stuff to verify identities
pub struct PeerVerification {
    identities: HashMap<UserId, PeerIdentity>,
}

pub struct PeerIdentity {
    master_key: VerifyingKey,
    ssk: VerifyingKey,
    sessions: HashMap<SessionId, PeerSession>,

    /// whether we have verified this user's master key with our usk
    verified_by_me: bool,
}

pub struct PeerSession {
    session_key: VerifyingKey,
    trust_level: TrustLevel,
}

#[derive(Debug, Clone, Copy)]
pub enum TrustLevel {
    CrossSigned, // signed by user's own SSK
    Manual,      // user manually verified (TOFU/emoji SAS)
    Unverified,
}

impl PeerVerification {}

#[cfg(any())]
mod old {
    use std::collections::HashMap;

    use ed25519_dalek::VerifyingKey;
    use lamprey::v2::types::{SessionId, UserId};

    impl TrustCache {
        pub fn ssk_for(&self, user_id: UserId) -> Option<&VerifyingKey> {
            self.identities.get(&user_id).map(|p| &p.ssk)
        }

        pub fn is_verified(&self, user_id: UserId) -> bool {
            self.identities
                .get(&user_id)
                .map(|p| p.verified_by_me)
                .unwrap_or(false)
        }

        pub fn device_trust(&self, user_id: UserId, session_id: SessionId) -> DeviceTrust {
            self.identities
                .get(&user_id)
                .and_then(|p| p.devices.get(&session_id))
                .map(|d| d.trust_status)
                .unwrap_or(DeviceTrust::Untrusted)
        }
    }

    impl TrustCache {
        pub fn ingest_keys_query(&mut self, response: KeysQueryResponse) -> Result<()> {
            for (user_id, bundle) in response.identities {
                // verify master self-signed ssk (from the verify_bundle logic sketched earlier)
                let master = VerifyingKey::from_bytes(&bundle.master.key.try_into()?)?;
                let ssk_pub = VerifyingKey::from_bytes(&bundle.ssk.key.try_into()?)?;
                verify_master_signed(&master, &bundle.ssk)?;

                let devices = response.devices.get(&user_id)
                .map(|ds| ds.iter().map(|d| {
                    let trust = verify_device_signature(&ssk_pub, d)
                        .map(|_| DeviceTrust::CrossSigned)
                        .unwrap_or(DeviceTrust::Untrusted);
                    (d.session_id, PeerDevice { mls_signing_key: /* from d */ todo!(), trust_status: trust })
                }).collect())
                .unwrap_or_default();

                let verified_by_me = self
                    .identities
                    .get(&user_id)
                    .map(|old| old.verified_by_me) // preserve if already set
                    .unwrap_or(false);

                self.identities.insert(
                    user_id,
                    PeerIdentity {
                        master_key: master,
                        ssk: ssk_pub,
                        devices,
                        verified_by_me,
                    },
                );
            }
            Ok(())
        }

        /// call when you complete SAS/QR verification and sign their master with your USK
        pub fn mark_verified(&mut self, user_id: UserId) {
            if let Some(p) = self.identities.get_mut(&user_id) {
                p.verified_by_me = true;
            }
        }

        /// call on IdentityReplaced dispatch — this is the critical invalidation path
        pub fn invalidate(&mut self, user_id: UserId) {
            self.identities.remove(&user_id); // force full re-verify, don't silently carry over trust
        }
    }
}
