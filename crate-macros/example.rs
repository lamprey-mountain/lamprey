// in common: define new endpoints with lamprey::endpoint macro
// in backend: define a handler for a route with lamprey::handler macro

/// User get
///
/// Get another user, including your relationship
#[lamprey::endpoint(
    get,
    path = "/user/{user_id}",
    tags = ["user"],

    // for badges
    scopes = ["identify", "email"],
    // permissions = ["Foo"],
    // permissions_server = ["Foo"],

    response(status = OK, body = UserWithRelationship, description = "success"),
    errors(UnknownUser), // error codes that can be returned
)]
pub mod user_get {
    pub struct Request {
        /// the user id
        #[path] // path parameter
        pub user_id: UserIdReq,

        // if this endpoint had a query param, this is how you'd do it
        #[query]
        pub foo: String,

        // if this endpoint had a header
        #[header]
        pub name: HeaderValue,

        // override names
        #[query("bar")]
        pub zxcv: String,

        // return this json object
        #[json]
        pub foo: SomeJson,
    }

    pub struct Response {
        #[json]
        pub user: UserWithRelationship,
    }
}

/// Message create
///
/// Send a message to a channel
#[lamprey::endpoint(
    get,
    path = "/channel/{channel_id}/message",
    tags = ["message"],
    scopes = ["full"],
    permissions = ["MessageCreate"],
    permissions_optional = ["MessageAttachments", "MessageEmbeds", "MemberBridge"],
    response(status = CREATED, body = Message, description = "success"),
    response(status = OK, body = Message, description = "already created with same nonce"),
    errors(UnknownChannel),
)]
pub mod user_get {
    pub struct Request {
        /// the user id
        #[path]
        pub channel_id: ChannelId,

        #[json]
        pub body: MessageCreate,
    }

    pub struct Response {
        #[json]
        pub message: Message,
    }
}

#[lamprey::handler(user_get)]
async fn user_get(
    auth: Auth,
    State(s): State<Arc<ServerState>>,
    req: user_get::Request, // handles request parsing (param, query, body)
) -> Result<impl IntoResponse> {
    // ...
}
