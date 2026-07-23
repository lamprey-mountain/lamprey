pub use crate::v1::types::components::acl::Allow;

// // TODO: port AllowCheck
// use crate::{
//     v1::types::{error::ApiResult, interactions::InteractionCreate},
//     v2::types::components::Components,
// };
//
// /// utility to check whether an interaction is allowed
// #[derive(Debug)]
// pub struct AllowCheck<'a> {
//     components: &'a Components,
//     interaction_create: &'a InteractionCreate,
//     room_member: &'a RoomMember,
//     user: &'a User,
//     permissions: Vec<Permission>,
// }
//
// impl<'a> AllowCheck<'a> {
//     /// check whether this interaction can be applied to these components
//     pub fn check(self) -> ApiResult<()> {
//         todo!()
//     }
// }
