use lamprey::{
    v1::types::{
        automod::{AutomodAction, AutomodMatches},
        misc::duration::Duration,
    },
    v2::types::ChannelId,
};

mod indexes;

pub use indexes::{RuleIdx, RuleIndexes};

/// the result of scanning
#[derive(Debug, Default)]
pub struct Scan {
    /// the rules that were triggered as a bitset
    rule_indexes: RuleIndexes,

    /// the resulting actions that should be done
    actions: Actions,

    /// what was matched
    matches: Option<Matches>,
}

/// actions to take as a result of scanning
#[derive(Debug, Default, Clone)]
pub struct Actions {
    /// block creation of the associated resource
    ///
    /// if `None`, don't block. if `Some(None)`, its blocked without a custom message.
    pub block: Option<Option<String>>,

    /// timeout the actor for some time, if possible
    pub timeout: Option<Duration>,

    /// remove the associated resource
    pub remove: bool,

    /// send alerts to these channels
    pub alerts: Vec<ChannelId>,
}

#[derive(Debug, Clone)]
pub struct Matches {
    // TODO: use automod v2 types for this
}

impl From<Matches> for AutomodMatches {
    fn from(value: Matches) -> Self {
        todo!()
    }
}

impl Scan {
    /// returns if no rules matched
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.rule_indexes.is_empty()
    }

    /// iterate over matching rule indexes
    #[inline]
    pub fn rule_indexes(&self) -> impl Iterator<Item = RuleIdx> {
        self.rule_indexes.iter()
    }

    /// get what actions should be executed
    #[inline]
    pub fn actions(&self) -> &Actions {
        &self.actions
    }

    /// get what content matched
    #[inline]
    pub fn matches(&self) -> &Matches {
        todo!()
    }

    /// merge a scan
    pub fn merge(&mut self, other: Scan) {
        self.actions.merge(other.actions);
        self.rule_indexes |= other.rule_indexes;
        // FIXME: merge matches
    }
}

impl Actions {
    /// merge action(s)
    ///
    /// deduplicates actions
    pub fn merge(&mut self, other: Actions) {
        if let Some(block) = other.block {
            if self.block.is_none() {
                self.block = Some(block);
                self.remove = false;
            }
        }

        if let Some(timeout) = other.timeout {
            self.timeout = Some(self.timeout.map_or(timeout, |d| d.max(timeout)));
        }

        if other.remove && self.block.is_none() {
            self.remove = true;
        }

        for channel_id in other.alerts {
            if !self.alerts.contains(&channel_id) {
                self.alerts.push(channel_id);
            }
        }
    }
}

impl From<AutomodAction> for Actions {
    fn from(value: AutomodAction) -> Self {
        let mut actions = Actions::default();
        match value {
            AutomodAction::Block { message } => {
                actions.block = Some(message);
            }
            AutomodAction::Timeout { duration } => {
                actions.timeout = Some(duration);
            }
            AutomodAction::Remove => {
                actions.remove = true;
            }
            AutomodAction::SendAlert { channel_id } => {
                actions.alerts.push(channel_id);
            }
        }
        actions
    }
}
