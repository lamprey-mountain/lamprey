use std::collections::{BTreeMap, BTreeSet, HashMap};

use async_tempfile::TempFile;
use common::v1::types::MediaId;
use common::v1::types::MessageSync;
use common::v1::types::{
    AuditLogEntry, AuditLogFilter, Channel, ChannelId, HarvestId, MessageId, PaginationDirection,
    PaginationKey, PaginationQuery, PaginationResponse, Permission, Relationship, Role, Room,
    RoomBan, RoomId, RoomMember, User, UserId,
    application::Application,
    emoji::EmojiCustom,
    harvest::{
        Harvest, HarvestCreateRoom, HarvestCreateUser, HarvestFailedCode, HarvestStatus,
        HarvestType,
        archive::{
            ARCHIVE_API_VERSION, ARCHIVE_VERSION, HarvestInfo, Metadata, ReactionArchive, RoomFile,
        },
    },
    message::MessageVersion,
    reaction::{ReactionKeyParam, ReactionListItem},
    server::{ServerFeatures, ServerInfo, ServerMedia, ServerVersion},
    util::Time,
};
use lamprey_backend_data_postgres::data::AnyData;
use serde::Serialize;
use serde_json::json;
use tracing::{error, warn};

use crate::prelude::*;
use crate::services::harvest::EXPIRES_AFTER;
use crate::services::harvest::archive::{Archive, Ndjson};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD as B64;
use bytes::BytesMut;
use futures_util::future::BoxFuture;
use sha2::Digest as _;
use tokio::io::AsyncReadExt;

const IMMUTABLE: &str = "public, max-age=604800, immutable, stale-while-revalidate=86400";
const ARCHIVE_CONTENT_TYPE: &str = "application/zip";

/// size of the chunk used to read and upload an archive
const IO_BUF_SIZE: usize = 1024 * 1024;

/// page size for db pagination
const PAGE_SIZE: u16 = 500;

/// the information in `readme.txt`
const README: &str = include_str!("./embedded-readme.txt");

fn page<K: PaginationKey>(from: Option<K>) -> PaginationQuery<K> {
    PaginationQuery {
        from,
        to: None,
        dir: Some(PaginationDirection::F),
        limit: Some(PAGE_SIZE),
    }
}

impl super::ServiceHarvest {
    /// generate a harvest archive
    ///
    /// 1. update status to archiving
    /// 2. write the archive to a temporary zip file
    /// 3. upload it to blob storage
    /// 4. update status to completed
    /// 5. broadcast harvest update
    ///
    /// on any failure, update status to failed
    pub(super) async fn generate(&self, mut harvest: Harvest) -> Result<()> {
        let started_at = Time::now_utc();

        harvest.status = HarvestStatus::Archiving {
            started_at,
            eta_at: None,
        };
        put_harvest(&self.globals, &harvest).await?;

        match run(&self.globals, &harvest, started_at).await {
            Ok(completed) => harvest = completed,
            Err(e) => {
                error!("failed to generate harvest {}: {e}", harvest.id);

                harvest.status = HarvestStatus::Failed {
                    failed_at: Time::now_utc(),
                    code: HarvestFailedCode::Other,
                    message: "failed to generate archive".to_string(),
                };
                put_harvest(&self.globals, &harvest).await?;
            }
        }

        self.globals
            .messaging()
            .broadcast_user(
                harvest.requester_id,
                MessageSync::HarvestUpdate {
                    harvest: Box::new(harvest.clone()),
                },
            )
            .await?;

        Ok(())
    }
}

/// write the archive, upload it, and return the harvest with its completed status
async fn run(globals: &Globals, harvest: &Harvest, started_at: Time) -> Result<Harvest> {
    let mut archive = Archive::create().await?;
    // TODO: dynamically generate readme based on harvest config
    archive.write_text("readme.txt", README).await?;

    let mut extract = Extract::new(globals, &mut archive);

    match &harvest.ty {
        HarvestType::User {
            target_user_id,
            create,
        } => extract.user(*target_user_id, create).await?,
        HarvestType::Room {
            target_room_id,
            create,
        } => {
            extract
                .room(*target_room_id, create, harvest.requester_id)
                .await?
        }
    }

    extract.finish().await?;

    let completed_at = Time::now_utc();
    let metadata = Metadata {
        archive_version: ARCHIVE_VERSION,
        api_version: ARCHIVE_API_VERSION,
        harvest: HarvestInfo {
            id: harvest.id,
            requester_id: harvest.requester_id,
            queued_at: harvest.queued_at,
            started_at,
            completed_at,
            ty: harvest.ty.clone(),
        },
        user: extract.user.take(),
        room: extract.room.take(),
        applications: std::mem::take(&mut extract.applications),
        server: server_info(globals)?,
    };
    archive.write("harvest.json", &metadata).await?;

    let file = archive.finish().await?;
    let url = archive_url(globals, harvest.id)?;
    let size = tokio::fs::metadata(file.file_path()).await?.len();
    let hashes = hash_file(&file).await?;

    upload(globals, &url, &file).await?;

    let mut updated = harvest.clone();
    updated.status = HarvestStatus::Completed {
        started_at,
        completed_at,
        expires_at: completed_at + EXPIRES_AFTER,
        url,
        size,
        hashes,
    };
    put_harvest(globals, &updated).await?;

    Ok(updated)
}

/// writes one harvest's worth of data into an [`Archive`]
struct Extract<'a> {
    globals: &'a Globals,
    archive: &'a mut Archive,

    /// media referenced by the exported messages
    media: BTreeSet<MediaId>,
    /// users referenced by the archive, and the requester's relationship with them
    users: BTreeMap<UserId, Option<Relationship>>,

    /// for `harvest.json`
    user: Option<User>,
    room: Option<Room>,
    applications: Vec<Application>,

    /// audit log entries of the exported user (user harvests)
    audit: Vec<AuditLogEntry>,

    /// user harvests: lines are buffered per channel, because every channel
    /// directory can only be written once
    // PERF: maybe it would be better to write to plain files, then create the zip later?
    buffered_messages: BTreeMap<ChannelId, Vec<u8>>,
    buffered_versions: BTreeMap<ChannelId, Vec<u8>>,
    buffered_reactions: BTreeMap<ChannelId, Vec<u8>>,
}

impl<'a> Extract<'a> {
    fn new(globals: &'a Globals, archive: &'a mut Archive) -> Self {
        Self {
            globals,
            archive,
            media: BTreeSet::new(),
            users: BTreeMap::new(),
            user: None,
            room: None,
            applications: Vec::new(),
            audit: Vec::new(),
            buffered_messages: BTreeMap::new(),
            buffered_versions: BTreeMap::new(),
            buffered_reactions: BTreeMap::new(),
        }
    }

    /// write everything that is derived from the extracted data
    async fn finish(&mut self) -> Result<()> {
        for (channel_id, buf) in std::mem::take(&mut self.buffered_messages) {
            write_buffer(
                self.archive,
                &format!("channels/{channel_id}/messages.ndjson"),
                buf,
            )
            .await?;
        }
        for (channel_id, buf) in std::mem::take(&mut self.buffered_versions) {
            write_buffer(
                self.archive,
                &format!("channels/{channel_id}/versions.ndjson"),
                buf,
            )
            .await?;
        }
        for (channel_id, buf) in std::mem::take(&mut self.buffered_reactions) {
            write_buffer(
                self.archive,
                &format!("channels/{channel_id}/reactions.ndjson"),
                buf,
            )
            .await?;
        }

        let mut out = self.archive.ndjson("users.ndjson").await?;
        for (user_id, relationship) in std::mem::take(&mut self.users) {
            let mut db = self.globals.begin_read().await?;
            let user = match db.user_get(user_id).await {
                Ok(user) => user,
                Err(e) => {
                    warn!("skipping missing user {user_id}: {e:?}");
                    continue;
                }
            };

            out.line(&json!({
                "user": user,
                "relationship": relationship.unwrap_or(Relationship {
                    relation: None,
                    ignore: None,
                }),
            }))
            .await?;
        }
        out.finish().await?;

        let mut out = self.archive.ndjson("media.ndjson").await?;
        let mut db = self.globals.begin_read().await?;
        for media_id in std::mem::take(&mut self.media) {
            match db.media_select(media_id).await {
                Ok(media) => out.line(&media).await?,
                Err(e) => warn!("skipping missing media {media_id}: {e:?}"),
            }
        }
        drop(db);
        out.finish().await?;

        if !self.audit.is_empty() {
            let mut out = self.archive.ndjson("audit-log.ndjson").await?;
            for entry in std::mem::take(&mut self.audit) {
                out.line(&entry).await?;
            }
            out.finish().await?;
        }

        Ok(())
    }

    /// extract all data for a user harvest
    async fn user(&mut self, user_id: UserId, create: &HarvestCreateUser) -> Result<()> {
        // TODO: try using services instead of db for everything

        let mut db = self.globals.begin_read().await?;
        let user = db.user_get(user_id).await?;
        self.user = Some(user);

        // relationships
        for kind in ["friends", "blocked", "pending"] {
            let mut from = None;
            loop {
                let res = match kind {
                    "friends" => {
                        page_of(
                            self.globals,
                            move |db, q| db.user_relationship_list_friends(user_id, q),
                            from,
                        )
                        .await?
                    }
                    "blocked" => {
                        page_of(
                            self.globals,
                            move |db, q| db.user_relationship_list_blocked(user_id, q),
                            from,
                        )
                        .await?
                    }
                    _ => {
                        page_of(
                            self.globals,
                            move |db, q| db.user_relationship_list_pending(user_id, q),
                            from,
                        )
                        .await?
                    }
                };

                for relationship in &res.items {
                    self.users
                        .insert(relationship.user_id, Some(relationship.inner.clone()));
                }

                if !res.has_more || res.items.is_empty() {
                    break;
                }
                from = res.items.last().map(|r| r.user_id);
            }
        }

        // applications
        let mut from = None;
        loop {
            let res = page_of(
                self.globals,
                move |db, q| db.application_list(user_id, q),
                from,
            )
            .await?;
            self.applications.extend(res.items.iter().cloned());

            if !res.has_more || res.items.is_empty() {
                break;
            }
            from = res.items.last().map(|a| a.id);
        }

        // all rooms the user is a member of
        let room_ids = db.room_list_user_all(user_id).await?;
        drop(db);

        let mut room_prefs = {
            let mut db = self.globals.begin_read().await?;
            db.preferences_room_get_many(user_id, &room_ids).await?
        };

        let mut channels = Vec::new();
        for room_id in &room_ids {
            let mut db = self.globals.begin_read().await?;
            let mut room = db.room_get(*room_id).await?;
            room.preferences = room_prefs.remove(room_id);

            let roles = db.role_list(*room_id).await?;
            let room_channels = db.channel_list(*room_id).await?;
            let member = db
                .room_member_get_many_for_user(user_id, &[*room_id])
                .await?;
            drop(db);

            self.room_file(*room_id, &room, &roles, &room_channels, &member, &[])
                .await?;
            channels.extend(room_channels);
        }

        let channel_ids: Vec<ChannelId> = channels.iter().map(|c| c.id).collect();
        let mut channel_prefs = {
            let mut db = self.globals.begin_read().await?;
            db.preferences_channel_get_many(user_id, &channel_ids)
                .await?
        };

        for mut channel in channels {
            channel.preferences = channel_prefs.remove(&channel.id);
            self.channel_file(&channel).await?;
        }

        // messages this user has sent
        if create.include_messages {
            let mut from = None;
            loop {
                let res = page_of(
                    self.globals,
                    move |db, q| db.message_list_by_author(user_id, q),
                    from,
                )
                .await?;

                for message in &res.items {
                    self.users.insert(message.author_id, None);
                    let data = normalize(message, &mut self.media)?;
                    self.buffer_message(message.channel_id, &data)?;
                }

                if !res.has_more || res.items.is_empty() {
                    break;
                }
                from = res.items.last().map(|m| m.id);
            }
        }

        // audit log entries this user caused, across all their rooms and theirself
        let filter = AuditLogFilter {
            user_id: vec![user_id],
            ..Default::default()
        };
        for room_id in room_ids.iter().copied().chain([RoomId::from(*user_id)]) {
            let mut from = None;
            loop {
                let filter = filter.clone();
                let res = page_of(
                    self.globals,
                    move |db, q| db.audit_logs_room_fetch(room_id, q, filter),
                    from,
                )
                .await?;
                self.audit.extend(res.items.iter().cloned());

                if !res.has_more || res.items.is_empty() {
                    break;
                }
                from = res.items.last().map(|e| e.id);
            }
        }

        Ok(())
    }

    /// extract all data for a room harvest
    async fn room(
        &mut self,
        room_id: RoomId,
        create: &HarvestCreateRoom,
        requester_id: UserId,
    ) -> Result<()> {
        let mut db = self.globals.begin_read().await?;
        let room = db.room_get(room_id).await?;
        let roles = db.role_list(room_id).await?;

        // removed channels are included too, flagged by their `removed_at`
        let mut channels = db.channel_list(room_id).await?;
        let mut from = None;
        loop {
            let res = page_of(
                self.globals,
                move |db, q| db.channel_list_removed(room_id, q, None),
                from,
            )
            .await?;
            channels.extend(res.items.iter().cloned());

            if !res.has_more || res.items.is_empty() {
                break;
            }
            from = res.items.last().map(|c| c.id);
        }

        let members = if create.include_members {
            db.room_member_list_all(room_id).await?
        } else {
            Vec::new()
        };
        drop(db);

        for member in &members {
            self.users.insert(member.user_id, None);
        }

        // bans, only if the requester is allowed to see them
        let perms = self
            .globals
            .services()
            .perms
            .for_room(requester_id, room_id)
            .await?;
        let mut bans = Vec::new();
        if perms.has(Permission::MemberBan) || perms.has(Permission::AuditLogView) {
            let mut from = None;
            loop {
                let res = page_of(
                    self.globals,
                    move |db, q| db.room_ban_list(room_id, q),
                    from,
                )
                .await?;
                bans.extend(res.items.iter().cloned());

                if !res.has_more || res.items.is_empty() {
                    break;
                }
                from = res.items.last().map(|b: &RoomBan| b.user_id);
            }
            for ban in &bans {
                self.users.insert(ban.user_id, None);
            }
        }

        self.room_file(room_id, &room, &roles, &channels, &members, &bans)
            .await?;

        // the audit log of this room
        let mut out = self
            .archive
            .ndjson(&format!("rooms/{room_id}/audit-log.ndjson"))
            .await?;
        let mut from = None;
        let filter = if perms.has(Permission::AuditLogView) {
            AuditLogFilter::default()
        } else {
            AuditLogFilter {
                user_id: vec![requester_id],
                ..Default::default()
            }
        };
        loop {
            let filter = filter.clone();
            let res = page_of(
                self.globals,
                move |db, q| db.audit_logs_room_fetch(room_id, q, filter),
                from,
            )
            .await?;
            for entry in &res.items {
                out.line(entry).await?;
            }

            if !res.has_more || res.items.is_empty() {
                break;
            }
            from = res.items.last().map(|e| e.id);
        }
        out.finish().await?;

        // FIXME: don't include channels the requester can't see
        for channel in &channels {
            self.channel_file(channel).await?;

            if create.include_messages {
                self.channel_messages(room_id, channel.id, create.include_reactions)
                    .await?;
            } else if create.include_reactions {
                self.channel_reactions(room_id, channel.id, &[]).await?;
            }
        }

        Ok(())
    }

    /// write `rooms/<room_id>/room.json`, `emojis.ndjson` and `members.ndjson`
    async fn room_file(
        &mut self,
        room_id: RoomId,
        room: &Room,
        roles: &[Role],
        channels: &[Channel],
        members: &[RoomMember],
        bans: &[RoomBan],
    ) -> Result<()> {
        self.archive
            .write(
                &format!("rooms/{room_id}/room.json"),
                &RoomFile {
                    room: room.clone(),
                    roles: roles.to_vec(),
                    channels: channels.to_vec(),
                    bans: bans.to_vec(),
                },
            )
            .await?;

        let mut out = self
            .archive
            .ndjson(&format!("rooms/{room_id}/emojis.ndjson"))
            .await?;
        let mut from = None;
        loop {
            let res = page_of(self.globals, move |db, q| db.emoji_list(room_id, q), from).await?;
            for emoji in &res.items {
                out.line(&emoji).await?;
            }

            if !res.has_more || res.items.is_empty() {
                break;
            }
            from = res.items.last().map(|e: &EmojiCustom| e.id);
        }
        out.finish().await?;

        if !members.is_empty() {
            let mut out = self
                .archive
                .ndjson(&format!("rooms/{room_id}/members.ndjson"))
                .await?;
            for member in members {
                out.line(member).await?;
            }
            out.finish().await?;
        }

        Ok(())
    }

    /// write `channels/<channel_id>/channel.json`
    async fn channel_file(&mut self, channel: &Channel) -> Result<()> {
        self.archive
            .write(&format!("channels/{}/channel.json", channel.id), channel)
            .await?;

        Ok(())
    }

    /// write messages, versions and reactions of a channel
    async fn channel_messages(
        &mut self,
        room_id: RoomId,
        channel_id: ChannelId,
        include_reactions: bool,
    ) -> Result<()> {
        // TODO: maybe remove latest_version? or make it an id only? to deduplicate message version data.

        let mut message_ids = Vec::new();

        let mut out = self
            .archive
            .ndjson(&format!("channels/{channel_id}/messages.ndjson"))
            .await?;
        let mut from = None;
        loop {
            let res = page_of(
                self.globals,
                move |db, q| db.message_list_all(channel_id, q),
                from,
            )
            .await?;

            for message in &res.items {
                self.users.insert(message.author_id, None);
                message_ids.push(message.id);
                out.line(&normalize(message, &mut self.media)?).await?;
            }

            if !res.has_more || res.items.is_empty() {
                break;
            }
            from = res.items.last().map(|m| m.id);
        }
        out.finish().await?;

        // every version of the exported messages
        let mut out = self
            .archive
            .ndjson(&format!("channels/{channel_id}/versions.ndjson"))
            .await?;
        for message_id in &message_ids {
            let mut from = None;
            loop {
                // PERF: this is horrible
                let res = page_of(
                    self.globals,
                    move |db, q| db.message_version_list(channel_id, *message_id, q),
                    from,
                )
                .await?;

                for version in &res.items {
                    if let Some(author_id) = version.author_id {
                        self.users.insert(author_id, None);
                    }
                    out.line(&normalize(version, &mut self.media)?).await?;
                }

                if !res.has_more || res.items.is_empty() {
                    break;
                }
                from = res.items.last().map(|v: &MessageVersion| v.version_id);
            }
        }
        out.finish().await?;

        if include_reactions {
            self.channel_reactions(room_id, channel_id, &message_ids)
                .await?;
        }

        Ok(())
    }

    /// write reactions of a channel
    async fn channel_reactions(
        &mut self,
        _room_id: RoomId,
        channel_id: ChannelId,
        message_ids: &[MessageId],
    ) -> Result<()> {
        let mut out = self
            .archive
            .ndjson(&format!("channels/{channel_id}/reactions.ndjson"))
            .await?;

        for message_id in message_ids {
            let mut db = self.globals.begin_read().await?;
            let message = match db.message_get(channel_id, *message_id).await {
                Ok(message) => message,
                Err(e) => {
                    warn!("skipping reactions of missing message {message_id}: {e:?}");
                    continue;
                }
            };
            let keys: Vec<ReactionKeyParam> = message
                .reactions
                .0
                .iter()
                .map(|c| ReactionKeyParam::from(c.key.clone()))
                .collect();
            drop(db);

            for key in keys {
                let mut from = None;
                loop {
                    let reaction_key = key.clone();
                    let res = page_of(
                        self.globals,
                        move |db, q| {
                            db.reaction_list(channel_id, *message_id, reaction_key.clone(), q)
                        },
                        from,
                    )
                    .await?;

                    for item in &res.items {
                        self.users.insert(item.user_id, None);
                        out.line(&ReactionArchive {
                            message_id: *message_id,
                            key: key.clone(),
                            user_id: item.user_id,
                            created_at: item.created_at,
                        })
                        .await?;
                    }

                    if !res.has_more || res.items.is_empty() {
                        break;
                    }
                    from = res.items.last().map(|r: &ReactionListItem| r.user_id);
                }
            }
        }

        out.finish().await?;

        Ok(())
    }

    /// buffer a message for a channel (user harvests)
    fn buffer_message(&mut self, channel_id: ChannelId, data: &serde_json::Value) -> Result<()> {
        push_line(self.buffered_messages.entry(channel_id).or_default(), data)
    }
}

/// write a buffered ndjson file
async fn write_buffer(archive: &mut Archive, path: &str, buf: Vec<u8>) -> Result<()> {
    let mut out: Ndjson<'_> = archive.ndjson(path).await?;
    out.write_bytes(&buf).await?;
    out.finish().await
}

/// serialize one value as an ndjson line
fn push_line<T: Serialize + ?Sized>(buf: &mut Vec<u8>, value: &T) -> Result<()> {
    serde_json::to_writer(&mut *buf, value)?;
    buf.push(b'\n');

    Ok(())
}

/// fetch one page of a paginated query with a fresh read transaction
async fn page_of<T, K, Q>(
    globals: &Globals,
    query: Q,
    from: Option<K>,
) -> Result<PaginationResponse<T>>
where
    K: PaginationKey + Copy,
    Q: for<'q> FnOnce(
        &'q mut AnyData,
        PaginationQuery<K>,
    ) -> BoxFuture<'q, Result<PaginationResponse<T>>>,
{
    let mut db = globals.begin_read().await?;
    query(&mut db, page(from)).await
}

/// serialize a message or version, replacing embedded media objects by their id
fn normalize<T: Serialize>(value: &T, media: &mut BTreeSet<MediaId>) -> Result<serde_json::Value> {
    let mut value = serde_json::to_value(value)?;
    walk_media(&mut value, media);

    Ok(value)
}

/// replace every `attachments[].media` object with `{"id": ...}`
fn walk_media(value: &mut serde_json::Value, media: &mut BTreeSet<MediaId>) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Array(attachments)) = map.get_mut("attachments") {
                for attachment in attachments {
                    let Some(serde_json::Value::Object(media_obj)) = attachment.get_mut("media")
                    else {
                        continue;
                    };
                    let Some(id) = media_obj.get("id").and_then(|id| id.as_str()) else {
                        continue;
                    };
                    if let Ok(media_id) = serde_json::from_value::<MediaId>(json!(id)) {
                        media.insert(media_id);
                        *media_obj = serde_json::Map::from_iter([(
                            "id".to_string(),
                            serde_json::Value::String(id.to_string()),
                        )]);
                    }
                }
            }
            for (_, value) in map.iter_mut() {
                walk_media(value, media);
            }
        }
        serde_json::Value::Array(values) => {
            for value in values {
                walk_media(value, media);
            }
        }
        _ => {}
    }
}

/// the public info about this server, as stored in `harvest.json`
fn server_info(globals: &Globals) -> Result<ServerInfo> {
    let config = globals.config();

    // TODO: deduplicate this logic with crate-backend/src/routes/server.rs
    Ok(ServerInfo {
        api_url: config.api_url.clone(),
        sync_url: config.api_url.join("/api/v1/sync")?,
        html_url: config.html_url.clone(),
        cdn_url: config.cdn_url.clone(),
        features: ServerFeatures {
            registration: None,
            auth: None,
            media: Some(ServerMedia {
                max_file_size: config.media.max_size,
            }),
            voice: None,
            web_push: None,
            webtransport: None,
            redex: None,
        },
        version: ServerVersion {
            implementation: "kerosene".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            extra: HashMap::new(),
        },
    })
}

async fn put_harvest(globals: &Globals, harvest: &Harvest) -> Result<()> {
    let mut txn = globals.begin().await?;
    txn.harvest_put(harvest).await?;
    txn.commit().await
}

/// url the archive will be uploaded to, including its random access token
fn archive_url(globals: &Globals, harvest_id: HarvestId) -> Result<url::Url> {
    let token = generate_token();
    let date = Time::now_utc()
        .date()
        .format(&time::format_description::well_known::Iso8601::DATE)
        .map_err(|e| Error::Internal(e.to_string()))?;
    let filename = format!("harvest-{date}.zip");

    let mut url = globals.config().cdn_url.clone();
    url.set_path(&format!("/harvest/{harvest_id}/{token}/{filename}"));

    Ok(url)
}

/// stream the archive to blob storage
async fn upload(globals: &Globals, url: &url::Url, file: &TempFile) -> Result<()> {
    let mut w = globals
        .blobs()
        .writer_with(url.path().trim_start_matches('/'))
        .cache_control(IMMUTABLE)
        .content_type(ARCHIVE_CONTENT_TYPE)
        .await?;

    let mut file = file.open_ro().await?;
    let mut buf = BytesMut::with_capacity(IO_BUF_SIZE);
    loop {
        // splitting the buffer off consumes its capacity, so reserve per read
        buf.reserve(IO_BUF_SIZE);

        let n = file.read_buf(&mut buf).await?;
        if n == 0 {
            break;
        }
        w.write(buf.split_to(n).freeze()).await?;
    }
    w.close().await?;

    Ok(())
}

/// hash the archive, streaming
async fn hash_file(file: &TempFile) -> Result<common::v1::types::misc::hashes::Hashes> {
    use common::v1::types::misc::hashes::{HashData, HashType, Hashes};

    let mut blake3 = blake3::Hasher::new();
    let mut sha512_256 = sha2::Sha512_256::new();

    let mut file = file.open_ro().await?;
    let mut buf = vec![0u8; IO_BUF_SIZE];
    loop {
        let n = file.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        blake3.update(&buf[..n]);
        sha512_256.update(&buf[..n]);
    }

    let mut hashes = Hashes::new();
    hashes.insert(
        HashType::Blake3,
        HashData(blake3.finalize().as_bytes().to_vec().into()),
    );
    hashes.insert(
        HashType::Sha512_256,
        HashData(sha512_256.finalize().to_vec().into()),
    );

    Ok(hashes)
}

/// generate a random url-safe access token for a harvest archive
fn generate_token() -> String {
    let mut bytes = [0u8; 32];
    rand::fill(&mut bytes);
    B64.encode(bytes)
}
