//! compiling all automod rules in a room into one scanner

use std::collections::HashMap;

use common::{
    v1::types::automod::{
        AutomodMatchFragment, AutomodMatchKind, AutomodMatches, AutomodMediaLocation, AutomodRule,
        AutomodTarget, AutomodTextLocation, AutomodTrigger,
    },
    v2::types::{AutomodRuleId, MediaId, media::Media},
};
use kerosene_core::{
    config::Config,
    error::{ApiError, ApiResult, ErrorCode},
};
use regex::{Regex, RegexSet, RegexSetBuilder};
use tracing::warn;

use crate::services::automod::{ServiceAutomod, util::AutomodScan};
use crate::services::messages::links;

/// A compiled and optimized set of automod rules for a room
pub struct Compiled {
    rules: Vec<AutomodRule>,
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
    // PERF: don't store these, read from rules?
    kind_is_keyword: bool,
    original_pattern: String,
}

#[derive(Default, Clone)]
struct RuleState {
    allowed: Option<bool>,
    fragments: Vec<AutomodMatchFragment>,
}

impl ServiceAutomod {
    /// compile some automod rules
    pub fn compile(&self, rules: Vec<AutomodRule>) -> ApiResult<Compiled> {
        let config = self.globals.config();
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

        let regex_set = RegexSetBuilder::new(regexes)
            // TODO: configurable size limit
            // .size_limit(1 << 20)
            // TODO: make these properties configurable via api
            // .case_insensitive(yes)
            // .dot_matches_new_line(yes)
            // NOTE: maybe enable this? if enabled, `\ ` is required to match a literal whitespace and `#` can be used to start a comment
            // .ignore_whitespace(yes)
            .build()
            .map_err(|err| match err {
                regex::Error::Syntax(err) => ApiError::with_message(ErrorCode::RegexSyntax, err),
                regex::Error::CompiledTooBig(_) => ApiError::from_code(ErrorCode::RegexTooComplex),
                _ => ApiError::with_message(
                    ErrorCode::Internal,
                    "unknown internal regex error".to_string(),
                ),
            })?;

        Ok(Compiled {
            rules,
            regex_set,
            regex_map,
            link_rules,
            media_thresholds,
        })
    }
}

impl Compiled {
    pub fn rules(&self) -> &[AutomodRule] {
        &self.rules
    }

    pub(super) fn scan_text(
        &self,
        text: &str,
        target: AutomodTarget,
        location: AutomodTextLocation,
        relevant_rule_ids: &[AutomodRuleId],
    ) -> AutomodScan {
        let mut scan = AutomodScan::default();

        // TODO: allow configuring if decancer should be enabled (maybe disable by default for regex?)
        let cured_text = match decancer::cure(text, decancer::Options::default()) {
            Ok(s) => Some(s),
            Err(err) => {
                warn!("failed to cure string {:?}", err);
                None
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

                // FIXME: regex match iteration is quadratic
                // see https://docs.rs/regex/latest/regex/#iterating-over-matches
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
        if let Some(s) = &cured_text {
            scan_string(s, false);

            // TODO: use decancer's find_multiple instead of regex
            // are the ranges it returns for the decancered string or the raw string?
            // s.find_multiple(["foo", "bar"]);
        }

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
            sanitized_text: cured_text.map(|s| s.to_string()).unwrap_or_default(),
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
