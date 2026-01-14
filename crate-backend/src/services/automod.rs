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
            // return the first message
            AutomodAction::Block { .. } => {
                if !self
                    .inner
                    .iter()
                    .any(|a| matches!(a, AutomodAction::Block { .. }))
                {
                    self.inner.push(action.clone());
                }
            }
            // take the maximum duration
            AutomodAction::Timeout { duration } => {
                let mut found = false;
                for existing in &mut self.inner {
                    if let AutomodAction::Timeout { duration: d } = existing {
                        *d = (*d).max(*duration);
                        found = true;
                        break;
                    }
                }
                if !found {
                    self.inner.push(action.clone());
                }
            }
            AutomodAction::Remove => {
                if !self
                    .inner
                    .iter()
                    .any(|a| matches!(a, AutomodAction::Remove))
                {
                    self.inner.push(AutomodAction::Remove);
                }
            }
            AutomodAction::SendAlert { channel_id } => {
                if !self
                    .inner
                    .iter()
                    .any(|a| matches!(a, AutomodAction::SendAlert { channel_id: cid } if cid == channel_id))
                {
                    self.inner.push(action.clone());
                }
            }
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

                    for mat in found {
                        let matched_cured = cured_str[mat.start..mat.end].to_string();

                        if !m.keywords.contains(&matched_cured) {
                            m.keywords.push(matched_cured.clone());
                        }

                        if !m.matches.contains(&matched_cured) {
                            m.matches.push(matched_cured);
                        }
                    }

                    result.rules.push(rule.id);
                    for action in &rule.actions {
                        result.actions.add(action);
                    }
                }
                AutomodTrigger::TextBuiltin { list: _ } => todo!("server builtin lists"),
                AutomodTrigger::TextRegex { deny, allow } => {
                    let deny_set = regex::RegexSetBuilder::new(deny.iter())
                        .case_insensitive(true)
                        .build()
                        .expect("already validated to be valid regexes");
                    let allow_set = regex::RegexSetBuilder::new(allow.iter())
                        .case_insensitive(true)
                        .build()
                        .expect("already validated to be valid regexes");

                    if allow_set.matches(input).matched_any() {
                        // this is explicitly allowed, so its fine
                        continue;
                    }

                    let matches = deny_set.matches(input);
                    if !matches.matched_any() {
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

                    for index in matches {
                        let pattern = &deny[index];
                        if !m.regexes.contains(pattern) {
                            m.regexes.push(pattern.clone());
                        }

                        let re = regex::RegexBuilder::new(pattern)
                            .case_insensitive(true)
                            .build()
                            .expect("already validated to be valid regexes");

                        for mat in re.find_iter(input) {
                            let matched_text = mat.as_str().to_string();
                            if !m.matches.contains(&matched_text) {
                                m.matches.push(matched_text);
                            }
                        }
                    }

                    result.rules.push(rule.id);
                    for action in &rule.actions {
                        result.actions.add(action);
                    }
                }
                AutomodTrigger::TextLinks {
                    hostnames,
                    whitelist,
                } => {
                    let mut triggered = false;
                    // PERF: use a trie or something here instead
                    for link in LinkFinder::new().links(input) {
                        if !matches!(link.kind(), LinkKind::Url) {
                            // LinkFinder will find email addresses too
                            continue;
                        }

                        let Ok(url) = Url::from_str(link.as_str()) else {
                            continue;
                        };
                        let Some(host) = url.host_str() else {
                            // no hostname, ie. data: uri
                            continue;
                        };

                        let mut matches_target = false;
                        for target in hostnames {
                            if host == target || host.ends_with(&format!(".{}", target)) {
                                matches_target = true;
                                break;
                            }
                        }

                        // if whitelist is true: we want matches_target to be true. if false, violation.
                        // if whitelist is false: we want matches_target to be false. if true, violation.
                        if *whitelist != matches_target {
                            let m = result
                                .matched_text
                                .get_or_insert_with(|| AutomodResultMatch {
                                    text: input.to_owned(),
                                    cured_text: cured_str.to_string(),
                                    matches: vec![],
                                    keywords: vec![],
                                    regexes: vec![],
                                });

                            let link_str = link.as_str().to_string();
                            if !m.matches.contains(&link_str) {
                                m.matches.push(link_str);
                            }
                            triggered = true;
                        }
                    }

                    if triggered {
                        result.rules.push(rule.id);
                        for action in &rule.actions {
                            result.actions.add(action);
                        }
                    }
                }
                AutomodTrigger::MediaScan { scanner } => todo!("media scanning"),
            }
        }

        result
    }

    // TODO: other scanners
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

    /// load the automod ruleset for a room
    pub async fn load(&self, room_id: RoomId) -> Result<AutomodRuleset> {
        todo!()
    }

    // /// load a document into memory
    // #[allow(unused)] // TEMP
    // pub async fn scan(&self, channel_id: ChannelId, branch_id: DocumentBranchId) -> Result<()> {
    //     todo!()
    // }
}
