/// in what cases this tag is visible
// this might be a bit confusing, but the end goal is to make tags usable
// everywhere. there's an owner room that controls the tag metadata (name,
// description, etc), and it can be added and used in other rooms. but a step
// beyond that would be to let tags be used by *anyone*, even if its not added
// to a room. then you could say that you wanted to see this tag on posts if
// the tag was added by your friend, or by someone in the owner room who has the
// TagApply permission, and so on.
//
// this might be too confusing, so theres a good chance it'll get scrapped

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct TagVisibility {
    /// show this tag on a thread if its added to a room
    room: bool,

    /// show this tag on a thread if it was added by a friend
    friend: bool,

    /// show this tag on a thread if it was added by anyone in the owner room_id with the TagApply permission
    owner_room: bool,
}

impl Default for TagVisibility {
    fn default() -> Self {
        todo!()
    }
}

/// Tag user visibility get (TODO)
///
/// Get user visibility for a tag
#[utoipa::path(
    get,
    path = "/tag/{tag_id}/visibility",
    tags = ["tag"],
    params(("tag_id", description = "Tag id")),
    responses((status = OK, body = TagVisibility, description = "success"))
)]
async fn tag_visibility_get(
    Auth(_session): Auth,
    Path(_tag): Path<TagId>,
    State(_s): State<Arc<ServerState>>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Tag user visibility set (TODO)
///
/// Set user visibility for a tag
#[utoipa::path(
    put,
    path = "/tag/{tag_id}/visibility",
    tags = ["tag"],
    params(("tag_id", description = "Tag id")),
    responses(
        (status = OK, body = TagVisibility, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn tag_visibility_set(
    Auth(_session): Auth,
    Path(_tag): Path<TagId>,
    State(_s): State<Arc<ServerState>>,
    Json(_json): Json<TagVisibility>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}
