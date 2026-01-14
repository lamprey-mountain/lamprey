use async_trait::async_trait;
use common::v1::types::automod::{AutomodRule, AutomodRuleCreate, AutomodRuleUpdate};
use common::v1::types::{AutomodRuleId, RoomId};
use sqlx::query;

use super::Postgres;
use crate::data::DataAutomod;
use crate::error::Result;
use crate::types::{AutomodRuleData, DbAutomodTarget};

#[async_trait]
impl DataAutomod for Postgres {
    async fn automod_rule_create(
        &self,
        room_id: RoomId,
        create: AutomodRuleCreate,
    ) -> Result<AutomodRule> {
        let rule_id = AutomodRuleId::new();
        let mut tx = self.pool.begin().await?;

        let data = AutomodRuleData {
            trigger: create.trigger,
            actions: create.actions,
        };

        query!(
            "INSERT INTO automod_rule (id, room_id, name, enabled, data, except_nsfw, include_everyone, target) VALUES ($1, $2, $3, true, $4, $5, $6, $7)",
            *rule_id,
            *room_id,
            create.name,
            serde_json::to_value(data)?,
            create.except_nsfw,
            create.include_everyone,
            DbAutomodTarget::from(create.target) as DbAutomodTarget
        )
        .execute(&mut *tx)
        .await?;

        for role_id in create.except_roles {
            query!(
                "INSERT INTO automod_rule_except_role (rule_id, room_id, role_id) VALUES ($1, $2, $3)",
                *rule_id,
                *room_id,
                *role_id
            )
            .execute(&mut *tx)
            .await?;
        }

        for channel_id in create.except_channels {
            query!(
                "INSERT INTO automod_rule_except_channel (rule_id, room_id, channel_id) VALUES ($1, $2, $3)",
                *rule_id,
                *room_id,
                *channel_id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;

        self.automod_rule_get(rule_id).await
    }

    async fn automod_rule_get(&self, rule_id: AutomodRuleId) -> Result<AutomodRule> {
        let row = query!(
            r#"
            SELECT
                id, room_id, name, enabled, data, except_nsfw, include_everyone, target as "target: DbAutomodTarget",
                coalesce((SELECT json_agg(role_id) FROM automod_rule_except_role WHERE rule_id = id), '[]') as "except_roles!",
                coalesce((SELECT json_agg(channel_id) FROM automod_rule_except_channel WHERE rule_id = id), '[]') as "except_channels!"
            FROM automod_rule
            WHERE id = $1
            "#,
            *rule_id
        )
        .fetch_one(&self.pool)
        .await?;

        let data: AutomodRuleData = serde_json::from_value(row.data)?;

        Ok(AutomodRule {
            id: row.id.into(),
            room_id: row.room_id.into(),
            name: row.name,
            enabled: row.enabled,
            trigger: data.trigger,
            actions: data.actions,
            except_roles: serde_json::from_value(row.except_roles)?,
            except_channels: serde_json::from_value(row.except_channels)?,
            except_nsfw: row.except_nsfw,
            include_everyone: row.include_everyone,
            target: row.target.into(),
        })
    }

    async fn automod_rule_update(
        &self,
        rule_id: AutomodRuleId,
        update: AutomodRuleUpdate,
    ) -> Result<AutomodRule> {
        let mut tx = self.pool.begin().await?;
        let old = self.automod_rule_get(rule_id).await?;

        let name = update.name.unwrap_or(old.name);
        let enabled = update.enabled.unwrap_or(old.enabled);
        let trigger = update.trigger.unwrap_or(old.trigger);
        let actions = update.actions.unwrap_or(old.actions);
        let except_nsfw = update.except_nsfw.unwrap_or(old.except_nsfw);
        let include_everyone = update.include_everyone.unwrap_or(old.include_everyone);
        let target = update.target.unwrap_or(old.target);

        let data = AutomodRuleData { trigger, actions };

        query!(
            "UPDATE automod_rule SET name = $2, enabled = $3, data = $4, except_nsfw = $5, include_everyone = $6, target = $7 WHERE id = $1",
            *rule_id,
            name,
            enabled,
            serde_json::to_value(data)?,
            except_nsfw,
            include_everyone,
            DbAutomodTarget::from(target) as DbAutomodTarget
        )
        .execute(&mut *tx)
        .await?;

        if let Some(except_roles) = update.except_roles {
            query!(
                "DELETE FROM automod_rule_except_role WHERE rule_id = $1",
                *rule_id
            )
            .execute(&mut *tx)
            .await?;
            for role_id in except_roles {
                query!(
                    "INSERT INTO automod_rule_except_role (rule_id, room_id, role_id) VALUES ($1, $2, $3)",
                    *rule_id,
                    *old.room_id,
                    *role_id
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        if let Some(except_channels) = update.except_channels {
            query!(
                "DELETE FROM automod_rule_except_channel WHERE rule_id = $1",
                *rule_id
            )
            .execute(&mut *tx)
            .await?;
            for channel_id in except_channels {
                query!(
                    "INSERT INTO automod_rule_except_channel (rule_id, room_id, channel_id) VALUES ($1, $2, $3)",
                    *rule_id,
                    *old.room_id,
                    *channel_id
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        self.automod_rule_get(rule_id).await
    }

    async fn automod_rule_delete(&self, rule_id: AutomodRuleId) -> Result<()> {
        query!("DELETE FROM automod_rule WHERE id = $1", *rule_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn automod_rule_list(&self, room_id: RoomId) -> Result<Vec<AutomodRule>> {
        let rows = query!(
            r#"
            SELECT
                id, room_id, name, enabled, data, except_nsfw, include_everyone, target as "target: DbAutomodTarget",
                coalesce((SELECT json_agg(role_id) FROM automod_rule_except_role WHERE rule_id = id), '[]') as "except_roles!",
                coalesce((SELECT json_agg(channel_id) FROM automod_rule_except_channel WHERE rule_id = id), '[]') as "except_channels!"
            FROM automod_rule
            WHERE room_id = $1
            "#,
            *room_id
        )
        .fetch_all(&self.pool)
        .await?;

        let mut rules = Vec::new();
        for row in rows {
            let data: AutomodRuleData = serde_json::from_value(row.data)?;
            rules.push(AutomodRule {
                id: row.id.into(),
                room_id: row.room_id.into(),
                name: row.name,
                enabled: row.enabled,
                trigger: data.trigger,
                actions: data.actions,
                except_roles: serde_json::from_value(row.except_roles)?,
                except_channels: serde_json::from_value(row.except_channels)?,
                except_nsfw: row.except_nsfw,
                include_everyone: row.include_everyone,
                target: row.target.into(),
            });
        }
        Ok(rules)
    }
}
