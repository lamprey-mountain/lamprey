/// let bots supply url embeds
/// i probably could add an event and extra endpoint instead of all this
mod unfurl {
    use url::Url;

    use crate::v1::types::UrlEmbed;

    // urlembedrequest -> urlembed
    struct Unfurler {
        url: Url,
    }

    struct Bot {
        /// what to unfurl
        /// eg. https://domain.tld/path/*
        unfurl_patterns: Vec<String>,
    }

    // server -> bot
    struct UnfurlEvent {
        urls: Vec<UnfurlEventInner>,
    }

    struct UnfurlEventInner {
        url: Url,
        user_confirmed: bool,
    }

    // bot -> server POST /api/v1/unfurl/done
    struct UnfurlDone {
        embeds: Vec<UnfurlData>,
    }

    struct UnfurlData {
        url: Url,
        res: UnfurlRes,
    }

    enum UnfurlRes {
        AuthRequired,
        ConfirmationRequired,
        Embed { ttl: u64, embed: UrlEmbed },
    }
}

/// strongly typed errors
mod error {
    enum Error {
        // existing
        MissingAuth,
        BadHeader,
        UnauthSession,
        NotFound,
        MissingPermissions,

        // generic
        BadJson, // message content needs stuff
        BadData,
        Internal,
        Unimplemented,
        Timeout,  // sync
        TooOld,   // sync, 301 if possible otherwise 410(?)
        Conflict, // media already linked
        LimitTooBig,
        UploadTooBig,
        NotDeletable,
        NotEditable,
        Spam,           // maybe disguise this as another error, eh probably not
        Disallowed,     // eg nsfw in non nsfw room/thread
        Oauth,          // state expired
        BadTargetState, // maybe Disallowed, eg. thread deleted -> *
        BadUserType,    // maybe Disallowed, user create
        MediaDownload,  // media import
        MediaProcessing,
        InvalidMediaType, // eg. video where image expected
        InvalidUrl,       // url embed localhost
    }
}

/// bot slash commands
/// unlikely to be implemented
mod slash_commands {
    struct Command {
        name: String,
        description: Option<String>,
        args: Vec<Arg>,
    }

    struct Arg {
        name: String,
        short: Option<String>,
        description: Option<String>,
        required: bool,
        ty: ArgTy,
    }

    enum ArgTy {
        String,
        Int,
        Uint,
        Float,
        Enum(Vec<String>),
        // etc...
        Subcommand(Vec<Arg>),
    }
}

/// automatically create links based on matching text patterns
mod autolink {
    use url::Url;

    struct Autolink {
        prefix: String,
        template: Url,
    }
}

/// animated avatars
mod animated_avatars {
    use serde::{Deserialize, Serialize};
    use url::Url;

    #[cfg(feature = "utoipa")]
    use utoipa::ToSchema;

    #[cfg(feature = "validator")]
    use validator::Validate;

    use crate::v1::types::media::media3::*;

    #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
    #[cfg_attr(feature = "utoipa", derive(ToSchema))]
    #[serde(tag = "type")]
    pub enum Avatar {
        Static {
            #[serde(flatten)]
            image: File<Image>,
        },
        Animated {
            #[serde(flatten)]
            animated: File<Animated>,
        },
    }
}

/// this is a really bad idea
mod font {
    use crate::v1::types::MediaId;

    enum Font {
        /// default; atkinson hyperlegible
        Sans,

        /// which font?
        Serif,

        /// iosevka
        Monospace,

        /// comic sans like
        /// maybe make my own lmao
        Handwriting,

        // === fonts below this line are not meant to be used often ===
        // (and might not be added at all)
        /// excessively fancy and wide
        Cursive,

        /// yeh idk, css specs it though
        Fantasy,

        /// blocky and angular (what 90s people thought the future would be)
        Techno,

        /// pixellated (macimas minecraft font hmm)
        Pixel,

        /// those halloween fonts
        Spoopy,

        /// this probably a terrible idea lmao
        // #[serde(flatten)]
        Custom(MediaId),
    }
}

/// pull out all possible server actions into one big enum
mod action_enum {
    use crate::v1::types::{Permission, UserId};

    // MessageSync is kind of close to this..?
    enum AnyAction {
        MessageCreate,
        MessageUpdate,
        MessageDelete,
        MessageVersionDelete,
        ThreadCreate,
        ThreadUpdate,
    }

    // extract out permittion logic here
    struct AnyActionWrap {
        action: AnyAction,
        actor: UserId,
        perms: Vec<Permission>, // from roles
        header_reason: Option<String>,
        header_nonce: Option<String>,
    }
}

/// fancier ways to interact with bots
///
/// somewhat copied from discord. i don't think i've ever really seen this be
/// used much, and typing text commands generally is more ergonomic and feels
/// better than clicking buttons
///
/// probably wont implement, or will massively revise beforehand
mod interactions {
    /*
    edit interaction config: PATCH  /interactions
    get interactions:        GET    /interactions
    get interaction:         GET    /interactions/{token}
    finish interaction:      PUT    /interactions/{token}/callback
    send extra message:      PUT    /interactions/{token}/message
    edit extra message:      PATCH  /interactions/{token}/message/{message_id}
    delete extra message:    DELETE /interactions/{token}/message/{message_id}
    get extra message:       DELETE /interactions/{token}/message/{message_id}
    delete interaction:      DELETE /interactions/{token}

    interactions expire after 15 minutes (response cant be edited)

    // MessageType::Loading
    // is_loading (probably better)
    // flags: loading 0x01 (do i have flags? nah, is_ works better for now)

    // me -> them webhook
    POST /any/path
    */

    use crate::v1::types::{MessageSync, Permission, Resolved, RoomId, ThreadId};

    struct Permissions(Vec<Permission>);

    struct Interaction {
        token: String,
        event: MessageSync,
        resolved: Resolved,
        context: InteractionContext,
        actor_permissions: Permissions,
        bot_permissions: Permissions,
        ty: InteractionType,
    }

    /// where this interaction happened
    enum InteractionContext {
        Room {
            room_id: RoomId,
        },
        Thread {
            room_id: RoomId,
            thread_id: ThreadId,
        },
        // Message {},
        // User {},
        DmBot {
            room_id: RoomId,
        },
        DmOther {
            room_id: RoomId,
        },
    }

    enum InteractionType {
        MessageButton,
        MessageForm,
        Modal,
        Autocomplete,
    }

    enum InteractionCallback {
        // Action(Action), // do an action in Action? if its valid?
        Reply(String),
        ReplyProcessing,
        Message(String),
        MessageProcessing,
        Autocomplete(Vec<()>),
        // MessageEdit(MessagePatch),
        // Modal(()),
        Return(Vec<u8>), // for redexes
    }
}

/// webhook endpoint api compatibility
mod webhook {
    /*
    using bots as (incoming) webhooks through webhook api compatibility endpoints
    outgoing webhooks might still exist

    POST /webhook/{user_id}/{token} (alias for regular message create, but with token in url)
    POST /webhook/{user_id}/{token}/markdown
    POST /webhook/{user_id}/{token}/slack
    POST /webhook/{user_id}/{token}/discord
    POST /webhook/{user_id}/{token}/github

    it's probably a good idea to work on (bidirectional) bridges instead first
    */

    use url::Url;

    struct DiscordWebhookMessageCreate {
        content: String,
        embeds: Vec<DiscordEmbed>,
        allowed_mentions: DiscordAllowedMentions,
        files: Vec<DiscordFile>,
        flags: u64,

        // not supported?
        username: String,
        avatar_url: String,
        tts: bool,
        components: Vec<DiscordMessageComponent>,
        // form data
        // files: Vec<Bytes>,
        // payload_json: bool,
    }

    struct DiscordEmbed {}
    struct DiscordFile {}
    struct DiscordMessageComponent {}
    struct DiscordAllowedMentions {}

    struct SlackWebhookMessageCreate {
        text: String,
        blocks: Vec<SlackBlock>,
        attachments: Vec<SlackAttachment>,
    }

    // #[serde(rename_all = "snake_case", tag = "type")]
    enum SlackBlock {
        /// NOT the same as markdown!
        /// see https://api.slack.com/reference/surfaces/formatting
        Mrkdwn {
            text: String,
        },
        PlainText {
            text: String,
            emoji: bool,
        },
        Section {
            text: Option<Box<SlackBlock>>,
            accessory: Option<Box<SlackBlock>>,
            fields: Vec<SlackBlock>,
        },
        Header {
            text: Box<SlackBlock>,
        },
        Image {
            image_url: Url,
            alt_text: Option<String>,
        },
        // not supported?
        Actions {
            elements: Vec<SlackBlock>,
        },
        Button {
            text: Box<SlackBlock>,
            style: SlackStyle,
            value: String,
        },
    }

    struct SlackAttachment {
        fallback: String,
        color: String,
        pretext: String,
        author_name: String,
        author_link: Url,
        author_icon: Url,
        title: String,
        title_link: Url,
        text: String,
        fields: Vec<SlackAttachmentField>,
        image_url: Url,
        thumb_url: Url,
        footer: String,
        footer_icon: String,
        ts: u64,
    }

    struct SlackAttachmentField {
        title: String,
        value: String,
        short: bool,
    }

    enum SlackStyle {}

    struct GithubMessageCreate {
        // #[serde(flatten)]
        action: GithubAction,
    }

    // #[serde(rename_all = "snake_case", tag = "action")]
    enum GithubAction {
        CommitComment {
            comment: GithubComment,
            repository: GithubRepository,
            sender: GithubUser,
        },
        Create {
            description: String,
            master_branch: String,
            // #[serde(rename = "ref")]
            ref_name: String,
            ref_type: GithubRef,
            repository: GithubRepository,
            sender: GithubUser,
        },
        Delete {
            // #[serde(rename = "ref")]
            ref_name: String,
            ref_type: GithubRef,
            repository: GithubRepository,
            sender: GithubUser,
        },
        Fork {
            forkee: GithubRepository,
            repository: GithubRepository,
            sender: GithubUser,
        },
        IssueComment {
            repository: GithubRepository,
            sender: GithubUser,
        },
        Issues {
            repository: GithubRepository,
            sender: GithubUser,
        },
        Public {
            repository: GithubRepository,
            sender: GithubUser,
        },
        PullRequest {
            repository: GithubRepository,
            sender: GithubUser,
            pull_request: GithubPullRequest,
            number: u64,
            // assignee: (),
        },
        PullRequestReview {
            pull_request: GithubPullRequest,
            review: GithubPRReview,
            repository: GithubRepository,
            sender: GithubUser,
        },
        PullRequestReviewComment {
            pull_request: GithubPullRequest,
            comment: GithubComment,
            repository: GithubRepository,
            sender: GithubUser,
        },
        PullRequestReviewThread {
            pull_request: GithubPullRequest,
            comment: GithubComment,
            repository: GithubRepository,
            sender: Option<GithubUser>,
            thread: Option<GithubPRRThread>,
        },
        Push {
            compare: Url,
            created: bool,
            deleted: bool,
            forced: bool,
            commits: Vec<GithubCommit>,
            pusher: GithubGitUser,
            repository: GithubRepository,
            sender: Option<GithubUser>,
            // #[serde(rename = "ref")]
            ref_name: String,
            before: String,
            after: String,
        },
        Release {},
        Watch {},
    }

    struct GithubComment {}
    struct GithubRepository {}
    struct GithubUser {}
    struct GithubPullRequest {}
    struct GithubPRReview {}
    struct GithubPRRThread {}

    struct GithubGitUser {
        name: String,
        email: Option<String>,
    }

    struct GithubCommit {
        author: GithubGitUser,
        committer: GithubGitUser,
        id: String,
        message: String,
        // timestamp: Time,
        // would be nice to have diffstat here
        added: Vec<String>,
        modified: Vec<String>,
        removed: Vec<String>,
    }

    // #[serde(rename_all = "lowercase")]
    enum GithubRef {
        Tag,
        Branch,
    }
}

/// rss feeds and mailing lists for each thread?
/// rss is easy enough, mailing lists could be painful
mod rss_email {}

/// slowmode - limit interactions per second
mod slowmode {
    // very unlikely to be implemented
    //
    // if i do impl this, i probably should use leaky bucket or something instead
    // ...but then ui/ux problems

    struct Room {
        /// minimum delay in seconds between creating new threads
        pub slowmode_thread: u64,

        /// default slowmode_message for new threads
        /// is copied, changing this wont change old threads
        pub slowmode_message_default: u64,
    }

    struct Thread {
        /// minimum delay in seconds between creating new messages
        pub slowmode_message: u64,
    }
}
