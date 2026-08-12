use crate::v1::types::{
    ChannelId,
    e2ee::{
        cross_signing::{
            CrossSigningBundle, CrossSigningQuery, CrossSigningQueryRequest, CrossSigningSignatures,
        },
        keysharing::{KeyshareRequest, KeyshareRespond, SessionKeyUploadRequest},
        mls::{
            MlsCommitCreate, MlsEpochCreate, MlsKeyPackageClaim, MlsKeyPackageUpload,
            MlsMessageCreate, MlsWelcomeCreate,
        },
    },
};
use lamprey_macros::endpoint;

/// e2ee upload cross signing keys
#[endpoint(
    post,
    path = "/keys/upload-csk",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_upload_cross_signing_keys {
    pub struct Request {
        #[json]
        pub body: CrossSigningBundle,
    }
    pub struct Response {}
}

/// e2ee upload session key
#[endpoint(
    post,
    path = "/keys/upload-session",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_upload_session_key {
    pub struct Request {
        #[json]
        pub body: SessionKeyUploadRequest,
    }
    pub struct Response {}
}

/// e2ee upload mls key packages
#[endpoint(
    post,
    path = "/keys/upload-mls",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_upload_mls_key_packages {
    pub struct Request {
        #[json]
        pub body: MlsKeyPackageUpload,
    }
    pub struct Response {}
}

/// e2ee upload cross signing signatures
#[endpoint(
    post,
    path = "/keys/upload-csk-signatures",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_upload_cross_signing_signatures {
    pub struct Request {
        #[json]
        pub body: CrossSigningSignatures,
    }
    pub struct Response {}
}

/// e2ee query cross signing keys
#[endpoint(
    post,
    path = "/keys/query-csk",
    tags = ["e2ee"],
    scopes = [Full],
    response(OK, description = "success"),
)]
pub mod e2ee_query_cross_signing_keys {
    pub struct Request {
        #[json]
        pub body: CrossSigningQueryRequest,
    }
    pub struct Response {
        #[json]
        pub body: CrossSigningQuery,
    }
}

/// e2ee claim mls key packages
#[endpoint(
    post,
    path = "/keys/claim-mls",
    tags = ["e2ee"],
    scopes = [Full],
    response(OK, description = "success"),
)]
pub mod e2ee_claim_mls_key_packages {
    pub struct Request {
        #[json]
        pub body: MlsKeyPackageClaim,
    }
    pub struct Response {
        #[json]
        pub body: (), // TODO: define response type
    }
}

/// e2ee channel welcome
#[endpoint(
    post,
    path = "/channel/{channel_id}/mls/welcome",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_mls_welcome {
    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
        #[json]
        pub body: MlsWelcomeCreate,
    }
    pub struct Response {}
}

/// e2ee channel commit
#[endpoint(
    post,
    path = "/channel/{channel_id}/mls/commit",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_mls_commit {
    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
        #[json]
        pub body: MlsCommitCreate,
    }
    pub struct Response {}
}

/// e2ee channel message
#[endpoint(
    post,
    path = "/channel/{channel_id}/mls/message",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_mls_message {
    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
        #[json]
        pub body: MlsMessageCreate,
    }
    pub struct Response {}
}

/// e2ee channel epoch
#[endpoint(
    post,
    path = "/channel/{channel_id}/mls/epoch",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_mls_epoch {
    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
        #[json]
        pub body: MlsEpochCreate,
    }
    pub struct Response {}
}

/// e2ee keyshare request
#[endpoint(
    post,
    path = "/channel/{channel_id}/keyshare-request",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_keyshare_request {
    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
        #[json]
        pub body: KeyshareRequest,
    }
    pub struct Response {}
}

/// e2ee keyshare respond
#[endpoint(
    post,
    path = "/channel/{channel_id}/keyshare-respond",
    tags = ["e2ee"],
    scopes = [Full],
    response(NO_CONTENT, description = "success"),
)]
pub mod e2ee_keyshare_respond {
    pub struct Request {
        #[path]
        pub channel_id: ChannelId,
        #[json]
        pub body: KeyshareRespond,
    }
    pub struct Response {}
}
