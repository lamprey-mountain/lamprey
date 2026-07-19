use std::ops::Deref;

use crate::prelude::*;
use common::{
    v1::types::{
        Session, SessionStatus, User,
        error::{ApiError, ErrorCode},
        federation::Hostname,
        oauth::{Scope, Scopes},
        util::Time,
    },
    v2::types::UserId,
};
use kerosene_core::types::auth::Identity;

pub struct Auth {
    identity: Identity,
}

impl Auth {
    pub fn identity(&self) -> &Identity {
        &self.identity
    }
}

impl Deref for Auth {
    type Target = Identity;

    fn deref(&self) -> &Self::Target {
        &self.identity
    }
}

// TODO: copy crate-backend/src/routes/util/auth.rs here
