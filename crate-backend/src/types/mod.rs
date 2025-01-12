use serde::{Deserialize, Serialize};
// use serde::Serialize;
use utoipa::ToSchema;
use uuid7::Uuid;

mod invite;
mod media;
mod member;
mod message;
mod role;
mod room;
mod session;
mod sync;
mod thread;
mod user;
mod ids;

pub use invite::*;
pub use media::*;
pub use member::*;
pub use message::*;
pub use role::*;
pub use room::*;
pub use session::*;
pub use sync::*;
pub use thread::*;
pub use user::*;
pub use ids::*;

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, ToSchema, Serialize, Deserialize)]
#[schema(examples("00000000-0000-0000-0000-0auditlogent"))]
pub struct AuditLogEntryId(Uuid);
