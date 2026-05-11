// pub struct ScriptMetadata {
//     #[cfg_attr(feature = "utoipa", schema(min_length = 1, max_length = 64))]
//     #[cfg_attr(feature = "validator", validate(length(min = 1, max = 64)))]
//     pub name: String,

//     #[cfg_attr(
//         feature = "utoipa",
//         schema(required = false, min_length = 1, max_length = 8192)
//     )]
//     #[cfg_attr(feature = "validator", validate(length(min = 1, max = 8192)))]
//     pub description: Option<String>,
//     pub homepage_url: Url,
//     pub authors: Vec<ScriptAuthor>,
//     pub version: Semver,
//     pub license: ScriptLicense,
//     pub origin: Option<ScriptOrigin>,
// }

struct ScriptAuthor {
    name: String,
    user: Option<ScriptAuthorOrigin>,
    url: Option<String>,
}

struct ScriptAuthorOrigin {
    hostname: Hostname,
    user_id: UserId, // origin user id
}

struct ScriptOrigin {
    hostname: Hostname,
    channel_id: ChannelId, // origin channel id
    script_id: ScriptId,   // origin script id
}

/// a semantic version string
// TODO: validate that this is valid semver
// TODO: use this
pub struct Semver(pub String);
