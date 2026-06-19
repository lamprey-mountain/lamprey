use core::fmt;

use async_trait::async_trait;
use sqlx::{SqlitePool, query};

use crate::{
    bridge::{Message, Portal, PortalDiscord, PortalId, PortalLamprey, Realm, RealmId, User},
    prelude::*,
};

#[async_trait]
pub trait Database: fmt::Debug + Send + Sync {
    async fn realm_create(&self, realm: Realm) -> Result<RealmId>;
    async fn realm_update(&self, id: RealmId, realm: Realm) -> Result<()>;
    async fn realm_delete(&self, id: RealmId) -> Result<()>;
    async fn realm_list(&self) -> Result<Vec<(RealmId, Realm)>>;

    async fn portal_create(&self, portal: Portal) -> Result<PortalId>;
    async fn portal_update(&self, id: PortalId, portal: Portal) -> Result<()>;
    async fn portal_delete(&self, id: PortalId) -> Result<()>;
    async fn portal_list(&self) -> Result<Vec<(PortalId, Portal)>>;

    async fn message_create(&self, portal_id: PortalId, message: Message) -> Result<()>;
    async fn message_delete(
        &self,
        portal_id: PortalId,
        source_platform: String,
        source_id: String,
    ) -> Result<()>;

    // TODO: rename to user_foo
    async fn puppet_create(&self, puppet: User) -> Result<()>;
    async fn puppet_get_by_lamprey_id(&self, lamprey_id: String) -> Result<Option<User>>;
    async fn puppet_get_by_discord_id(&self, discord_id: String) -> Result<Option<User>>;
    async fn puppet_delete(&self, lamprey_id: String) -> Result<()>;
}

#[derive(Debug)]
pub struct SqliteDatabase {
    pool: SqlitePool,
}

impl SqliteDatabase {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl Database for SqliteDatabase {
    async fn realm_create(&self, realm: Realm) -> Result<RealmId> {
        let continuous = if realm.continuous { 1i64 } else { 0i64 };
        let id = query!("INSERT INTO realm (continuous) VALUES (?)", continuous)
            .execute(&self.pool)
            .await?
            .last_insert_rowid() as RealmId;
        Ok(id)
    }

    async fn realm_update(&self, id: RealmId, realm: Realm) -> Result<()> {
        let continuous = if realm.continuous { 1i64 } else { 0i64 };
        query!(
            "UPDATE realm SET continuous = ? WHERE id = ?",
            continuous,
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn realm_delete(&self, id: RealmId) -> Result<()> {
        query!("DELETE FROM realm WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn realm_list(&self) -> Result<Vec<(RealmId, Realm)>> {
        let rows = query!("SELECT id, continuous FROM realm")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.id as RealmId,
                    Realm {
                        continuous: r.continuous,
                    },
                )
            })
            .collect())
    }

    async fn portal_create(&self, portal: Portal) -> Result<PortalId> {
        let lamprey = portal.lamprey.as_ref();
        let discord = portal.discord.as_ref();

        let lamprey_channel_id = lamprey.map(|l| l.channel_id.to_string());
        let lamprey_room_id = lamprey.map(|l| l.room_id.to_string());
        let lamprey_last_id = lamprey.map(|l| l.last_id.to_string());
        let discord_guild_id = discord.map(|d| d.guild_id.to_string());
        let discord_parent_id = discord.and_then(|d| d.parent_id.as_ref().map(|id| id.to_string()));
        let discord_channel_id = discord.map(|d| d.channel_id.to_string());
        let discord_webhook_url = discord.map(|d| d.webhook_url.to_string());
        let discord_last_id = discord.map(|d| d.last_id.to_string());

        let id = query!(
            "INSERT INTO portal (realm_id, lamprey_channel_id, lamprey_room_id, lamprey_last_id, discord_guild_id, discord_parent_id, discord_channel_id, discord_webhook_url, discord_last_id) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            portal.realm_id,
            lamprey_channel_id,
            lamprey_room_id,
            lamprey_last_id,
            discord_guild_id,
            discord_parent_id,
            discord_channel_id,
            discord_webhook_url,
            discord_last_id
        )
        .execute(&self.pool)
        .await?
        .last_insert_rowid() as PortalId;
        Ok(id)
    }

    async fn portal_update(&self, id: PortalId, portal: Portal) -> Result<()> {
        let lamprey = portal.lamprey.as_ref();
        let discord = portal.discord.as_ref();

        let lamprey_channel_id = lamprey.map(|l| l.channel_id.to_string());
        let lamprey_room_id = lamprey.map(|l| l.room_id.to_string());
        let lamprey_last_id = lamprey.map(|l| l.last_id.to_string());
        let discord_guild_id = discord.map(|d| d.guild_id.to_string());
        let discord_parent_id = discord.and_then(|d| d.parent_id.as_ref().map(|id| id.to_string()));
        let discord_channel_id = discord.map(|d| d.channel_id.to_string());
        let discord_webhook_url = discord.map(|d| d.webhook_url.to_string());
        let discord_last_id = discord.map(|d| d.last_id.to_string());

        query!(
            "UPDATE portal SET realm_id = ?, lamprey_channel_id = ?, lamprey_room_id = ?, lamprey_last_id = ?, discord_guild_id = ?, discord_parent_id = ?, discord_channel_id = ?, discord_webhook_url = ?, discord_last_id = ? WHERE id = ?",
            portal.realm_id,
            lamprey_channel_id,
            lamprey_room_id,
            lamprey_last_id,
            discord_guild_id,
            discord_parent_id,
            discord_channel_id,
            discord_webhook_url,
            discord_last_id,
            id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn portal_delete(&self, id: PortalId) -> Result<()> {
        query!("DELETE FROM portal WHERE id = ?", id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn portal_list(&self) -> Result<Vec<(PortalId, Portal)>> {
        let rows = query!("SELECT id, realm_id, lamprey_channel_id, lamprey_room_id, lamprey_last_id, discord_guild_id, discord_parent_id, discord_channel_id, discord_webhook_url, discord_last_id FROM portal")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.id as PortalId,
                    Portal {
                        realm_id: r.realm_id.map(|id| id as RealmId),
                        lamprey: r.lamprey_channel_id.map(|channel_id| PortalLamprey {
                            channel_id: channel_id.parse().unwrap(),
                            room_id: r.lamprey_room_id.as_ref().unwrap().parse().unwrap(),
                            last_id: r.lamprey_last_id.as_ref().unwrap().parse().unwrap(),
                        }),
                        discord: r.discord_channel_id.map(|channel_id| PortalDiscord {
                            guild_id: r.discord_guild_id.as_ref().unwrap().parse().unwrap(),
                            parent_id: r.discord_parent_id.as_ref().map(|id| id.parse().unwrap()),
                            channel_id: channel_id.parse().unwrap(),
                            webhook_url: r.discord_webhook_url.as_ref().unwrap().parse().unwrap(),
                            last_id: r.discord_last_id.as_ref().unwrap().parse().unwrap(),
                        }),
                    },
                )
            })
            .collect())
    }

    async fn message_create(&self, portal_id: PortalId, message: Message) -> Result<()> {
        let source_platform = message.source_platform.to_string();
        let mut txn = self.pool.begin().await?;
        let message_id = query!(
            "INSERT INTO message (portal_id, source_platform, source_id) VALUES (?, ?, ?)",
            portal_id,
            source_platform,
            message.source_id
        )
        .execute(&mut *txn)
        .await?
        .last_insert_rowid() as u32;

        for (lamprey_media_id, discord_attachment_id) in message.attachments {
            let lamprey_media_str = lamprey_media_id.to_string();
            let discord_attachment_str = discord_attachment_id.to_string();
            query!(
                "INSERT INTO message_attachment (message_id, lamprey_media_id, discord_attachment_id) VALUES (?, ?, ?)",
                message_id,
                lamprey_media_str,
                discord_attachment_str
            )
            .execute(&mut *txn)
            .await?;
        }

        txn.commit().await?;
        Ok(())
    }

    async fn message_delete(
        &self,
        portal_id: PortalId,
        source_platform: String,
        source_id: String,
    ) -> Result<()> {
        // Need to find the message id first
        let message = query!(
            "SELECT id FROM message WHERE portal_id = ? AND source_platform = ? AND source_id = ?",
            portal_id,
            source_platform,
            source_id
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(msg) = message {
            let message_id = msg.id;

            query!(
                "DELETE FROM message_attachment WHERE message_id = ?",
                message_id
            )
            .execute(&self.pool)
            .await?;

            query!("DELETE FROM message WHERE id = ?", message_id)
                .execute(&self.pool)
                .await?;
        }
        Ok(())
    }

    async fn puppet_create(&self, puppet: User) -> Result<()> {
        let lamprey_id = puppet.lamprey_id.to_string();
        let discord_id = puppet.discord_id.to_string();
        let discord_avatar_url = puppet.discord_avatar_url.map(|u| u.to_string());
        let discord_banner_url = puppet.discord_banner_url.map(|u| u.to_string());

        query!(
            "INSERT INTO \"user\" (lamprey_id, discord_id, discord_avatar_url, discord_banner_url) VALUES (?, ?, ?, ?)",
            lamprey_id,
            discord_id,
            discord_avatar_url,
            discord_banner_url
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn puppet_get_by_lamprey_id(&self, lamprey_id: String) -> Result<Option<User>> {
        let row = query!(
            "SELECT lamprey_id, discord_id, discord_avatar_url, discord_banner_url FROM \"user\" WHERE lamprey_id = ?",
            lamprey_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| User {
            lamprey_id: r.lamprey_id.parse().unwrap(),
            discord_id: r.discord_id.parse().unwrap(),
            discord_avatar_url: r.discord_avatar_url,
            discord_banner_url: r.discord_banner_url,
        }))
    }

    async fn puppet_get_by_discord_id(&self, discord_id: String) -> Result<Option<User>> {
        let row = query!(
            "SELECT lamprey_id, discord_id, discord_avatar_url, discord_banner_url FROM \"user\" WHERE discord_id = ?",
            discord_id
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| User {
            lamprey_id: r.lamprey_id.parse().unwrap(),
            discord_id: r.discord_id.parse().unwrap(),
            discord_avatar_url: r.discord_avatar_url,
            discord_banner_url: r.discord_banner_url,
        }))
    }

    async fn puppet_delete(&self, lamprey_id: String) -> Result<()> {
        query!("DELETE FROM \"user\" WHERE lamprey_id = ?", lamprey_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
