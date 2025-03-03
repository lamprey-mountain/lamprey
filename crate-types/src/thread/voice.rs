#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadTypeVoicePublic {
    pub call_id: Option<CallId>,
    pub bitrate: u64,
    pub user_limit: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct ThreadTypeVoicePrivate {
    // what to put here?
}

// struct Call {
//     pub id: CallId, // same as thread_id?
//     pub thread_id: ThreadId,
// }

// struct VoiceState {
//     pub call_id: CallId,
//     pub user_id: UserId,
//     pub joined_at: Time,
//     pub is_muted: bool,
//     pub is_deafened: bool,
//     pub has_voice: bool,
//     pub has_video: bool,
//     pub has_stream: bool, // screen share
// }
