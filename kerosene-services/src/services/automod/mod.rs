use common::{
    v1::types::{
        Mentions, MentionsUser, MessageAutomodExecution, MessageSync, MessageType, Permission,
        RoomId, RoomMemberPatch,
        automod::{AutomodAction, AutomodRuleStripped, AutomodRuleTest, AutomodRuleTestRequest},
        ids::AUTOMOD_USER_ID,
        util::Time,
    },
    v2::types::AutomodRuleId,
};
use dashmap::DashMap;
use lamprey_backend_data_postgres::DbMessageCreate;

use crate::services::automod::compiled::Scannable;
use crate::{prelude::*, services::automod::compiled::Compiled};

pub use crate::services::automod::util::{AutomodContext, AutomodScan};

mod compiled;
mod scannable;
mod util;

#[cfg(test)]
mod test;

pub struct ServiceAutomod {
    globals: Globals,
    compiled: DashMap<RoomId, Arc<Compiled>>,
}

pub struct AutomodCalculator {
    room_id: RoomId, // TODO: read room_id from here this instead of using ctx?
    globals: Globals,
    compiled: Arc<Compiled>,
}

impl AutomodCalculator {
    // NOTE: should i make this return Result or should it always succeed?
    // TODO: make sure to call srv.automod.enforce() after calc.scan(), check all call sites
    pub async fn scan<S: Scannable>(&self, item: &S, ctx: &AutomodContext) -> AutomodScan {
        let relevant = self.relevant_rules(ctx).await;
        self.compiled.scan(item, &relevant)
    }

    /// get which rules affect this user
    async fn relevant_rules(&self, ctx: &AutomodContext) -> Vec<AutomodRuleId> {
        let srv = self.globals.services();
        let mut data = self
            .globals
            .begin_read()
            .await
            .expect("TODO: better error handling");

        let perms = srv
            .perms
            .for_room(ctx.user_id, ctx.room_id)
            .await
            .expect("TODO: better error handling");

        let member = data
            .room_member_get(ctx.room_id, ctx.user_id)
            .await
            .expect("TODO: better error handling");

        let channel = if let Some(channel_id) = ctx.channel_id {
            Some(
                data.channel_get(channel_id)
                    .await
                    .expect("TODO: better error handling"),
            )
        } else {
            None
        };

        let mut rule_ids = vec![];

        for rule in self.compiled.rules.iter() {
            // 1. check RoomManage exemption
            if perms.has(Permission::RoomEdit) && !rule.include_everyone {
                continue;
            }

            // 2. check role exemptions
            if rule
                .except_roles
                .iter()
                .any(|role_id| member.roles.contains(role_id))
            {
                continue;
            }

            // 3. check channel exemptions
            if let Some(channel_id) = ctx.channel_id {
                if rule.except_channels.contains(&channel_id) {
                    continue;
                }
            }

            // 4. check nsfw exemption
            if rule.except_nsfw {
                if let Some(channel) = &channel {
                    if channel.nsfw {
                        continue;
                    }
                }
            }

            rule_ids.push(rule.id);
        }

        rule_ids
    }

    pub fn test(&self, query: &AutomodRuleTestRequest) -> AutomodRuleTest {
        let relevant_rules: Vec<_> = self.compiled.rules.iter().map(|r| r.id).collect();
        let scan = self.compiled.scan(query, &relevant_rules);
        AutomodRuleTest {
            rules: self
                .compiled
                .rules
                .iter()
                .filter(|r| scan.rule_ids.contains(&r.id))
                .map(|r| r.clone().into())
                .collect(),
            actions: scan.actions.inner,
            matches: scan.matches,
        }
    }
}

impl ServiceAutomod {
    pub fn new(globals: Globals) -> Self {
        Self {
            globals,
            compiled: DashMap::new(),
        }
    }

    /// load an automod calculator for a room
    pub async fn load(&self, room_id: RoomId) -> Result<AutomodCalculator> {
        if let Some(compiled) = self.compiled.get(&room_id) {
            return Ok(AutomodCalculator {
                room_id,
                globals: self.globals.clone(),
                compiled: compiled.clone(),
            });
        }

        let rules = self
            .globals
            .begin_read()
            .await?
            .automod_rule_list(room_id)
            .await?;
        let compiled = Arc::new(Compiled::new(rules));
        self.compiled.insert(room_id, compiled.clone());
        Ok(AutomodCalculator {
            room_id,
            globals: self.globals.clone(),
            compiled,
        })
    }

    /// invalidate the compiled automod rules for a room
    pub fn invalidate(&self, room_id: RoomId) {
        self.compiled.remove(&room_id);
    }

    /// enforce an automod scan
    ///
    /// some actions must be enforced by the caller, namely `Block` and `Remove`
    pub async fn enforce(&self, scan: &AutomodScan, ctx: &AutomodContext) -> Result<()> {
        let srv = self.globals.services();

        let is_blocked = scan
            .actions
            .inner
            .iter()
            .any(|a| matches!(a, AutomodAction::Block { .. }));

        for action in &scan.actions.inner {
            match action {
                AutomodAction::Timeout { duration } => {
                    let room = srv.rooms.get(ctx.room_id, None).await?;
                    let perms = srv.perms.for_room(ctx.user_id, ctx.room_id).await?;
                    if room.owner_id == Some(ctx.user_id)
                        || perms.has(Permission::Admin)
                        || perms.has(Permission::RoomEdit)
                    {
                        // members who are able to edit automod rules don't get timed out
                        continue;
                    }

                    let timeout_until =
                        Time::now_utc() + std::time::Duration::from_millis(*duration);
                    let mut txn = self.globals.begin().await?;
                    txn.room_member_patch(
                        ctx.room_id,
                        ctx.user_id,
                        RoomMemberPatch {
                            timeout_until: Some(Some(timeout_until)),
                            ..Default::default()
                        },
                    )
                    .await?;
                    txn.commit().await?;
                    srv.perms.invalidate_room(ctx.user_id, ctx.room_id).await;
                    srv.perms
                        .update_timeout_task(ctx.user_id, ctx.room_id, Some(timeout_until))
                        .await;

                    let member = self
                        .globals
                        .begin_read()
                        .await?
                        .room_member_get(ctx.room_id, ctx.user_id)
                        .await?;
                    let user = srv.users.get(ctx.user_id, None).await?;
                    self.globals
                        .messaging()
                        .broadcast_room(ctx.room_id, MessageSync::RoomMemberUpdate { member, user })
                        .await?;
                }
                AutomodAction::SendAlert { channel_id } => {
                    let rules: Vec<AutomodRuleStripped> = self
                        .compiled
                        .get(&ctx.room_id)
                        .map(|c| {
                            c.rules
                                .iter()
                                .filter(|r| scan.rule_ids.contains(&r.id))
                                .map(|r| r.clone().into())
                                .collect()
                        })
                        .unwrap_or_default();

                    let execution = MessageAutomodExecution {
                        rules,
                        actions: scan.actions.inner.clone(),
                        matches: scan.matches.clone(),
                        user_id: ctx.user_id,
                        channel_id: ctx.channel_id,
                        flagged_message_id: if is_blocked { None } else { ctx.message_id },
                    };

                    let mut mentions = Mentions::default();
                    if let Ok(user) = srv.users.get(ctx.user_id, None).await {
                        mentions.users.push(MentionsUser {
                            id: ctx.user_id,
                            resolved_name: user.name,
                        });
                    } else {
                        mentions.users.push(MentionsUser {
                            id: ctx.user_id,
                            resolved_name: "Unknown".to_string(),
                        });
                    }

                    let message_create = DbMessageCreate {
                        id: None,
                        channel_id: *channel_id,
                        attachments: vec![],
                        author_id: AUTOMOD_USER_ID,
                        embeds: vec![],
                        components: vec![],
                        message_type: MessageType::AutomodExecution(execution).into(),
                        created_at: None,
                        removed_at: None,
                        mentions,
                        flume: None,
                        interaction: None,
                        ephemeral: false,
                    };

                    let mut txn = self.globals.begin().await?;
                    let msg_id = txn.message_create(message_create).await?;
                    txn.commit().await?;

                    let message = self
                        .globals
                        .begin_read()
                        .await?
                        .message_get(*channel_id, msg_id)
                        .await?;
                    let msg = MessageSync::MessageCreate { message };
                    self.globals
                        .messaging()
                        .broadcast_channel(*channel_id, msg)
                        .await?;
                }
                AutomodAction::Block { .. } | AutomodAction::Remove => {
                    // not handled by this method
                }
            }
        }

        Ok(())
    }
}
