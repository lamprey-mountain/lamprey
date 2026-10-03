use lamprey::{
    v1::types::{
        automod::{AutomodAction, AutomodMatches},
        misc::duration::Duration,
    },
    v2::types::ChannelId,
};

mod indexes;

pub use indexes::{RuleIdx, RuleIndexes};

// TODO: copy kerosene-services/src/services/automod/util.rs

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
#[derive(Debug, Default)]
pub struct Actions {
    /// block creation of the associated resource
    block: Option<String>,

    /// timeout the actor for some time, if possible
    timeout: Option<Duration>,

    /// remove the associated resource
    remove: bool,

    /// send alerts to these channels
    alerts: Vec<ChannelId>,
}

#[derive(Debug)]
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
    pub fn with<S: Into<Scan>>(self, scan: S) -> Self {
        todo!()
    }
}

impl Actions {
    /// merge action(s)
    ///
    /// deduplicates actions
    pub fn with<A: Into<Actions>>(self, actions: A) -> Self {
        todo!()
    }
}

impl From<AutomodAction> for Actions {
    fn from(value: AutomodAction) -> Self {
        match value {
            AutomodAction::Block { message } => todo!(),
            AutomodAction::Timeout { duration } => todo!(),
            AutomodAction::Remove => todo!(),
            AutomodAction::SendAlert { channel_id } => todo!(),
        }
    }
}
