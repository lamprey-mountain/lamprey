use lamprey_macros::endpoint;

/// Upload keys
///
/// Upload MLS key packages
#[endpoint(
    post,
    path = "/key/mls/upload",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_mls_key_upload {
    use bytes::Bytes;

    pub struct Request {
        #[body]
        pub package: Bytes,
    }

    pub struct Response {}
}

/// Query keys
#[endpoint(
    post,
    path = "/key/mls/query",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_mls_key_query {
    use bytes::Bytes;

    pub struct Request {
        #[body]
        pub package: Bytes,
    }

    pub struct Response {}
}

/// Claim keys
#[endpoint(
    post,
    path = "/key/mls/claim",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_mls_key_claim {
    use bytes::Bytes;

    pub struct Request {
        #[body]
        pub package: Bytes,
    }

    pub struct Response {}
}

/// Upload cross signing keys
#[endpoint(
    post,
    path = "/key/cs/keys",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_cs_key_upload {
    use crate::v1::types::e2ee::CrossSigningBundle;

    pub struct Request {
        #[json]
        pub bundle: CrossSigningBundle,
    }

    pub struct Response {}
}

/// Upload cross signing signatures
#[endpoint(
    post,
    path = "/key/cs/signatures",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_cs_signature_publish {
    use crate::v1::types::e2ee::CrossSigningSignature;

    pub struct Request {
        #[json]
        pub signature: CrossSigningSignature,
    }

    pub struct Response {}
}

/// Encryption channel commit
#[endpoint(
    post,
    path = "/channel/{channel_id}/e2ee/commit",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_channel_commit {
    use crate::v1::types::{ChannelId, e2ee::MlsCommitCreate};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub query: MlsCommitCreate,
    }

    pub struct Response {}
}

/// Encryption channel welcome
#[endpoint(
    post,
    path = "/channel/{channel_id}/e2ee/welcome",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_channel_welcome {
    use crate::v1::types::{ChannelId, e2ee::MlsWelcomeCreate};

    pub struct Request {
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub body: MlsWelcomeCreate,
    }

    pub struct Response {}
}
