//! compiling all automod rules in a room into one scanner

use common::{
    v1::types::automod::{
        AutomodMatch, AutomodMatchFragment, AutomodMediaLocation, AutomodRule, AutomodTarget,
        AutomodTextLocation, AutomodTrigger,
    },
    v2::types::{AutomodRuleId, media::Media},
};
use regex::RegexSet;
use tracing::warn;

use crate::services::automod::util::AutomodScan;
use crate::services::messages::links;

/// A compiled and optimized set of automod rules for a room
pub struct Compiled {
    pub(super) rules: Vec<AutomodRule>,
    regex_set: RegexSet,
    regex_map: Vec<RegexMapping>,
    link_rules: Vec<usize>,
}

struct RegexMapping {
    rule_idx: usize,
    keyword_idx: usize,
    allowed: bool,
}

#[derive(Default, Clone)]
struct RuleState {
    allowed: Option<bool>,
    fragments: Vec<AutomodMatchFragment>,
}

impl Compiled {
    pub fn new(rules: Vec<AutomodRule>) -> Self {
        let mut regexes = vec![];
        let mut regex_map = vec![];
        let mut link_rules = vec![];

        // TODO: validate regexes
        for (rule_idx, rule) in rules.iter().enumerate() {
            match &rule.trigger {
                AutomodTrigger::TextRegex { deny, allow } => {
                    for (keyword_idx, pat) in deny.iter().enumerate() {
                        regexes.push(pat.to_string());
                        regex_map.push(RegexMapping {
                            rule_idx,
                            keyword_idx,
                            allowed: false,
                        });
                    }
                    for (keyword_idx, pat) in allow.iter().enumerate() {
                        regexes.push(pat.to_string());
                        regex_map.push(RegexMapping {
                            rule_idx,
                            keyword_idx,
                            allowed: true,
                        });
                    }
                }
                AutomodTrigger::TextKeywords { deny, allow } => {
                    for (keyword_idx, pat) in deny.iter().enumerate() {
                        let pat = regex::escape(pat.as_str());
                        regexes.push(pat);
                        regex_map.push(RegexMapping {
                            rule_idx,
                            keyword_idx,
                            allowed: false,
                        });
                    }
                    for (keyword_idx, pat) in allow.iter().enumerate() {
                        let pat = regex::escape(pat.as_str());
                        regexes.push(pat);
                        regex_map.push(RegexMapping {
                            rule_idx,
                            keyword_idx,
                            allowed: true,
                        });
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
        // } else {
        //     warn!("Invalid regex pattern in rule {}: {}", rule.id, pat);
        // }

        Self {
            rules,
            regex_set,
            regex_map,
            link_rules,
        }
    }

    /// Scans a scannable item against the compiled rules, only including relevant rule ids.
    pub fn scan<S: Scannable>(&self, item: &S, relevant_rule_ids: &[AutomodRuleId]) -> AutomodScan {
        let mut set = ScannableSet {
            target: item.target(),
            text: vec![],
            media: vec![],
        };
        item.scan(&mut set);

        let mut scan = AutomodScan::default();

        for (text, loc) in set.text {
            let s = self.scan_text(text, set.target, loc, relevant_rule_ids);
            scan.merge(s);
        }

        scan
    }

    fn scan_text(
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

        // 1. scan raw text
        let mut rule_states = vec![RuleState::default(); self.rules.len()];

        // scan raw text
        for regex_idx in self.regex_set.matches(&text).iter() {
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

            // FIXME: keep track of matches/fragments
            // rs.fragments.push();
        }

        // scan decancered text
        for regex_idx in self.regex_set.matches(&cured_text).iter() {
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

            // FIXME: keep track of matches/fragments
            // rs.fragments.push();
        }

        // scan links
        // TODO: populate matches/fragments from link rules
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
        let mut text_matches = AutomodMatch {
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

    fn scan_media(
        &self,
        _media: &Media,
        _target: AutomodTarget,
        _location: AutomodMediaLocation,
    ) -> AutomodScan {
        todo!()
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
    fn visit_media(&mut self, media: &'a Media, location: AutomodMediaLocation);
}

/// utility to collect all scannable text from a Scannable
struct ScannableSet<'a> {
    target: AutomodTarget,
    text: Vec<(&'a str, AutomodTextLocation)>,
    media: Vec<(&'a Media, AutomodMediaLocation)>,
}

impl<'a> Scanner<'a> for ScannableSet<'a> {
    fn visit_text(&mut self, text: &'a str, location: AutomodTextLocation) {
        self.text.push((text, location));
    }

    fn visit_media(&mut self, media: &'a Media, location: AutomodMediaLocation) {
        self.media.push((media, location));
    }
}
