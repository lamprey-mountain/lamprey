use std::{str::FromStr, sync::Arc, time::Duration};

use common::{
    v1::types::{
        automod::{
            AutomodAction, AutomodMatches, AutomodRule, AutomodTarget, AutomodTextLocation,
            AutomodTrigger,
        },
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
    state: Arc<ServerStateInner>,
    rulesets: DashMap<RoomId, Arc<AutomodRuleset>>,
}

#[derive(Debug, Clone)]
pub struct AutomodRuleset {
    rules: Vec<AutomodRule>,
    compiled: CompiledRuleset,
}

#[derive(Debug, Clone, Default)]
struct CompiledRuleset {
    regex_deny_set: Option<regex::RegexSet>,
    regex_deny_map: Vec<usize>, // pattern index -> rule index
    regex_deny_patterns: Vec<String>,
    regex_allow_sets: Vec<Option<regex::RegexSet>>, // rule index -> allow set

    keyword_deny_set: Option<regex::RegexSet>,
    keyword_deny_map: Vec<usize>, // pattern index -> rule index
    keyword_deny_patterns: Vec<String>,
    keyword_allow_sets: Vec<Option<regex::RegexSet>>, // rule index -> allow set
}

/// the result of scanning
#[derive(Default)]
pub struct AutomodResult {
    /// the rules that were triggered
    rule_ids: Vec<AutomodRuleId>,

    /// the resulting actions that should be done
    actions: AutomodResultActions,

    /// what was matched
    matches: Option<AutomodMatches>,
}

impl AutomodResult {
    pub fn is_triggered(&self) -> bool {
        !self.rule_ids.is_empty()
    }

    pub fn rule_ids(&self) -> &[AutomodRuleId] {
        &self.rule_ids
    }

    pub fn actions(&self) -> &[AutomodAction] {
        &self.actions.inner
    }

    pub fn matches(&self) -> Option<&AutomodMatches> {
        self.matches.as_ref()
    }

    pub fn merge(&mut self, other: Self) {
        for id in other.rule_ids {
            if !self.rule_ids.contains(&id) {
                self.rule_ids.push(id);
            }
        }

        self.actions.merge(other.actions);

        if let Some(other_matches) = other.matches {
            if let Some(self_matches) = &mut self.matches {
                for m in other_matches.matches {
                    if !self_matches.matches.contains(&m) {
                        self_matches.matches.push(m);
                    }
                }
                for k in other_matches.keywords {
                    if !self_matches.keywords.contains(&k) {
                        self_matches.keywords.push(k);
                    }
                }
                for r in other_matches.regexes {
                    if !self_matches.regexes.contains(&r) {
                        self_matches.regexes.push(r);
                    }
                }
            } else {
                self.matches = Some(other_matches);
            }
        }
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
                    self.inner.retain(|a| !matches!(a, AutomodAction::Remove));
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
                    .any(|a| matches!(a, AutomodAction::Block { .. } | AutomodAction::Remove))
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

    /// merge another automod action set into this one
    pub fn merge(&mut self, other: Self) {
        for action in &other.inner {
            self.add(action);
        }
    }
}

impl AutomodRuleset {
    pub fn new(rules: Vec<AutomodRule>) -> Self {
        let mut regex_deny_patterns = Vec::new();
        let mut regex_deny_map = Vec::new();
        let mut regex_allow_sets = Vec::with_capacity(rules.len());

        let mut keyword_deny_patterns = Vec::new();
        let mut keyword_deny_map = Vec::new();
        let mut keyword_allow_sets = Vec::with_capacity(rules.len());

        for (rule_idx, rule) in rules.iter().enumerate() {
            let mut regex_allow = None;
            let mut keyword_allow = None;

            match &rule.trigger {
                AutomodTrigger::TextRegex { deny, allow } => {
                    for p in deny {
                        regex_deny_patterns.push(p.clone());
                        regex_deny_map.push(rule_idx);
                    }
                    if !allow.is_empty() {
                        regex_allow = Some(
                            regex::RegexSetBuilder::new(allow)
                                .case_insensitive(true)
                                .build()
                                .expect("valid regexes"),
                        );
                    }
                }
                AutomodTrigger::TextKeywords { keywords, allow } => {
                    for p in keywords {
                        keyword_deny_patterns.push(regex::escape(p));
                        keyword_deny_map.push(rule_idx);
                    }
                    if !allow.is_empty() {
                        keyword_allow = Some(
                            regex::RegexSetBuilder::new(allow.iter().map(|s| regex::escape(s)))
                                .case_insensitive(true)
                                .build()
                                .expect("valid regexes"),
                        );
                    }
                }
                _ => {}
            }
            regex_allow_sets.push(regex_allow);
            keyword_allow_sets.push(keyword_allow);
        }

        let regex_deny_set = if !regex_deny_patterns.is_empty() {
            Some(
                regex::RegexSetBuilder::new(&regex_deny_patterns)
                    .case_insensitive(true)
                    .build()
                    .expect("valid regexes"),
            )
        } else {
            None
        };

        let keyword_deny_set = if !keyword_deny_patterns.is_empty() {
            Some(
                regex::RegexSetBuilder::new(&keyword_deny_patterns)
                    .case_insensitive(true)
                    .build()
                    .expect("valid regexes"),
            )
        } else {
            None
        };

        Self {
            rules,
            compiled: CompiledRuleset {
                regex_deny_set,
                regex_deny_map,
                regex_deny_patterns,
                regex_allow_sets,
                keyword_deny_set,
                keyword_deny_map,
                keyword_deny_patterns,
                keyword_allow_sets,
            },
        }
    }

    /// scans a piece of text and returns the result
    fn scan_text(
        &self,
        text: &str,
        target: AutomodTarget,
        location: AutomodTextLocation,
    ) -> AutomodResult {
        let mut result = AutomodResult::default();

        let cured_str = match decancer::cure(&text, decancer::Options::default()) {
            Ok(s) => s,
            Err(err) => {
                warn!("failed to cure string {:?}", err);
                return result;
            }
        };
        let cured_text = cured_str.to_string();

        // 1. Regex scanning (raw text)
        if let Some(deny_set) = &self.compiled.regex_deny_set {
            let matches = deny_set.matches(text);
            if matches.matched_any() {
                for idx in matches {
                    let rule_idx = self.compiled.regex_deny_map[idx];
                    let rule = &self.rules[rule_idx];

                    if rule.target != target {
                        continue;
                    }

                    if let Some(allow_set) = &self.compiled.regex_allow_sets[rule_idx] {
                        if allow_set.matches(text).matched_any() {
                            continue;
                        }
                    }

                    if !result.rule_ids.contains(&rule.id) {
                        result.rule_ids.push(rule.id);
                        for action in &rule.actions {
                            result.actions.add(action);
                        }
                    }

                    let m = result.matches.get_or_insert_with(|| AutomodMatches {
                        text: text.to_owned(),
                        sanitized_text: cured_text.clone(),
                        matches: vec![],
                        keywords: vec![],
                        regexes: vec![],
                        location: location.clone(),
                    });

                    let pattern = &self.compiled.regex_deny_patterns[idx];
                    if !m.regexes.contains(pattern) {
                        m.regexes.push(pattern.clone());
                    }

                    if let Ok(re) = regex::RegexBuilder::new(pattern)
                        .case_insensitive(true)
                        .build()
                    {
                        for mat in re.find_iter(text) {
                            let matched = mat.as_str().to_string();
                            if !m.matches.contains(&matched) {
                                m.matches.push(matched);
                            }
                        }
                    }
                }
            }
        }

        // 2. Keyword scanning (cured text)
        if let Some(deny_set) = &self.compiled.keyword_deny_set {
            let matches = deny_set.matches(&cured_text);
            if matches.matched_any() {
                for idx in matches {
                    let rule_idx = self.compiled.keyword_deny_map[idx];
                    let rule = &self.rules[rule_idx];

                    if rule.target != target {
                        continue;
                    }

                    if let Some(allow_set) = &self.compiled.keyword_allow_sets[rule_idx] {
                        if allow_set.matches(&cured_text).matched_any() {
                            continue;
                        }
                    }

                    if !result.rule_ids.contains(&rule.id) {
                        result.rule_ids.push(rule.id);
                        for action in &rule.actions {
                            result.actions.add(action);
                        }
                    }

                    let m = result.matches.get_or_insert_with(|| AutomodMatches {
                        text: text.to_owned(),
                        sanitized_text: cured_text.clone(),
                        matches: vec![],
                        keywords: vec![],
                        regexes: vec![],
                        location: location.clone(),
                    });

                    // We store keyword patterns escaped in the set, but we want the original matched text from cured_text.
                    let escaped_pattern = &self.compiled.keyword_deny_patterns[idx];
                    if let Ok(re) = regex::RegexBuilder::new(escaped_pattern)
                        .case_insensitive(true)
                        .build()
                    {
                        for mat in re.find_iter(&cured_text) {
                            let matched = mat.as_str().to_string();
                            if !m.keywords.contains(&matched) {
                                m.keywords.push(matched.clone());
                            }
                            if !m.matches.contains(&matched) {
                                m.matches.push(matched);
                            }
                        }
                    }
                }
            }
        }

        // 3. Other rules (links)
        for rule in &self.rules {
            if rule.target != target {
                continue;
            }

            match &rule.trigger {
                AutomodTrigger::TextLinks {
                    hostnames,
                    whitelist,
                } => {
                    let mut triggered = false;
                    for link in LinkFinder::new().links(text) {
                        if !matches!(link.kind(), LinkKind::Url) {
                            continue;
                        }

                        let Ok(url) = Url::from_str(link.as_str()) else {
                            continue;
                        };
                        let Some(host) = url.host_str() else {
                            continue;
                        };

                        let mut matches_target = false;
                        for target in hostnames {
                            if host == target || host.ends_with(&format!(".{}", target)) {
                                matches_target = true;
                                break;
                            }
                        }

                        if *whitelist != matches_target {
                            let m = result.matches.get_or_insert_with(|| AutomodMatches {
                                text: text.to_owned(),
                                sanitized_text: cured_text.clone(),
                                matches: vec![],
                                keywords: vec![],
                                regexes: vec![],
                                location: location.clone(),
                            });

                            let link_str = link.as_str().to_string();
                            if !m.matches.contains(&link_str) {
                                m.matches.push(link_str);
                            }
                            triggered = true;
                        }
                    }

                    if triggered && !result.rule_ids.contains(&rule.id) {
                        result.rule_ids.push(rule.id);
                        for action in &rule.actions {
                            result.actions.add(action);
                        }
                    }
                }
                _ => {}
            }
        }

        result
    }

    pub fn scan_message_create(&self, req: &MessageCreate) -> AutomodResult {
        let mut result = AutomodResult::default();

        if let Some(t) = req.content.as_deref() {
            result.merge(self.scan_text(
                t,
                AutomodTarget::Content,
                AutomodTextLocation::MessageContent,
            ));
        }

        if let Some(t) = &req.override_name {
            result.merge(self.scan_text(
                t,
                AutomodTarget::Member,
                AutomodTextLocation::MemberNickname,
            ));
        }

        for emb in &req.embeds {
            if let Some(t) = &emb.title {
                result.merge(self.scan_text(
                    t,
                    AutomodTarget::Content,
                    AutomodTextLocation::EmbedTitle,
                ));
            }

            if let Some(t) = &emb.description {
                result.merge(self.scan_text(
                    t,
                    AutomodTarget::Content,
                    AutomodTextLocation::EmbedDescription,
                ));
            }

            if let Some(t) = &emb.author_name {
                result.merge(self.scan_text(
                    t,
                    AutomodTarget::Content,
                    AutomodTextLocation::EmbedAuthorName,
                ));
            }

            if let Some(t) = &emb.author_url {
                result.merge(self.scan_text(
                    t.as_str(),
                    AutomodTarget::Content,
                    AutomodTextLocation::EmbedAuthorUrl,
                ));
            }

            if let Some(t) = &emb.url {
                result.merge(self.scan_text(
                    t.as_str(),
                    AutomodTarget::Content,
                    AutomodTextLocation::EmbedUrl,
                ));
            }
        }

        result
    }

    pub fn scan_message_update(&self, _message: &Message, req: &MessagePatch) -> AutomodResult {
        let mut result = AutomodResult::default();

        if let Some(content) = &req.content {
            if let Some(t) = content.as_deref() {
                result.merge(self.scan_text(
                    t,
                    AutomodTarget::Content,
                    AutomodTextLocation::MessageContent,
                ));
            }
        }

        if let Some(override_name) = &req.override_name {
            if let Some(t) = override_name.as_deref() {
                result.merge(self.scan_text(
                    t,
                    AutomodTarget::Member,
                    AutomodTextLocation::MemberNickname,
                ));
            }
        }

        if let Some(embeds) = &req.embeds {
            for emb in embeds {
                if let Some(t) = &emb.title {
                    result.merge(self.scan_text(
                        t,
                        AutomodTarget::Content,
                        AutomodTextLocation::EmbedTitle,
                    ));
                }

                if let Some(t) = &emb.description {
                    result.merge(self.scan_text(
                        t,
                        AutomodTarget::Content,
                        AutomodTextLocation::EmbedDescription,
                    ));
                }

                if let Some(t) = &emb.author_name {
                    result.merge(self.scan_text(
                        t,
                        AutomodTarget::Content,
                        AutomodTextLocation::EmbedAuthorName,
                    ));
                }

                if let Some(t) = &emb.author_url {
                    result.merge(self.scan_text(
                        t.as_str(),
                        AutomodTarget::Content,
                        AutomodTextLocation::EmbedAuthorUrl,
                    ));
                }

                if let Some(t) = &emb.url {
                    result.merge(self.scan_text(
                        t.as_str(),
                        AutomodTarget::Content,
                        AutomodTextLocation::EmbedUrl,
                    ));
                }
            }
        }

        result
    }

    pub fn scan_thread_create(&self, req: &ChannelCreate) -> AutomodResult {
        let mut result = AutomodResult::default();
        result.merge(self.scan_text(
            &req.name,
            AutomodTarget::Content,
            AutomodTextLocation::ThreadTitle,
        ));

        if let Some(t) = &req.description {
            result.merge(self.scan_text(
                t,
                AutomodTarget::Content,
                AutomodTextLocation::ThreadTopic,
            ));
        }

        result
    }

    pub fn scan_thread_update(&self, _channel: &Channel, req: &ChannelPatch) -> AutomodResult {
        let mut result = AutomodResult::default();

        if let Some(name) = &req.name {
            result.merge(self.scan_text(
                name,
                AutomodTarget::Content,
                AutomodTextLocation::ThreadTitle,
            ));
        }

        if let Some(description) = &req.description {
            if let Some(t) = description.as_deref() {
                result.merge(self.scan_text(
                    t,
                    AutomodTarget::Content,
                    AutomodTextLocation::ThreadTopic,
                ));
            }
        }

        result
    }

    pub fn scan_member(&self, member: &RoomMember, user: &User) -> AutomodResult {
        let mut result = AutomodResult::default();
        result.merge(self.scan_text(
            &user.name,
            AutomodTarget::Member,
            AutomodTextLocation::UserName,
        ));

        if let Some(t) = &member.override_name {
            result.merge(self.scan_text(
                t,
                AutomodTarget::Member,
                AutomodTextLocation::MemberNickname,
            ));
        }

        // NOTE: this may be removed later
        if let Some(t) = &member.override_description {
            result.merge(self.scan_text(
                t,
                AutomodTarget::Member,
                AutomodTextLocation::MemberDescription,
            ));
        }

        result
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
        let ruleset = Arc::new(AutomodRuleset::new(rules));
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
