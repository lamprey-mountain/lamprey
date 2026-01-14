use std::{str::FromStr, sync::Arc};

use common::{
    v1::types::{
        automod::{AutomodAction, AutomodRule, AutomodTarget, AutomodTrigger},
        AutomodRuleId, Channel, ChannelCreate, ChannelPatch, MessageCreate, MessagePatch, RoomId,
        RoomMember, User,
    },
    v2::types::message::Message,
};
use dashmap::DashMap;
use linkify::{LinkFinder, LinkKind};
use tracing::warn;
use url::Url;

use crate::{Result, ServerStateInner};

pub struct ServiceAutomod {
    #[allow(unused)] // TEMP
    state: Arc<ServerStateInner>,
    #[allow(unused)] // TEMP
    rulesets: DashMap<RoomId, AutomodRuleset>,
}

struct AutomodRuleset {
    rules: Vec<AutomodRule>,
}

// TODO: move to common?
/// the result of scanning
#[derive(Default)]
struct AutomodResult {
    /// the rules that were triggered
    rules: Vec<AutomodRuleId>,

    /// the resulting actions that should be done
    actions: AutomodResultActions,

    /// what was matched
    matched_text: Option<AutomodResultMatch>,
}

#[derive(Default)]
struct AutomodResultActions {
    inner: Vec<AutomodAction>,
}

struct AutomodResultMatch {
    /// the original text that was matched against
    text: String,

    /// the cured text that was matched against
    cured_text: String,

    /// the substrings in the input text that matched
    matches: Vec<String>,

    /// the keywords in the automod rule that matched
    keywords: Vec<String>,

    /// the regexes in the automod rule that matched
    regexes: Vec<String>,
    // location: AutomodTextLocation,
}

impl AutomodResultActions {
    /// add an action to this action set, deduplicating similar actions
    pub fn add(&mut self, action: &AutomodAction) {
        match action {
            AutomodAction::Block { message } => todo!("return the first message"),
            AutomodAction::Timeout { duration } => todo!("take the maximum duration"),
            AutomodAction::Remove => todo!("deduplicate removes"),
            AutomodAction::SendAlert { channel_id } => todo!("send multiple alerts"),
        }
    }
}

// TODO: compile all regexes, decancers together for performance
impl AutomodRuleset {
    pub fn scan_message_create(&self, req: &MessageCreate) -> AutomodResult {
        let mut result = AutomodResult::default();
        let Some(input) = req.content.as_deref() else {
            // TODO: scan media (in attachments, embeds)
            // TODO: scan attachment fields (filename, alt, etc)
            // TODO: scan embed fields (title, description, etc)
            return result;
        };

        let cured_str = match decancer::cure(&input, decancer::Options::default()) {
            Ok(s) => s,
            Err(err) => {
                // TODO: better error handling
                warn!("failed to cure string {:?}", err);
                return result;
            }
        };

        for rule in &self.rules {
            if rule.target != AutomodTarget::Content {
                continue;
            }

            match &rule.trigger {
                AutomodTrigger::TextKeywords { keywords, allow } => {
                    if !cured_str.find_multiple(allow.iter()).is_empty() {
                        // this is explicitly allowed, so its fine
                        continue;
                    }

                    let found = cured_str.find_multiple(keywords.iter());
                    if found.is_empty() {
                        // no bad words found
                        continue;
                    }

                    let m = result
                        .matched_text
                        .get_or_insert_with(|| AutomodResultMatch {
                            text: input.to_owned(),
                            cured_text: cured_str.to_string(),
                            matches: vec![],
                            keywords: vec![],
                            regexes: vec![],
                        });

                    // TODO: populate m.keywords
                    // TODO: populate m.matches
                    result.rules.push(rule.id);
                    for action in &rule.actions {
                        result.actions.add(action);
                    }
                }
                AutomodTrigger::TextBuiltin { list: _ } => todo!("server builtin lists"),
                AutomodTrigger::TextRegex { deny, allow } => {
                    let deny_set = regex::RegexSetBuilder::new(deny.iter())
                        .build()
                        .expect("already validated to be valid regexes");
                    let allow_set = regex::RegexSetBuilder::new(allow.iter())
                        .build()
                        .expect("already validated to be valid regexes");

                    if allow_set.matches(input).matched_any() {
                        // this is explicitly allowed, so its fine
                        continue;
                    }

                    if !deny_set.matches(input).matched_any() {
                        // no bad regexes matched
                        continue;
                    }

                    let m = result
                        .matched_text
                        .get_or_insert_with(|| AutomodResultMatch {
                            text: input.to_owned(),
                            cured_text: cured_str.to_string(),
                            matches: vec![],
                            keywords: vec![],
                            regexes: vec![],
                        });

                    // TODO: populate m.regexes
                    // TODO: populate m.matches
                    result.rules.push(rule.id);
                    for action in &rule.actions {
                        result.actions.add(action);
                    }
                }
                AutomodTrigger::TextLinks {
                    hostnames,
                    whitelist,
                } => {
                    // PERF: use a trie or something here instead
                    for link in LinkFinder::new().links(input) {
                        if matches!(link.kind(), LinkKind::Url) {
                            // LinkFinder will find email addresses too
                            continue;
                        }

                        let url = Url::from_str(link.as_str())
                            .expect("LinkFinder should only find links");
                        let Some(host) = url.host_str() else {
                            // no hostname, ie. data: uri
                            continue;
                        };

                        for target in hostnames {
                            // FIXME: match subdomains (host = foo.example.com, target = example.com should be a match)
                            // TODO: populate `result.matched_text.matches`, but keep scanning to match other hosts
                        }
                    }

                    // TODO: populate m.matches with links
                    // result.rules.push(rule.id);
                    // for action in &rule.actions {
                    //     result.actions.add(action);
                    // }
                    todo!("regex links")
                }
                AutomodTrigger::MediaScan { scanner } => todo!("media scanning"),
            }
        }

        result
    }

    // TODO: other scanning situations
    pub fn scan_message_update(&self, _message: &Message, _req: &MessagePatch) -> AutomodResult {
        todo!()
    }

    pub fn scan_thread_create(&self, _req: &ChannelCreate) -> AutomodResult {
        todo!()
    }

    pub fn scan_thread_update(&self, _thread: &Channel, _req: &ChannelPatch) -> AutomodResult {
        todo!()
    }

    pub fn scan_member(&self, _member: &RoomMember, _user: &User) -> AutomodResult {
        todo!()
    }
}

impl ServiceAutomod {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            rulesets: DashMap::new(),
        }
    }

    #[allow(unused)] // TEMP
    pub async fn load(&self, room_id: RoomId) -> Result<()> {
        todo!()
    }

    // /// load a document into memory
    // #[allow(unused)] // TEMP
    // pub async fn scan(&self, channel_id: ChannelId, branch_id: DocumentBranchId) -> Result<()> {
    //     todo!()
    // }
}
