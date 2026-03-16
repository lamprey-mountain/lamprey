use lamprey_macros::endpoint;
use common::v1::types::{UserWithRelationship, UserIdReq};
/// User get
///
/// Get another user, including your relationship
pub mod user_get {
    use super::*;
    pub struct Request {
        /// the user id
        pub user_id: UserIdReq,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Request {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "Request",
                "user_id",
                &&self.user_id,
            )
        }
    }
    pub struct Response {
        pub user: UserWithRelationship,
    }
    #[automatically_derived]
    impl ::core::fmt::Debug for Response {
        #[inline]
        fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
            ::core::fmt::Formatter::debug_struct_field1_finish(
                f,
                "Response",
                "user",
                &&self.user,
            )
        }
    }
    pub fn __utoipa_path() -> utoipa::openapi::path::PathItem {
        ::core::panicking::panic("not yet implemented")
    }
}
const _: () = {
    let _ = "user";
    let _ = "badge.scope.identify";
};
