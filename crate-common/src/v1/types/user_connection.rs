//! A connection represents a third party application that has been connected to a user account
//!
//! these can include
//!
//! - oauth "log in with 3rdparty" to log in to lamprey with third parties
//! - oauth "log in with lamprey" to log in to third parties with lamprey
//! - user bots/applications that have access to your account

// TODO: implement
// see application.rs
// see user.rs

use std::collections::HashMap;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::{
    application::{Application, Scope},
    misc::Time,
};

/// who can view this connection
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ConnectionVisibility {
    /// everyone on the internet
    Public,

    /// people who are in mutual rooms with you and mutual friends
    Shared,

    /// people you are friends with
    Friends,

    /// nobody but you
    Private,
}

/// a connection between a user an an external platform
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Connection {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub ty: ConnectionType,

    // TODO: rename to authorized_at?
    pub created_at: Time,

    /// who can see this connection
    pub visibility: ConnectionVisibility,

    pub metadata: ConnectionMetadata,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ConnectionPatch {
    pub visibility: ConnectionVisibility,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "serde", serde(tag = "type"))]
pub enum ConnectionType {
    /// we redirect to external platform to authorize user
    ///
    /// uses same system as oauth for logging in. must be configured by server admin.
    Oauth {
        /// the oauth provider this user is connected to
        provider: String,

        external_id: String,
        external_username: String,
        external_url: String,
    },

    /// external platform redirects to us to authorize user
    //  see discord's "application role connection" system
    Application {
        // maybe include for external platform stuff?
        // external_id: String,
        // external_username: String,
        // external_url: String,
        application: Application,

        /// the scopes this application has access to
        ///
        /// only returned for user
        scopes: Option<Vec<Scope>>,
    },

    #[cfg(any())]
    /// a website
    ///
    /// verified via rel="me" links
    // should i also allow domain connections too?
    Website { url: String, verified: bool },
}

/// metadata/fields displayed for this connection
// TODO: limit number of fields, size of values
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ConnectionMetadata(pub HashMap<String, ConnectionValue>);

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub enum ConnectionValue {
    Int(i64),
    // TODO: string, bool, time
}
