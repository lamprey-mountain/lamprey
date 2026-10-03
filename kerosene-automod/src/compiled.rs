use std::collections::HashMap;

use kerosene::types::permission::Permissions;
use lamprey::{
    v1::types::{
        Channel, RoomMember, User,
        automod::{AutomodMatchFragment, AutomodMatchKind, AutomodTrigger},
    },
    v2::types::{ChannelId, MessageId, RoleId, RoomId, media::Media},
};
use regex::RegexSet;
use regex_automata::meta::Regex;

use crate::{
    Actions, MediaLocation, RuleId, RuleIdx, RuleIndexes, Scan, Scannable, Target, TextLocation,
};

/// A compiled and optimized set of automod rules for a room
pub struct Compiled {
    rules: Vec<Rule>,
    router: Router,
    regex: Regex,
    regex_map: Vec<RegexMapping>,
    // regex_cache: Cache,
    // TODO
    // link_rules: Vec<usize>,
    // media_thresholds: HashMap<String, f32>,
}

/// a minimal auto moderation rule
pub struct Rule {
    id: RuleId,
    name: String,
    target: Target,
    actions: Actions,
}

struct RegexMapping {
    /// the index of the rule
    rule_idx: RuleIdx,

    /// the index of the keyword
    keyword_idx: u8,

    /// flags for this regex
    ///
    /// ## flags
    ///
    /// - `1 << 0` this is allowed and should bypass matching denied regexes for this trigger
    /// - `1 << 1` this is a keyword trigger. otherwise this is a regex trigger
    flags: u8,

    /// the original keyword pattern
    pattern: Box<str>,
}

/// maps which rules should affect a user
pub struct Router {
    /// rules which don't run in nsfw channels
    except_nsfw: RuleIndexes,

    /// rules which affect everyone
    include_everyone: RuleIndexes,

    /// rules which don't affect certain roles
    except_roles: HashMap<RoleId, RuleIndexes>,

    /// rules which don't affect certain channels
    except_channels: HashMap<ChannelId, RuleIndexes>,
}

// PERF: use cached room member/channel structs
// TODO: move to util
/// context for automoderation
#[derive(Debug, Clone)]
pub struct Context<'a> {
    pub room_id: RoomId,
    pub user: &'a User,
    pub member: Option<&'a RoomMember>,
    pub channel: Option<&'a Channel>,
    pub message_id: Option<MessageId>,
    pub permissions: Permissions,
}

impl Router {
    /// get a set of relevant rules in a context
    pub fn relevant_rules(&self, ctx: Context<'_>) -> RuleIndexes {
        let mut relevant = self.include_everyone;

        // Rules which don't run in nsfw channels
        if let Some(channel) = ctx.channel {
            if channel.nsfw {
                relevant &= !self.except_nsfw;
            }
        }

        // Rules which don't affect certain roles
        if let Some(member) = ctx.member {
            for role_id in &member.roles {
                if let Some(rules) = self.except_roles.get(role_id) {
                    relevant &= !(*rules);
                }
            }
        }

        // Rules which don't affect certain channels
        if let Some(channel_id) = ctx.channel.map(|c| c.id) {
            if let Some(rules) = self.except_channels.get(&channel_id) {
                relevant &= !(*rules);
            }
        }

        relevant
    }
}

impl Compiled {
    // see kerosene-services/src/services/automod/compiled.rs
    pub fn compile() -> Self {
        // compile all keywords and regexes into one regex
        let regex = Regex::builder().build_many(&["foo", "bar"]).unwrap();
        // TODO: generate a "decancered" regex for each keyword pattern
        // TODO: calculate `RegexMapping`s
        // PERF(?): manual cache management *could* be faster? docs say that the builtin thread pool may be good enough though
        // let mut cache = regex.create_cache();

        todo!()
    }

    /// scan something
    pub fn scan<S: Scannable>(&self, s: S, target: Target) -> Scan {
        todo!()
    }

    /// scan a piece of text
    pub fn scan_text<S: AsRef<str>>(
        &self,
        text: S,
        target: Target,
        location: TextLocation,
    ) -> Scan {
        let text = text.as_ref();
        let mut scan = Scan::default();

        let input = regex_automata::Input::new(text);

        // NOTE: this no longer finds overlapping patterns (which is probably a good thing?)
        for m in self.regex.find_iter(input) {
            let original_text = &text[m.span().range()];
            let mapping = &self.regex_map[m.pattern().as_usize()];
            let decancered_text = todo!();
            // TODO: generate `AutomodMatchFragment`s
            // TODO: update scan
        }

        scan
    }

    /// scan a piece of media
    pub fn scan_media(&self, media: &Media, target: Target, location: MediaLocation) -> Scan {
        todo!()
    }
}
