use crate::prelude::*;
use common::{
    v1::types::automod::{AutomodAction, AutomodMatches},
    v2::types::{AutomodRuleId, ChannelId, MessageId, RoomId, UserId},
};
use kerosene_core::error::{ApiError, ErrorCode};

/// the result of scanning
// TODO: make fields private entirely
#[derive(Default)]
pub struct AutomodScan {
    /// the rules that were triggered
    pub(super) rule_ids: Vec<AutomodRuleId>,

    /// the resulting actions that should be done
    pub(super) actions: AutomodResultActions,

    /// what was matched
    pub(super) matches: Option<AutomodMatches>,
    // probably add room_id, channel_id, user_id
    // maybe add message_id, but how would i populate it?
}

#[derive(Default)]
pub struct AutomodResultActions {
    pub inner: Vec<AutomodAction>,
}

// PERF: this would be better than iterating over inner constantly
// #[derive(Default)]
// pub struct AutomodResultActions2 {
//     pub block: Option<String>,
//     pub timeout: Option<u64>,
//     pub remove: bool,
//     pub alerts: Vec<ChannelId>,
// }

pub struct AutomodContext {
    pub room_id: RoomId,
    pub user_id: UserId,
    pub channel_id: Option<ChannelId>,
    pub message_id: Option<MessageId>,
}

// TODO: maybe use this instead of AutomodContext?
// pub struct AutomodContext2<'a> {
//     pub room_id: RoomId,
//     pub user: &'a User,
//     pub room_member: &'a RoomMember,
//     pub channel: Option<&'a Channel>,
//     pub message_id: Option<MessageId>,
// }

impl AutomodScan {
    pub fn is_triggered(&self) -> bool {
        !self.rule_ids.is_empty()
    }

    pub fn rule_ids(&self) -> &[AutomodRuleId] {
        &self.rule_ids
    }

    pub fn actions(&self) -> &[AutomodAction] {
        &self.actions.inner
    }

    /// whether this piece of content should be created but removed immediately
    pub fn should_remove(&self) -> bool {
        self.actions
            .inner
            .iter()
            .any(|a| matches!(a, AutomodAction::Remove))
    }

    /// get the message explaining why this action is blocked, or None if it isn't blocked
    pub fn block_message(&self) -> Option<&str> {
        self.actions.inner.iter().find_map(|a| {
            if let AutomodAction::Block { message } = a {
                message.as_deref()
            } else {
                None
            }
        })
    }

    /// returns whether this action was blocked
    pub fn should_block(&self) -> bool {
        self.block_message().is_some()
    }

    /// ensure this resource isn't blocked
    pub fn ensure_unblocked(&self) -> Result<()> {
        if let Some(message) = self.block_message() {
            let mut err = ApiError::from_code(ErrorCode::Automod);
            err.automod_message = Some(message.to_owned());
            return Err(err.into());
        } else {
            Ok(())
        }
    }

    pub fn merge(&mut self, other: Self) {
        // merge actions
        self.actions.merge(other.actions);

        // merge rule_ids (deduplicate)
        for rule_id in other.rule_ids {
            if !self.rule_ids.contains(&rule_id) {
                self.rule_ids.push(rule_id);
            }
        }

        // merge matches
        if let Some(other_matches) = other.matches {
            if let Some(self_matches) = &mut self.matches {
                self_matches.fragments.extend(other_matches.fragments);
            } else {
                // NOTE: should i handle other_matches being for different text?
                self.matches = Some(other_matches);
            }
        }
    }
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

impl AutomodContext {
    pub fn new(room_id: RoomId, user_id: UserId) -> Self {
        Self {
            room_id,
            user_id,
            channel_id: None,
            message_id: None,
        }
    }
}
