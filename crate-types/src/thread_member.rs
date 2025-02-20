use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

#[cfg(feature = "validator")]
use validator::Validate;

use crate::util::{some_option, Diff};
use crate::UserId;

use super::ThreadId;

// NOTE: maybe i could merge the room_member and thread_member types?

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ThreadMember {
    pub thread_id: ThreadId,
    pub user_id: UserId,

    #[validate(nested)]
    #[serde(flatten)]
    pub membership: ThreadMembership,

    /// When this member's membership last changed (joined, left, was kicked, or banned).
    #[serde(
        serialize_with = "time::serde::rfc3339::serialize",
        deserialize_with = "time::serde::rfc3339::deserialize"
    )]
    pub membership_updated_at: time::OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ThreadMemberPut {
    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 64)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    pub override_name: Option<String>,

    #[cfg_attr(
        feature = "utoipa",
        schema(required = false, min_length = 1, max_length = 8192)
    )]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    pub override_description: Option<String>,
    // pub override_avatar: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[cfg_attr(feature = "validator", derive(Validate))]
pub struct ThreadMemberPatch {
    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
    #[serde(default, deserialize_with = "some_option")]
    pub override_name: Option<Option<String>>,

    #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 8192))]
    #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
    #[serde(default, deserialize_with = "some_option")]
    pub override_description: Option<Option<String>>,
    // #[serde(default, deserialize_with = "some_option")]
    // pub override_avatar: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "membership")]
pub enum ThreadMembership {
    /// joined
    Join {
        override_name: Option<String>,
        override_description: Option<String>,
        // override_avatar: z.string().url().or(z.literal("")),
    },

    /// kicked or left, can rejoin with an invite. todo: can still view messages up until then
    Leave {
        // TODO: copy kick/ban reason here
        // /// user supplied reason why this user was banned
        // reason: Option<String>,
        // /// which user caused the kick, or None if the user left themselves
        // user_id: Option<UserId>,
    },

    /// banned. todo: can still view messages up until they were banned
    Ban {
        // /// user supplied reason why this user was banned
        // reason: Option<String>,
        // /// which user caused the ban
        // user_id: Option<UserId>,
    },
}

impl Diff<ThreadMember> for ThreadMemberPatch {
    fn changes(&self, other: &ThreadMember) -> bool {
        match &other.membership {
            ThreadMembership::Join {
                override_name,
                override_description,
            } => {
                self.override_name.changes(override_name)
                    || self.override_description.changes(override_description)
            }
            _ => false,
        }
    }
}

#[cfg(feature = "validator")]
mod val {
    use super::ThreadMembership;
    use serde_json::json;
    use validator::{Validate, ValidateLength, ValidationError, ValidationErrors};

    impl Validate for ThreadMembership {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut v = ValidationErrors::new();
            match self {
                ThreadMembership::Join {
                    override_name,
                    override_description,
                } => {
                    if override_name
                        .as_ref()
                        .is_some_and(|n| n.validate_length(Some(1), Some(64), None))
                    {
                        let mut err = ValidationError::new("length");
                        err.add_param("max".into(), &json!(64));
                        err.add_param("min".into(), &json!(1));
                        v.add("override_name", err);
                    }
                    if override_description
                        .as_ref()
                        .is_some_and(|n| n.validate_length(Some(1), Some(8192), None))
                    {
                        let mut err = ValidationError::new("length");
                        err.add_param("max".into(), &json!(8192));
                        err.add_param("min".into(), &json!(1));
                        v.add("override_description", err);
                    }
                }
                ThreadMembership::Leave {} => {}
                ThreadMembership::Ban {} => {}
            }
            if v.is_empty() {
                Ok(())
            } else {
                Err(v)
            }
        }
    }
}
