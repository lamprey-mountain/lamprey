use axum::{extract::State, Json};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::ServerState;

use crate::error::Result;
use super::util::Auth;

/// Auth discord init
#[utoipa::path(
    get,
    path = "/session/{session_id}/auth/discord",
    params(
        ("session_id", description = "Session id"),
    ),
    tags = ["session"],
    responses(
        (status = FOUND, description = "success"),
    )
)]
pub async fn auth_discord_init(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Auth discord redirect
#[utoipa::path(
    get,
    path = "/session/{session_id}/auth/discord/redirect",
    params(
        ("session_id", description = "Session id"),
    ),
    tags = ["session"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn auth_discord_redirect(
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Auth discord logout
#[utoipa::path(
    delete,
    path = "/session/{session_id}/auth/discord",
    params(
        ("session_id", description = "Session id"),
    ),
    tags = ["session"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn auth_discord_logout(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Auth discord get
#[utoipa::path(
    get,
    path = "/users/{user_id}/auth/discord",
    params(
        ("session_id", description = "Session id"),
    ),
    tags = ["session"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn auth_discord_get(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

/// Auth discord delete
/// 
/// Delete the link between discord and this user
#[utoipa::path(
    delete,
    path = "/users/{user_id}/auth/discord",
    params(
        ("user_id", description = "User id"),
    ),
    tags = ["session"],
    responses(
        (status = OK, description = "success"),
    )
)]
pub async fn auth_discord_delete(
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    todo!()
}

// /// Auth email set
// #[utoipa::path(
//     put,
//     path = "/users/{user_id}/auth/email",
//     params(
//         ("user_id", description = "User id"),
//     ),
//     tags = ["session"],
//     responses(
//         (status = CREATED, description = "success"),
//         (status = OK, description = "already exists"),
//     )
// )]
// pub async fn auth_email_set(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<()>> {
//     todo!()
// }

// /// Auth email get
// #[utoipa::path(
//     get,
//     path = "/users/{user_id}/auth/email",
//     params(
//         ("user_id", description = "User id"),
//     ),
//     tags = ["session"],
//     responses(
//         (status = OK, description = "success"),
//         (status = NOT_FOUND, description = "doesn't exist"),
//     )
// )]
// pub async fn auth_email_set(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<()>> {
//     todo!()
// }

// /// Auth email delete
// #[utoipa::path(
//     delete,
//     path = "/users/{user_id}/auth/email",
//     params(
//         ("user_id", description = "User id"),
//     ),
//     tags = ["session"],
//     responses(
//         (status = CREATED, description = "success"),
//         (status = OK, description = "already exists"),
//     )
// )]
// pub async fn auth_email_delete(
//     Auth(session): Auth,
//     State(s): State<ServerState>,
// ) -> Result<Json<()>> {
//     todo!()
// }

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        // .routes(routes!(auth_discord_init))
        // .routes(routes!(auth_discord_redirect))
        // .routes(routes!(auth_discord_logout))
        // .routes(routes!(auth_discord_delete))
        // .routes(routes!(auth_discord_get))
        // .routes(routes!(auth_email_exec))
        // .routes(routes!(auth_email_set))
        // .routes(routes!(auth_email_get))
        // .routes(routes!(auth_email_delete))
        // .routes(routes!(auth_totp_set))
        // .routes(routes!(auth_totp_exec))
}
