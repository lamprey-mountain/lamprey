//! compiling all automod rules in a room into one scanner

use std::collections::HashMap;

use common::{
    v1::types::automod::{
        AutomodMatchFragment, AutomodMatchKind, AutomodMatches, AutomodMediaLocation, AutomodRule,
        AutomodTarget, AutomodTextLocation, AutomodTrigger,
    },
    v2::types::{AutomodRuleId, MediaId, media::Media},
};
use kerosene_core::config::Config;
use regex::{Regex, RegexSet};
use tracing::warn;

use crate::services::automod::util::AutomodScan;
use crate::services::messages::links;

/// A compiled and optimized set of automod rules for a room
pub struct Compiled {
    pub(super) rules: Vec<AutomodRule>,
    regex_set: RegexSet,
    regex_map: Vec<RegexMapping>,
    link_rules: Vec<usize>,
    media_thresholds: HashMap<String, f32>,
}

struct RegexMapping {
    rule_idx: usize,
    keyword_idx: usize,
    allowed: bool,
    pattern: regex::Regex,
    kind_is_keyword: bool,
    original_pattern: String,
}

#[derive(Default, Clone)]
struct RuleState {
    allowed: Option<bool>,
    fragments: Vec<AutomodMatchFragment>,
}

impl Compiled {
    pub fn new(rules: Vec<AutomodRule>, config: &Config) -> Self {
        let media_thresholds = config
            .moderation
            .automod_media
            .iter()
            .map(|m| (m.key.clone(), m.threshold))
            .collect();

        let mut regexes = vec![];
        let mut regex_map = vec![];
        let mut link_rules = vec![];

        let mut add_pattern = |rule_idx: usize,
                               keyword_idx: usize,
                               pat: &str,
                               allowed: bool,
                               kind_is_keyword: bool| {
            let re_pat = if kind_is_keyword {
                regex::escape(pat)
            } else {
                pat.to_string()
            };
            let pattern = match Regex::new(&re_pat) {
                Ok(pat) => pat,
                Err(_err) => {
                    // TODO: do something with this error? i dont think logging would work well here?
                    return;
                }
            };
            regexes.push(re_pat.clone());
            regex_map.push(RegexMapping {
                rule_idx,
                keyword_idx,
                allowed,
                pattern,
                kind_is_keyword,
                original_pattern: pat.to_string(),
            });
        };

        // TODO: validate regexes
        for (rule_idx, rule) in rules.iter().enumerate() {
            match &rule.trigger {
                AutomodTrigger::TextRegex { deny, allow }
                | AutomodTrigger::TextKeywords { deny, allow } => {
                    let kind_is_keyword =
                        matches!(rule.trigger, AutomodTrigger::TextKeywords { .. });
                    for (keyword_idx, pat) in deny.iter().enumerate() {
                        add_pattern(rule_idx, keyword_idx, pat, false, kind_is_keyword);
                    }
                    for (keyword_idx, pat) in allow.iter().enumerate() {
                        add_pattern(rule_idx, keyword_idx, pat, true, kind_is_keyword);
                    }
                }
                AutomodTrigger::TextLinks { .. } => {
                    link_rules.push(rule_idx);
                }
                // TODO: grab patterns from config file?
                // AutomodTrigger::TextBuiltin { list } => {}
                _ => {}
            }
        }

        let regex_set = RegexSet::new(regexes).expect("better error handling");
        // TODO: better error handling
        // warn!("Invalid regex pattern in rule {}: {}", rule.id, pat);

        Self {
            rules,
            regex_set,
            regex_map,
            link_rules,
            media_thresholds,
        }
    }

    pub(super) fn scan_text(
        &self,
        text: &str,
        target: AutomodTarget,
        location: AutomodTextLocation,
        relevant_rule_ids: &[AutomodRuleId],
    ) -> AutomodScan {
        let mut scan = AutomodScan::default();

        let cured_text = match decancer::cure(&text, decancer::Options::default()) {
            Ok(s) => s.to_string(),
            Err(err) => {
                warn!("failed to cure string {:?}", err);
                text.to_string()
            }
        };

        let mut rule_states = vec![RuleState::default(); self.rules.len()];

        let mut scan_string = |scanned_text: &str, is_raw: bool| {
            for regex_idx in self.regex_set.matches(scanned_text).iter() {
                let meta = &self.regex_map[regex_idx];
                let rule = &self.rules[meta.rule_idx];

                if rule.target != target || !relevant_rule_ids.contains(&rule.id) {
                    continue;
                }

                let rs = &mut rule_states[meta.rule_idx];
                rs.allowed = match (rs.allowed, meta.allowed) {
                    (None, a) => Some(a),
                    (Some(false), false) => Some(false),
                    (Some(_), _) => Some(true),
                };

                for m in meta.pattern.find_iter(scanned_text) {
                    rs.fragments.push(AutomodMatchFragment {
                        // TODO: include both text and sanitized_text for every fragment
                        // FIXME: deduplicate matches on raw and decancered strings
                        // if decancering doesn't change the string, this will generaet two separate fragments (one with text, one with sanitized_text)
                        text: if is_raw {
                            m.as_str().to_string()
                        } else {
                            String::new()
                        },
                        sanitized_text: if is_raw {
                            String::new()
                        } else {
                            m.as_str().to_string()
                        },
                        start: m.start(),
                        end: m.end(),
                        kind: if meta.kind_is_keyword {
                            AutomodMatchKind::Keyword {
                                keywords: meta.original_pattern.clone(),
                            }
                        } else {
                            AutomodMatchKind::Regex {
                                regex: meta.original_pattern.clone(),
                            }
                        },
                    });
                }
            }
        };

        // scan raw text
        scan_string(text, true);

        // scan decancered text
        scan_string(&cured_text, false);

        // scan links
        // TODO: populate matches/fragments from link rules (this may need an api change first)
        if !self.link_rules.is_empty() {
            let extracted_links = links::extract_links(text);

            for rule_idx in &self.link_rules {
                let rule = &self.rules[*rule_idx];

                if rule.target != target || !relevant_rule_ids.contains(&rule.id) {
                    continue;
                }

                if let AutomodTrigger::TextLinks {
                    hostnames,
                    whitelist,
                } = &rule.trigger
                {
                    let mut valid = *whitelist;

                    for url in &extracted_links {
                        let Some(host) = url.host_str() else {
                            continue;
                        };

                        let matches_target = hostnames
                            .iter()
                            .any(|t| host == t || host.ends_with(&format!(".{}", t)));

                        // TODO: theres probably a better way to do this
                        valid = match (*whitelist, matches_target) {
                            (true, true) => false,
                            (true, false) => true,
                            (false, true) => false,
                            (false, false) => true,
                        };
                    }

                    let rs = &mut rule_states[*rule_idx];
                    if valid {
                        rs.allowed = Some(true);
                        if !scan.rule_ids.contains(&rule.id) {
                            scan.rule_ids.push(rule.id);
                        }
                    } else {
                        rs.allowed = Some(false);
                    }
                }
            }
        }

        // collect rules, actions, matches
        let mut text_matches = AutomodMatches {
            text: text.to_string(),
            sanitized_text: cured_text.clone(),
            fragments: vec![],
            location,
        };

        for (idx, rs) in rule_states.into_iter().enumerate() {
            if rs.allowed != Some(false) {
                continue;
            };

            let rule = &self.rules[idx];
            scan.rule_ids.push(rule.id);

            for action in &rule.actions {
                scan.actions.add(action);
            }

            text_matches.fragments.extend(rs.fragments);
        }

        scan.matches = Some(text_matches);
        scan
    }

    pub(super) fn scan_media(
        &self,
        media: &Media,
        target: AutomodTarget,
        location: AutomodMediaLocation,
        relevant_rule_ids: &[AutomodRuleId],
    ) -> AutomodScan {
        let mut scan = AutomodScan::default();

        for (idx, rule) in self.rules.iter().enumerate() {
            if rule.target != target || !relevant_rule_ids.contains(&rule.id) {
                continue;
            }

            if let AutomodTrigger::MediaScan { scanner } = &rule.trigger {
                if let Some(threshold) = self.media_thresholds.get(scanner) {
                    // PERF: maybe i should store scans as a HashMap instead of a Vec?
                    if let Some(result) = media.scans.iter().find(|s| &s.key == scanner) {
                        if result.result >= *threshold {
                            scan.rule_ids.push(rule.id);
                            for action in &rule.actions {
                                scan.actions.add(action);
                            }
                        }
                    }
                }
            }
        }

        scan
    }
}

// TODO: move below to a separate module
/// Defines an item that can be scanned by the automod service.
pub trait Scannable {
    /// Returns the target type of the scannable item.
    fn target(&self) -> AutomodTarget;

    /// Visits every piece of scannable text or media within the item.
    fn scan<'a, S: Scanner<'a>>(&'a self, visitor: &mut S);
}

/// A visitor trait for handling scanned item fields.
pub trait Scanner<'a> {
    /// Handles a piece of text component.
    fn visit_text(&mut self, text: &'a str, location: AutomodTextLocation);

    /// Handles a media component.
    fn visit_media(&mut self, media: MediaId, location: AutomodMediaLocation);
}

/// utility to collect all scannable text from a Scannable
pub(super) struct ScannableSet<'a> {
    pub target: AutomodTarget,
    pub text: Vec<(&'a str, AutomodTextLocation)>,
    pub media: Vec<(MediaId, AutomodMediaLocation)>,
}

impl<'a> Scanner<'a> for ScannableSet<'a> {
    fn visit_text(&mut self, text: &'a str, location: AutomodTextLocation) {
        self.text.push((text, location));
    }

    fn visit_media(&mut self, media: MediaId, location: AutomodMediaLocation) {
        self.media.push((media, location));
    }
}
