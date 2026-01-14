use std::{str::FromStr, sync::Arc, time::Duration};

use common::{
    v1::types::{
        automod::{AutomodAction, AutomodMatches, AutomodRule, AutomodTarget, AutomodTrigger},
        util::Time,
        AutomodRuleId, Channel, ChannelCreate, ChannelPatch, MessageCreate, MessagePatch, RoomId,
        RoomMember, RoomMemberPatch, User, UserId,
    },
    v2::types::message::Message,
};
use dashmap::DashMap;
use linkify::{LinkFinder, LinkKind};
use tracing::warn;
use url::Url;

use crate::{Error, Result, ServerStateInner};

pub struct ServiceAutomod {
    #[allow(unused)] // TEMP
    state: Arc<ServerStateInner>,
    #[allow(unused)] // TEMP
    rulesets: DashMap<RoomId, Arc<AutomodRuleset>>,
}

#[derive(Debug, Clone)]
pub struct AutomodRuleset {
    rules: Vec<AutomodRule>,
}

// TODO: move to common?
/// the result of scanning
#[derive(Default)]
pub struct AutomodResult {
    /// the rules that were triggered
    rules: Vec<AutomodRuleId>,

    /// the resulting actions that should be done
    actions: AutomodResultActions,

    /// what was matched
    matched_text: Option<AutomodMatches>,
}

impl AutomodResult {
    pub fn is_triggered(&self) -> bool {
        !self.rules.is_empty()
    }

    pub fn rules(&self) -> &[AutomodRuleId] {
        &self.rules
    }

    pub fn actions(&self) -> &[AutomodAction] {
        &self.actions.inner
    }

    pub fn matched(&self) -> Option<&AutomodMatches> {
        self.matched_text.as_ref()
    }
}

#[derive(Default)]
struct AutomodResultActions {
    inner: Vec<AutomodAction>,
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

                    let m = result.matched_text.get_or_insert_with(|| AutomodMatches {
                        text: input.to_owned(),
                        sanitized_text: cured_str.to_string(),
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

                    let m = result.matched_text.get_or_insert_with(|| AutomodMatches {
                        text: input.to_owned(),
                        sanitized_text: cured_str.to_string(),
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
                            let m = result.matched_text.get_or_insert_with(|| AutomodMatches {
                                text: input.to_owned(),
                                sanitized_text: cured_str.to_string(),
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
                AutomodTrigger::MediaScan { scanner: _ } => todo!("media scanning"),
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
    pub async fn load(&self, room_id: RoomId) -> Result<Arc<AutomodRuleset>> {
        if let Some(ruleset) = self.rulesets.get(&room_id) {
            return Ok(ruleset.clone());
        }

        let rules = self.state.data().automod_rule_list(room_id).await?;
        let ruleset = Arc::new(AutomodRuleset { rules });
        self.rulesets.insert(room_id, ruleset.clone());
        Ok(ruleset)
    }

    /// invalidate the automod ruleset for a room
    pub fn invalidate(&self, room_id: RoomId) {
        self.rulesets.remove(&room_id);
    }

    /// enforce automod rules for message creation
    ///
    /// returns true if the message should be removed
    pub async fn enforce_message_create(
        &self,
        room_id: RoomId,
        user_id: UserId,
        scan: &AutomodResult,
    ) -> Result<bool> {
        let mut removed = false;

        for action in scan.actions() {
            match action {
                AutomodAction::Block { message } => {
                    return Err(Error::BadRequest(
                        message
                            .clone()
                            .unwrap_or_else(|| "message blocked by automod".to_string()),
                    ));
                }
                AutomodAction::Timeout { duration } => {
                    let timeout_until = Time::now_utc() + Duration::from_millis(*duration);
                    let data = self.state.data();
                    let srv = self.state.services();
                    data.room_member_patch(
                        room_id,
                        user_id,
                        RoomMemberPatch {
                            timeout_until: Some(Some(timeout_until)),
                            ..Default::default()
                        },
                    )
                    .await?;
                    srv.perms.invalidate_room(user_id, room_id).await;
                    srv.perms
                        .update_timeout_task(user_id, room_id, Some(timeout_until))
                        .await;
                }
                AutomodAction::Remove => {
                    // handled by upstream
                    removed = true;
                }
                AutomodAction::SendAlert { channel_id: _ } => {
                    todo!("SendAlert action not yet implemented (or designed)")
                }
            }
        }

        Ok(removed)
    }
}
