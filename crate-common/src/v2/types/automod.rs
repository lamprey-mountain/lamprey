use lamprey_macros::record;

use crate::{
    v1::types::{ChannelType, automod::AutomodRuleSummary},
    v2::types::{AutomodRuleId, ChannelId, MediaId, MessageId, RoomId, UserId},
};

pub use crate::v1::types::automod::{
    AutomodMediaLocation as MediaLocation, AutomodTextLocation as TextLocation,
};

/// Matched content found in something scannable
#[record]
pub struct Match {
    pub rule_id: AutomodRuleId,

    #[serde(flatten)]
    pub inner: MatchType,
}

#[record]
#[serde(tag = "type")]
pub enum MatchType {
    TextRegex {
        regex: String,

        #[serde(flatten)]
        inner: MatchText,
    },

    TextKeywords {
        keyword: String,

        #[serde(flatten)]
        inner: MatchText,
    },

    MediaScan {
        /// the id of the piece of media that matched
        media_id: MediaId,

        /// the name of the scanner that was used
        scanner: String,

        /// where this piece of media is
        location: MediaLocation,
    },
}

/// some text that matched
#[record]
pub struct MatchText {
    /// the original text
    pub text: String,

    /// the sanitized text that was matched against
    // NOTE: kerosene uses the decancer crate internally
    pub sanitized_text: String,

    /// each individual match
    pub fragments: Vec<MatchFragment>,

    /// where this piece of text was found
    pub location: TextLocation,
}

/// a fragment of text that matched
#[record]
pub struct MatchFragment {
    /// the substring in the input text that matched
    pub text: String,

    /// the substring in the sanitized input text that matched
    pub sanitized_text: Option<String>,

    /// the byte offset of the start of this fragment
    pub start: usize,

    /// the byte offset of the end this fragment
    pub end: usize,
}

#[record]
pub struct Execution {
    /// the id of the room that this execution happened in
    pub room_id: RoomId,

    /// the user who triggered this execution
    pub user_id: UserId,

    /// the channel this happened in
    ///
    /// Only exists for [`AutomodTarget::Content`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channel: Option<ExecutionChannel>,

    /// The target thing that was affected.
    ///
    /// Only exists for [`AutomodTarget::Content`]. This is the message or thread id.
    ///
    /// This is only set if the target still exists. If the message or thread wass `Block`ed, this will be `None`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<ExecutionTarget>,

    /// The ids of any automod execution message that was sent due to a SendAlert action
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub alert_message_ids: Vec<MessageId>,

    /// the rules that matched
    pub rules: Vec<AutomodRuleSummary>,

    /// the content that was matched
    pub matches: Vec<Match>,

    /// what actions were taken
    pub actions: ExecutionActions,
}

#[record]
pub struct ExecutionActions {
    // TODO: copy kerosene-automod/src/util.rs Actions
    // TODO: maybe rename to ResolvedActions
}

// TODO: finish type
#[record]
#[serde(tag = "type", content = "id")]
pub enum ExecutionTarget {
    Message(MessageId),
    Thread(ChannelId),
}

// TODO: finish type
#[record]
pub struct ExecutionChannel {
    pub id: ChannelId,
    pub name: String,

    #[serde(rename = "type")]
    pub ty: ChannelType,
}
