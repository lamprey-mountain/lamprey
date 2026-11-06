//! types for ringing users into a call

use lamprey_macros::record;

use crate::{util::registry::export_models, v1::types::UserId};

#[record]
pub struct RingEligibility {
    /// whether ring endpoints can be used
    ///
    /// true in dms and gdms, false otherwise
    pub ringable: bool,
}

#[record]
pub struct RingStart {
    pub user_ids: Vec<UserId>,
}

#[record]
pub struct RingStop {
    pub user_ids: Vec<UserId>,
}

export_models!(RingEligibility, RingStart, RingStop);
