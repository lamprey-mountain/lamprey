use std::time::Duration;

use common::v1::types::audit_logs::AuditLogEntryType;
use common::v1::types::emoji::{EmojiCustom, EmojiCustomCreate, EmojiCustomPatch, EmojiOwner};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::sync::MessageSync;
use common::v1::types::util::Changes;
use common::v1::types::{EmojiId, PaginationQuery, PaginationResponse, Permission, RoomId};
use moka::future::Cache;
use validator::Validate;

use crate::error::Result;
use crate::globals::messaging::BroadcastSync;
use crate::prelude::*;
use crate::routes::util::Auth;

pub struct ServiceEmoji {
    state: Globals,
    idempotency_keys: Cache<String, EmojiCustom>,
}

impl ServiceEmoji {
    pub fn new(state: Globals) -> Self {
        Self {
            state,
            idempotency_keys: Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .build(),
        }
    }

    pub async fn create(
        &self,
        room_id: RoomId,
        auth: &Auth,
        json: EmojiCustomCreate,
        nonce: Option<String>,
    ) -> Result<EmojiCustom> {
        if let Some(n) = &nonce {
            self.idempotency_keys
                .try_get_with(
                    n.clone(),
                    self.create_inner(room_id, auth, json, nonce.clone()),
                )
                .await
                .map_err(|err| err.fake_clone())
        } else {
            self.create_inner(room_id, auth, json, nonce).await
        }
    }

    async fn create_inner(
        &self,
        room_id: RoomId,
        auth: &Auth,
        json: EmojiCustomCreate,
        nonce: Option<String>,
    ) -> Result<EmojiCustom> {
        json.validate()?;
        let mut data = self.state.begin().await?;
        let srv = self.state.services();

        let perms = srv.perms.for_room(auth.user.id, room_id).await?;
        perms.ensure(Permission::EmojiManage)?;

        let media = data.media_select(json.media_id).await?;
        if !media.metadata.is_image() {
            return Err(ApiError::from_code(ErrorCode::MediaNotAnImage).into());
        }

        let emoji = data
            .emoji_create(auth.user.id, room_id, json.clone())
            .await?;

        let changes = Changes::new()
            .add("name", &json.name)
            .add("animated", &json.animated)
            .add("media_id", &json.media_id);

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::EmojiCreate {
            changes: changes.build(),
        })
        .await?;

        data.commit().await?;

        let sync_msg = MessageSync::EmojiCreate {
            emoji: emoji.clone(),
        };

        let mut broadcast = BroadcastSync::sync(sync_msg);
        if let Some(n) = nonce {
            broadcast = broadcast.with_nonce(n);
        }

        self.state
            .messaging()
            .broadcast_room(room_id, broadcast)
            .await?;

        Ok(emoji)
    }

    pub async fn get(&self, emoji_id: EmojiId) -> Result<EmojiCustom> {
        self.state.begin_read().await?.emoji_get(emoji_id).await
    }

    pub async fn update(
        &self,
        room_id: RoomId,
        emoji_id: EmojiId,
        auth: &Auth,
        patch: EmojiCustomPatch,
    ) -> Result<EmojiCustom> {
        let mut data = self.state.begin().await?;
        let srv = self.state.services();

        let perms = srv.perms.for_room(auth.user.id, room_id).await?;
        perms.ensure(Permission::EmojiManage)?;

        let emoji_before = data.emoji_get(emoji_id).await?;
        data.emoji_update(emoji_id, patch).await?;
        let emoji = data.emoji_get(emoji_id).await?;

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::EmojiUpdate {
            changes: Changes::new()
                .change("name", &emoji_before.name, &emoji.name)
                .build(),
        })
        .await?;

        data.commit().await?;

        if let Some(EmojiOwner::Room { room_id }) = emoji.owner {
            let sync_msg = MessageSync::EmojiUpdate {
                emoji: emoji.clone(),
            };
            self.state
                .messaging()
                .broadcast_room(room_id, sync_msg)
                .await?;
        }

        Ok(emoji)
    }

    pub async fn delete(&self, room_id: RoomId, emoji_id: EmojiId, auth: &Auth) -> Result<()> {
        let mut data = self.state.begin().await?;
        let emoji = data.emoji_get(emoji_id).await?;

        let perms = self
            .state
            .services()
            .perms
            .for_room(auth.user.id, room_id)
            .await?;
        perms.ensure(Permission::EmojiManage)?;

        data.emoji_delete(emoji_id).await?;

        let al = auth.audit_log(room_id);
        al.commit_success(AuditLogEntryType::EmojiDelete {
            emoji_id,
            changes: Changes::new()
                .remove("name", &emoji.name)
                .remove("animated", &emoji.animated)
                .remove("media_id", &emoji.media_id)
                .build(),
        })
        .await?;

        data.commit().await?;

        if let Some(EmojiOwner::Room { room_id }) = emoji.owner {
            let sync_msg = MessageSync::EmojiDelete {
                emoji_id: emoji.id,
                room_id,
            };
            self.state
                .messaging()
                .broadcast_room(room_id, sync_msg)
                .await?;
        }

        Ok(())
    }

    pub async fn list(
        &self,
        room_id: RoomId,
        auth: &Auth,
        pagination: PaginationQuery<EmojiId>,
    ) -> Result<PaginationResponse<EmojiCustom>> {
        let _perms = self
            .state
            .services()
            .perms
            .for_room(auth.user.id, room_id)
            .await?;

        self.state
            .begin_read()
            .await?
            .emoji_list(room_id, pagination)
            .await
    }

    pub async fn search(
        &self,
        auth: &Auth,
        query: String,
        pagination: PaginationQuery<EmojiId>,
    ) -> Result<PaginationResponse<EmojiCustom>> {
        self.state
            .begin_read()
            .await?
            .emoji_search(auth.user.id, query, pagination)
            .await
    }

    pub async fn lookup(&self, emoji_id: EmojiId, auth: &Auth) -> Result<EmojiCustom> {
        let mut data = self.state.begin_read().await?;
        let mut emoji = data.emoji_get(emoji_id).await?;

        let original_owner = emoji.owner.clone();
        let original_creator_id = emoji.creator_id;

        emoji.creator_id = None;
        emoji.owner = None;

        match original_owner {
            Some(EmojiOwner::Room { room_id }) => {
                if data.room_member_get(room_id, auth.user.id).await.is_ok() {
                    emoji.owner = original_owner;
                    emoji.creator_id = original_creator_id;
                }
            }
            Some(EmojiOwner::User) => {
                if original_creator_id == Some(auth.user.id) {
                    emoji.owner = original_owner;
                    emoji.creator_id = original_creator_id;
                }
            }
            None => {}
        }

        Ok(emoji)
    }
}
