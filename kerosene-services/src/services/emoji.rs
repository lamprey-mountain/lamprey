use std::time::Duration;

use common::v1::types::audit_logs::AuditLogEntryType;
use common::v1::types::emoji::{EmojiCustom, EmojiCustomCreate, EmojiCustomPatch, EmojiOwner};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::sync::MessageSync;
use common::v1::types::util::Changes;
use common::v1::types::{EmojiId, PaginationQuery, PaginationResponse, Permission, RoomId};
use kerosene_core::types::auth::{Auth5, Auth5Ext};
use moka::future::Cache;
use validator::Validate;

use crate::globals::messaging::Broadcast;
use crate::prelude::*;

pub struct ServiceEmoji {
    globals: Globals,
    idempotency_keys: Cache<String, EmojiCustom>,
    // TODO: make this not pub
    pub(crate) cache: Cache<EmojiId, Arc<EmojiCustom>>,
}

impl ServiceEmoji {
    pub fn new(globals: Globals) -> Self {
        Self {
            globals,
            idempotency_keys: Cache::builder()
                .time_to_live(Duration::from_secs(300))
                .build(),
            cache: Cache::builder().max_capacity(100_000).build(),
        }
    }

    pub async fn create<A: Auth5>(
        &self,
        room_id: RoomId,
        auth: &mut A,
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

    async fn create_inner<A: Auth5>(
        &self,
        room_id: RoomId,
        auth: &mut A,
        json: EmojiCustomCreate,
        nonce: Option<String>,
    ) -> Result<EmojiCustom> {
        json.validate()?;
        let mut data = self.globals.begin().await?;
        let srv = self.globals.services();

        let user = auth.ensure_user()?;
        let user_id = user.id;
        let perms = srv.perms.for_room(user_id, room_id).await?;
        perms.ensure(Permission::EmojiManage)?;

        let media = data.media_select(json.media_id).await?;
        if !media.metadata.is_image() {
            return Err(ApiError::from_code(ErrorCode::MediaNotAnImage).into());
        }

        let emoji = data.emoji_create(user_id, room_id, json.clone()).await?;

        let changes = Changes::new()
            .add("name", &json.name)
            .add("animated", &json.animated)
            .add("media_id", &json.media_id);

        auth.set_room_id(room_id);
        auth.al_push(AuditLogEntryType::EmojiCreate {
            changes: changes.build(),
        });

        data.commit().await?;

        let sync_msg = MessageSync::EmojiCreate {
            emoji: emoji.clone(),
        };

        self.cache.insert(emoji.id, Arc::new(emoji.clone())).await;

        let mut broadcast = Broadcast::sync(sync_msg);
        if let Some(n) = nonce {
            broadcast = broadcast.with_nonce(n);
        }

        self.globals
            .messaging()
            .broadcast_room(room_id, broadcast)
            .await?;

        Ok(emoji)
    }

    pub async fn get(&self, emoji_id: EmojiId) -> Result<EmojiCustom> {
        let emoji = self
            .cache
            .try_get_with(emoji_id, async {
                let emoji = self.globals.begin_read().await?.emoji_get(emoji_id).await?;
                Result::<Arc<EmojiCustom>>::Ok(Arc::new(emoji))
            })
            .await
            .map_err(|e| e.fake_clone())?;
        // PERF: don't clone
        Ok((*emoji).clone())
    }

    pub async fn get_many(&self, emoji_ids: &[EmojiId]) -> Result<Vec<EmojiCustom>> {
        if emoji_ids.is_empty() {
            return Ok(vec![]);
        }

        let mut out = Vec::with_capacity(emoji_ids.len());
        let mut missing = Vec::new();

        for id in emoji_ids {
            if let Some(emoji) = self.cache.get(id).await {
                out.push((*emoji).clone());
            } else {
                missing.push(*id);
            }
        }

        if !missing.is_empty() {
            let emojis = self
                .globals
                .begin_read()
                .await?
                .emoji_get_many(&missing)
                .await?;
            for emoji in emojis {
                self.cache.insert(emoji.id, Arc::new(emoji.clone())).await;
                out.push(emoji);
            }
        }

        Ok(out)
    }

    pub async fn invalidate(&self, emoji_id: EmojiId) {
        self.cache.invalidate(&emoji_id).await
    }

    pub fn purge_cache(&self) {
        self.cache.invalidate_all();
    }

    pub async fn update<A: Auth5>(
        &self,
        room_id: RoomId,
        emoji_id: EmojiId,
        auth: &mut A,
        patch: EmojiCustomPatch,
    ) -> Result<EmojiCustom> {
        let mut data = self.globals.begin().await?;
        let srv = self.globals.services();

        let user = auth.ensure_user()?;
        let user_id = user.id;
        let perms = srv.perms.for_room(user_id, room_id).await?;
        perms.ensure(Permission::EmojiManage)?;

        let emoji_before = data.emoji_get(emoji_id).await?;
        data.emoji_update(emoji_id, patch).await?;
        let emoji = data.emoji_get(emoji_id).await?;

        auth.set_room_id(room_id);
        auth.al_push(AuditLogEntryType::EmojiUpdate {
            changes: Changes::new()
                .change("name", &emoji_before.name, &emoji.name)
                .build(),
        });

        data.commit().await?;

        self.cache.insert(emoji.id, Arc::new(emoji.clone())).await;

        if let Some(EmojiOwner::Room { room_id }) = emoji.owner {
            let sync_msg = MessageSync::EmojiUpdate {
                emoji: emoji.clone(),
            };
            self.globals
                .messaging()
                .broadcast_room(room_id, sync_msg)
                .await?;
        }

        Ok(emoji)
    }

    pub async fn delete<A: Auth5>(
        &self,
        room_id: RoomId,
        emoji_id: EmojiId,
        auth: &mut A,
    ) -> Result<()> {
        let mut data = self.globals.begin().await?;
        let emoji = data.emoji_get(emoji_id).await?;

        let user = auth.ensure_user()?;
        let user_id = user.id;
        let perms = self
            .globals
            .services()
            .perms
            .for_room(user_id, room_id)
            .await?;
        perms.ensure(Permission::EmojiManage)?;

        data.emoji_delete(emoji_id).await?;

        auth.set_room_id(room_id);
        auth.al_push(AuditLogEntryType::EmojiDelete {
            emoji_id,
            changes: Changes::new()
                .remove("name", &emoji.name)
                .remove("animated", &emoji.animated)
                .remove("media_id", &emoji.media_id)
                .build(),
        });

        data.commit().await?;

        self.cache.invalidate(&emoji_id).await;

        if let Some(EmojiOwner::Room { room_id }) = emoji.owner {
            let sync_msg = MessageSync::EmojiDelete {
                emoji_id: emoji.id,
                room_id,
            };
            self.globals
                .messaging()
                .broadcast_room(room_id, sync_msg)
                .await?;
        }

        Ok(())
    }

    pub async fn list<A: Auth5>(
        &self,
        room_id: RoomId,
        auth: &A,
        pagination: PaginationQuery<EmojiId>,
    ) -> Result<PaginationResponse<EmojiCustom>> {
        let user = auth.ensure_user()?;
        let user_id = user.id;
        let _perms = self
            .globals
            .services()
            .perms
            .for_room(user_id, room_id)
            .await?;

        self.globals
            .begin_read()
            .await?
            .emoji_list(room_id, pagination)
            .await
    }

    pub async fn search<A: Auth5>(
        &self,
        auth: &A,
        query: String,
        pagination: PaginationQuery<EmojiId>,
    ) -> Result<PaginationResponse<EmojiCustom>> {
        let user = auth.ensure_user()?;
        let user_id = user.id;
        self.globals
            .begin_read()
            .await?
            .emoji_search(user_id, query, pagination)
            .await
    }

    pub async fn lookup<A: Auth5>(&self, emoji_id: EmojiId, auth: &A) -> Result<EmojiCustom> {
        let mut data = self.globals.begin_read().await?;
        let mut emoji = data.emoji_get(emoji_id).await?;

        let user = auth.ensure_user()?;
        let user_id = user.id;
        let original_owner = emoji.owner.clone();
        let original_creator_id = emoji.creator_id;

        emoji.creator_id = None;
        emoji.owner = None;

        match original_owner {
            Some(EmojiOwner::Room { room_id }) => {
                if data.room_member_get(room_id, user_id).await.is_ok() {
                    emoji.owner = original_owner;
                    emoji.creator_id = original_creator_id;
                }
            }
            Some(EmojiOwner::User) => {
                if original_creator_id == Some(user_id) {
                    emoji.owner = original_owner;
                    emoji.creator_id = original_creator_id;
                }
            }
            None => {}
        }

        Ok(emoji)
    }
}
