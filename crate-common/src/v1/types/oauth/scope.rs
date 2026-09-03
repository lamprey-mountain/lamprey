use core::fmt;
use lamprey_macros::record;
use std::{ops::Deref, str::FromStr};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use crate::v1::types::error::{ApiError, ErrorCode};

/// an oauth scope
// TODO: use strum for this (if it can handle "identify" | "openid")
// TODO: remove "implied scopes"?
#[record]
#[derive(Copy, PartialEq, Eq, Hash, strum::EnumIter)]
#[serde(rename_all = "lowercase")]
pub enum Scope {
    /// basic user profle information
    ///
    /// affects user_get and oauth_userinfo
    Identify,

    /// same as identify
    Openid,

    /// return email address in user profile
    ///
    /// implies `identify`
    Email,

    /// list rooms the user is in
    ///
    /// with `identify`, this returns the room member
    Rooms,

    /// list friends the user has
    ///
    /// implies `identify`
    Relationships,

    /// full read/write access to the user's account (except auth)
    ///
    /// in the future, this will be split into separate scopes
    ///
    /// implies all of the above scopes
    Full,

    /// full read/write access to /auth. implies `full`. very dangerous, will be reworked later!
    ///
    /// implies `full`
    Auth,
}

// TODO: copy macro from crate-common/src/v1/types/permission/mod.rs
bitflags::bitflags! {
    /// compact representation of a set of scopes
    ///
    /// for internal use, not sent to clients
    #[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
    #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
    pub struct ScopeBits: u8 {
        const Identify = 1 << 0;
        const Openid = 1 << 1;
        const Email = 1 << 2;
        const Rooms = 1 << 3;
        const Relationships = 1 << 4;
        const Full = 1 << 5;
        const Auth = 1 << 6;
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize), serde(transparent))]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct Scopes(pub Vec<Scope>);

impl Deref for Scopes {
    type Target = Vec<Scope>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for Scopes {
    type Item = Scope;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Scopes {
    type Item = &'a Scope;
    type IntoIter = std::slice::Iter<'a, Scope>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Scopes {
    /// check if this set of scopes contains a scope
    pub fn has(&self, scope: &Scope) -> bool {
        self.0.iter().any(|s| s.implies(scope))
    }

    /// check that this set of scopes contains a required scope, returning an error if it is missing
    pub fn ensure(&self, scope: &Scope) -> Result<(), ApiError> {
        if self.has(scope) {
            Ok(())
        } else {
            Err(ApiError {
                required_scopes: vec![scope.clone()],
                ..ApiError::from_code(ErrorCode::MissingScopes)
            })
        }
    }

    /// check that this set of scopes contains all required scopes, returning an error if any are missing
    pub fn ensure_all(&self, scopes: &[Scope]) -> Result<(), ApiError> {
        let mut missing = vec![];

        for required_scope in scopes {
            if !self.has(required_scope) {
                missing.push(*required_scope);
            }
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(ApiError {
                required_scopes: missing.clone(),
                ..ApiError::from_code(ErrorCode::MissingScopes)
            })
        }
    }
}

impl Scope {
    /// check if this scope implies another scope
    pub fn implies(&self, other: &Scope) -> bool {
        if self == other {
            return true;
        }

        // NOTE: identify and openid imply each other (they're logically the same scope)
        match self {
            Scope::Auth => true,
            Scope::Full => matches!(
                other,
                Scope::Email | Scope::Identify | Scope::Rooms | Scope::Relationships
            ),
            Scope::Relationships => *other == Scope::Identify,
            Scope::Rooms => *other == Scope::Identify,
            Scope::Email => *other == Scope::Identify,
            Scope::Identify => *other == Scope::Openid,
            Scope::Openid => *other == Scope::Identify,
        }
    }

    /// get a human-readable description of the scope
    // NOTE: keep this in sync with frontend getScopeDescription
    pub fn description(&self) -> &'static str {
        match self {
            Scope::Identify | Scope::Openid => "basic profile information",
            Scope::Email => "return email address in user profile",
            Scope::Rooms => "list rooms the user is in",
            Scope::Relationships => "list friends the user has",
            Scope::Full => "full access to your account",
            Scope::Auth => "full access, including authorization information",
        }
    }
}

impl ScopeBits {
    /// check if this set of scopes contains a scope
    pub fn has(&self, scope: Scope) -> bool {
        self.contains(scope.into())
    }

    /// expand this bitset into a vec of scopes
    // TODO: serde serialize/deserialize from this
    pub fn to_vec(&self) -> Vec<Scope> {
        Scopes::from(*self).0
    }

    /// check that this set of scopes contains a required scope, returning an error if it is missing
    pub fn ensure_single(&self, scope: Scope) -> Result<(), ApiError> {
        if self.has(scope) {
            Ok(())
        } else {
            Err(ApiError {
                required_scopes: vec![scope.clone()],
                ..ApiError::from_code(ErrorCode::MissingScopes)
            })
        }
    }

    /// check that this set of scopes contains all required scopes, returning an error if any are missing
    pub fn ensure_all(&self, required: ScopeBits) -> Result<(), ApiError> {
        let missing = required - *self;

        if missing.is_empty() {
            Ok(())
        } else {
            Err(ApiError {
                required_scopes: missing.to_vec(),
                ..ApiError::from_code(ErrorCode::MissingScopes)
            })
        }
    }
}

impl fmt::Display for Scope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Scope::Identify => "identify",
            Scope::Openid => "openid",
            Scope::Email => "email",
            Scope::Rooms => "rooms",
            Scope::Relationships => "relationships",
            Scope::Full => "full",
            Scope::Auth => "auth",
        };
        f.write_str(s)
    }
}

impl FromStr for Scope {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "identify" => Ok(Scope::Identify),
            "openid" => Ok(Scope::Openid),
            "email" => Ok(Scope::Email),
            "rooms" => Ok(Scope::Rooms),
            "relationships" => Ok(Scope::Relationships),
            "full" => Ok(Scope::Full),
            "auth" => Ok(Scope::Auth),
            _ => Err(()),
        }
    }
}

impl From<Scope> for ScopeBits {
    fn from(value: Scope) -> Self {
        match value {
            Scope::Identify => ScopeBits::Identify,
            Scope::Openid => ScopeBits::Openid,
            Scope::Email => ScopeBits::Email,
            Scope::Rooms => ScopeBits::Rooms,
            Scope::Relationships => ScopeBits::Relationships,
            Scope::Full => ScopeBits::Full,
            Scope::Auth => ScopeBits::Auth,
        }
    }
}

impl From<&Scopes> for ScopeBits {
    fn from(value: &Scopes) -> Self {
        let mut bits = ScopeBits::empty();
        for scope in &value.0 {
            match scope {
                Scope::Identify => bits |= ScopeBits::Identify,
                Scope::Openid => bits |= ScopeBits::Openid,
                Scope::Email => bits |= ScopeBits::Email,
                Scope::Rooms => bits |= ScopeBits::Rooms,
                Scope::Relationships => bits |= ScopeBits::Relationships,
                Scope::Full => bits |= ScopeBits::Full,
                Scope::Auth => bits |= ScopeBits::Auth,
            }
        }
        bits
    }
}

impl From<ScopeBits> for Scopes {
    fn from(value: ScopeBits) -> Self {
        let mut scopes = Vec::new();
        if value.contains(ScopeBits::Identify) {
            scopes.push(Scope::Identify);
        }
        if value.contains(ScopeBits::Openid) {
            scopes.push(Scope::Openid);
        }
        if value.contains(ScopeBits::Email) {
            scopes.push(Scope::Email);
        }
        if value.contains(ScopeBits::Rooms) {
            scopes.push(Scope::Rooms);
        }
        if value.contains(ScopeBits::Relationships) {
            scopes.push(Scope::Relationships);
        }
        if value.contains(ScopeBits::Full) {
            scopes.push(Scope::Full);
        }
        if value.contains(ScopeBits::Auth) {
            scopes.push(Scope::Auth);
        }
        Scopes(scopes)
    }
}
