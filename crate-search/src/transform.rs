use common::{
    v1::types::{
        AuditLogEntry, Channel, Message, MessageAttachmentType, MessageType, Room, RoomMember,
        User, search::Doctype, util::Time,
    },
    v2::types::{ChannelId, RoomId, media::Media},
};
use lamprey_markdown::{Parser, query::QueryableExt};
use std::collections::BTreeMap;
use tantivy::schema::OwnedValue;
use tantivy::{DateTime as TantivyDT, TantivyDocument};
use url::Url;

use crate::schema::{SCHEMA, UnifiedSchema};

/// trait for transforming data into tantivy compatible documents
pub trait SearchDocument {
    /// convert this into a tantivy document
    // TODO: make this return a Result?
    fn to_tantivy(&self) -> TantivyDocument;

    // PERF: maybe i could manually impl tantivy::Document?
}

macro_rules! define_transformer {
    ($(
        pub struct $struct_name:ident $(< $( $gen:tt ),* >)? {
            $(pub $field_name:ident : $field_type:ty),* $(,)?
        }
    )*) => {
        $(
            pub struct $struct_name $(< $( $gen ),* >)? {
                $(pub $field_name: $field_type),*
            }

            impl $(< $( $gen ),* >)? $struct_name $(< $( $gen ),* >)? {
                pub fn new($($field_name: $field_type),*) -> Self {
                    Self {
                        $($field_name),*
                    }
                }

                pub fn transform($($field_name: $field_type),*) -> TantivyDocument {
                    Self::new($($field_name),*).to_tantivy()
                }
            }

            pastey::paste! {
                impl $(< $( $gen ),* >)? UnifiedSchema {
                    pub fn [< transform_ $struct_name:replace("Search", ""):snake >] (
                        &self,
                        $($field_name: $field_type),*
                    ) -> TantivyDocument {
                        $struct_name::transform($($field_name),*)
                    }
                }
            }
        )*
    };
}

define_transformer! {
    pub struct SearchMessage<'a> {
        pub message: &'a Message,
        pub room_id: Option<RoomId>,
        pub parent_channel_id: Option<ChannelId>,
    }

    pub struct SearchUser<'a> {
        pub user: &'a User,
    }

    pub struct SearchRoom<'a> {
        pub room: &'a Room,
    }

    pub struct SearchChannel<'a> {
        pub channel: &'a Channel,
        pub first_message: Option<&'a Message>,
        pub hotness: Option<f64>,
    }

    pub struct SearchMedia<'a> {
        pub media: &'a Media,
    }

    pub struct SearchRoomMember<'a> {
        pub member: &'a RoomMember,
    }

    pub struct SearchAuditLogEntry<'a> {
        pub ent: &'a AuditLogEntry,
    }

    // TODO: fill out rest of SearchFoo structs
}

impl SearchDocument for SearchMessage<'_> {
    fn to_tantivy(&self) -> TantivyDocument {
        let s = &*SCHEMA;
        let message = self.message;

        let mut doc = TantivyDocument::new();
        doc.add_text(s.id, message.id.to_string());
        doc.add_text(s.doctype, Doctype::Message);
        doc.add_text(s.channel_id, message.channel_id.to_string());

        if let Some(pid) = self.parent_channel_id {
            doc.add_text(s.parent_channel_id, pid.to_string());
        }

        doc.add_text(s.author_id, message.author_id.to_string());
        doc.add_date(s.created_at, TantivyDT::from_utc(*message.created_at));

        let updated_at = message.latest_version.created_at;
        if updated_at != message.created_at {
            doc.add_date(s.updated_at, TantivyDT::from_utc(*updated_at));
        }

        if let Some(deleted_at) = message.deleted_at {
            doc.add_date(s.deleted_at, TantivyDT::from_utc(*deleted_at));
        }

        if let Some(removed_at) = message.removed_at {
            doc.add_date(s.removed_at, TantivyDT::from_utc(*removed_at));
        }

        if let Some(room_id) = self.room_id {
            doc.add_text(s.room_id, room_id.to_string());
        }

        doc.add_text(s.subtype, message.latest_version.message_type.as_str());

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        let mut meta_text: BTreeMap<String, OwnedValue> = BTreeMap::new();

        let reply = match &message.latest_version.message_type {
            MessageType::DefaultMarkdown(m) | MessageType::ThreadInitial(m) => m.reply_id,
            MessageType::MessagePinned(p) => Some(p.pinned_message_id),
            MessageType::ThreadCreated(m) => m.source_message_id,
            _ => None,
        };

        doc.add_text(s.content, message.latest_version.message_type.to_string());

        if let MessageType::DefaultMarkdown(ref m) | MessageType::ThreadInitial(ref m) =
            message.latest_version.message_type
        {
            if !m.attachments.is_empty() {
                meta_fast.insert("has_attachment".to_string(), true.into());

                // NOTE: maybe i should calculate these based on media.metadata instead of media.content_type?
                let has_audio = m.attachments.iter().any(|a| {
                    let MessageAttachmentType::Media { media } = &a.ty;
                    media.content_type.to_string().starts_with("audio/")
                });
                let has_image = m.attachments.iter().any(|a| {
                    let MessageAttachmentType::Media { media } = &a.ty;
                    media.content_type.to_string().starts_with("image/")
                });
                let has_video = m.attachments.iter().any(|a| {
                    let MessageAttachmentType::Media { media } = &a.ty;
                    media.content_type.to_string().starts_with("video/")
                });

                meta_fast.insert("has_audio".to_string(), has_audio.into());
                meta_fast.insert("has_image".to_string(), has_image.into());
                meta_fast.insert("has_video".to_string(), has_video.into());

                for att in &m.attachments {
                    let MessageAttachmentType::Media { media } = &att.ty;
                    let push_val =
                        |map: &mut BTreeMap<String, OwnedValue>, key: &str, val: OwnedValue| {
                            let entry = map
                                .entry(key.to_string())
                                .or_insert_with(|| OwnedValue::Array(Vec::new()));
                            if let OwnedValue::Array(vec) = entry {
                                vec.push(val);
                            }
                        };

                    push_val(&mut meta_fast, "media_size", media.size.into());
                    push_val(
                        &mut meta_fast,
                        "media_content_type",
                        media.content_type.to_string().into(),
                    );
                    push_val(
                        &mut meta_fast,
                        "media_filename",
                        media.filename.as_str().into(),
                    );

                    if let Some(alt) = &media.alt {
                        push_val(&mut meta_text, "media_alt", alt.as_str().into());
                    }

                    let extension = std::path::Path::new(&media.filename)
                        .extension()
                        .and_then(|e| e.to_str())
                        .map(|ext| ext.to_lowercase());
                    if let Some(e) = extension {
                        push_val(&mut meta_fast, "media_extension", e.as_str().into());
                    }
                }
            } else {
                meta_fast.insert("has_attachment".to_string(), false.into());
                meta_fast.insert("has_audio".to_string(), false.into());
                meta_fast.insert("has_image".to_string(), false.into());
                meta_fast.insert("has_video".to_string(), false.into());
            }

            meta_fast.insert("has_embed".to_string(), (!m.embeds.is_empty()).into());
        };

        meta_fast.insert("has_thread".to_string(), message.thread.is_some().into());
        meta_fast.insert("pinned".to_string(), message.pinned.is_some().into());

        if let Some(reply_id) = reply {
            meta_fast.insert("reply".to_string(), reply_id.to_string().into());
        }

        let mn = &message.latest_version.mentions;
        meta_fast.insert("mentions_everyone".to_string(), mn.everyone.into());

        if !mn.roles.is_empty() {
            let roles: Vec<OwnedValue> = mn.roles.iter().map(|r| r.id.to_string().into()).collect();
            meta_fast.insert("mentions_role".to_string(), OwnedValue::Array(roles));
        }

        if !mn.users.is_empty() {
            let users: Vec<OwnedValue> = mn.users.iter().map(|u| u.id.to_string().into()).collect();
            meta_fast.insert("mentions_user".to_string(), OwnedValue::Array(users));
        }

        let mut has_links = false;
        if let MessageType::DefaultMarkdown(ref m) = message.latest_version.message_type {
            if let Some(ref content) = m.content {
                let mut hostnames = Vec::new();
                let parser = Parser::new();
                let parsed = parser.parse(content);
                for url in parsed
                    .tree()
                    .iter_links()
                    .filter_map(|link| Url::parse(&link.href()).ok())
                {
                    if let Some(host) = url.host_str() {
                        let reversed_hostname = host.split('.').rev().collect::<Vec<_>>().join(".");
                        hostnames.push(reversed_hostname.into());
                        has_links = true;
                    }
                }

                if !hostnames.is_empty() {
                    meta_fast.insert("link_hostname".to_string(), OwnedValue::Array(hostnames));
                }
            }
        }
        meta_fast.insert("has_link".to_string(), has_links.into());

        doc.add_object(s.metadata_fast, meta_fast);
        doc.add_object(s.metadata_text, meta_text);

        doc
    }
}

impl SearchDocument for SearchUser<'_> {
    fn to_tantivy(&self) -> TantivyDocument {
        let s = &*SCHEMA;
        let user = self.user;

        let mut doc = TantivyDocument::new();
        doc.add_text(s.id, user.id.to_string());
        doc.add_text(s.doctype, Doctype::User);
        doc.add_text(s.name, user.name.clone());

        if let Some(description) = user.description.clone() {
            doc.add_text(s.content, description);
        }

        let created_at: Time = user.id.try_into().unwrap();
        doc.add_date(s.created_at, TantivyDT::from_utc(*created_at));

        if let Some(deleted_at) = user.deleted_at {
            doc.add_date(s.deleted_at, TantivyDT::from_utc(*deleted_at));
        }

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        meta_fast.insert("bot".to_string(), user.bot.into());
        meta_fast.insert("system".to_string(), user.system.into());
        meta_fast.insert("suspended".to_string(), user.is_suspended().into());

        if let Some(registered_at) = user.registered_at {
            meta_fast.insert(
                "registered_at".to_string(),
                TantivyDT::from_utc(*registered_at).into(),
            );
        }

        meta_fast.insert("puppet".to_string(), user.puppet.is_some().into());
        if let Some(puppet) = &user.puppet {
            meta_fast.insert(
                "puppet_owner_id".to_string(),
                puppet.owner_id.to_string().into(),
            );
            if let Some(alias_id) = &puppet.alias_id {
                meta_fast.insert("puppet_alias_id".to_string(), alias_id.to_string().into());
            }
        }

        // TODO(future): index these extra fields:
        // server_role_id: Vec<RoleId>, -- ids of roles in the server room this user has
        // room_id: Vec<RoomId>, -- ids of rooms this user is in

        doc.add_object(s.metadata_fast, meta_fast);

        doc
    }
}

impl SearchDocument for SearchRoom<'_> {
    fn to_tantivy(&self) -> TantivyDocument {
        let s = &*SCHEMA;
        let room = self.room;
        let mut doc = TantivyDocument::new();
        doc.add_text(s.id, room.id.to_string());
        doc.add_text(s.doctype, Doctype::Room);
        doc.add_text(s.name, room.name.clone());

        if let Some(description) = &room.description {
            doc.add_text(s.content, description.clone());
        }

        let created_at: Time = room.id.try_into().unwrap();
        doc.add_date(s.created_at, TantivyDT::from_utc(*created_at));

        if let Some(deleted_at) = room.deleted_at {
            doc.add_date(s.deleted_at, TantivyDT::from_utc(*deleted_at));
        }

        if let Some(archived_at) = room.archived_at {
            doc.add_date(s.archived_at, TantivyDT::from_utc(*archived_at));
        }

        if let Some(owner_id) = room.owner_id {
            doc.add_text(s.author_id, owner_id.to_string());
        }

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        meta_fast.insert("public".to_string(), room.public.into());
        meta_fast.insert("member_count".to_string(), room.member_count.into());
        meta_fast.insert("quarantined".to_string(), room.quarantined.into());

        doc.add_object(s.metadata_fast, meta_fast);
        doc
    }
}

impl SearchDocument for SearchChannel<'_> {
    fn to_tantivy(&self) -> TantivyDocument {
        let s = &*SCHEMA;
        let channel = self.channel;
        let mut doc = TantivyDocument::new();
        doc.add_text(s.id, channel.id.to_string());
        doc.add_text(s.doctype, Doctype::Channel);

        let last_activity: Time = channel
            .archived_at
            .or_else(|| channel.last_version_id.and_then(|id| id.try_into().ok()))
            .or_else(|| channel.id.try_into().ok())
            .unwrap();
        doc.add_date(s.updated_at, TantivyDT::from_utc(*last_activity));
        doc.add_text(s.name, channel.name.clone());

        if let Some(description) = &channel.description {
            doc.add_text(s.content, description.clone());
        }

        if let Some(room_id) = channel.room_id {
            doc.add_text(s.room_id, room_id.to_string());
        }

        if let Some(parent_id) = channel.parent_id {
            doc.add_text(s.channel_id, parent_id.to_string());
        }

        if let Some(owner_id) = channel.owner_id.map(|i| i.to_string()) {
            doc.add_text(s.author_id, owner_id);
        }

        if let Some(tags) = &channel.tags {
            for tag_id in tags {
                doc.add_text(s.tag_id, tag_id.to_string());
            }
        }

        let created_at: Time = channel.id.try_into().unwrap();
        doc.add_date(s.created_at, TantivyDT::from_utc(*created_at));

        if let Some(deleted_at) = channel.deleted_at {
            doc.add_date(s.deleted_at, TantivyDT::from_utc(*deleted_at));
        }

        if let Some(archived_at) = channel.archived_at {
            doc.add_date(s.archived_at, TantivyDT::from_utc(*archived_at));
        }

        doc.add_text(s.subtype, channel.ty.as_str());

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        meta_fast.insert("nsfw".to_string(), channel.nsfw.into());

        if let Some(bitrate) = channel.bitrate {
            meta_fast.insert("bitrate".to_string(), bitrate.into());
        }

        if let Some(user_limit) = channel.user_limit {
            meta_fast.insert("user_limit".to_string(), user_limit.into());
        }

        let recipients: Vec<OwnedValue> = channel
            .recipients
            .iter()
            .map(|u| u.id.to_string().into())
            .collect();
        meta_fast.insert("recipients".to_string(), OwnedValue::Array(recipients));

        if let Some(m) = self.first_message {
            for r in &m.reactions.0 {
                let key = r.key.to_key_str().replace(r"\", r"\\").replace(r".", r"\.");
                meta_fast.insert(format!("reactions.{key}"), OwnedValue::U64(r.count));
            }

            // TODO: use calculate_hotness here
            if let Some(hotness) = self.hotness {
                meta_fast.insert("hotness".to_string(), OwnedValue::F64(hotness));
            }
        }

        doc.add_object(s.metadata_fast, meta_fast);
        doc
    }
}

impl SearchDocument for SearchMedia<'_> {
    fn to_tantivy(&self) -> TantivyDocument {
        let s = &*SCHEMA;
        let media = self.media;
        let user_id = media.user_id.expect("Media missing user_id");

        let mut doc = TantivyDocument::new();

        doc.add_text(s.id, media.id.to_string());
        doc.add_text(s.doctype, Doctype::Media);

        let created_at: Time = media.id.try_into().expect("Invalid media id format");
        doc.add_date(s.created_at, TantivyDT::from_utc(*created_at));
        doc.add_text(s.author_id, user_id.to_string());

        if let Some(r) = media.room_id {
            doc.add_text(s.room_id, r.to_string());
        }

        if let Some(r) = media.channel_id {
            doc.add_text(s.channel_id, r.to_string());
        }

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        let mut meta_text: BTreeMap<String, OwnedValue> = BTreeMap::new();

        meta_fast.insert("media_size".to_string(), media.size.into());
        meta_fast.insert(
            "media_content_type".to_string(),
            media.content_type.to_string().into(),
        );
        meta_fast.insert("media_filename".to_string(), media.filename.clone().into());

        let extension = std::path::Path::new(&media.filename)
            .extension()
            .and_then(|e| e.to_str())
            .map(|ext| ext.to_lowercase());
        if let Some(e) = extension {
            meta_fast.insert("media_extension".to_string(), e.into());
        }

        if let Some(alt) = &media.alt {
            meta_text.insert("media_alt".to_string(), alt.clone().into());
        }

        meta_fast.insert("quarantined".to_string(), media.quarantine.is_some().into());

        doc.add_object(s.metadata_fast, meta_fast);
        doc.add_object(s.metadata_text, meta_text);

        doc
    }
}

impl SearchDocument for SearchAuditLogEntry<'_> {
    fn to_tantivy(&self) -> TantivyDocument {
        let s = &*SCHEMA;
        let ent = self.ent;
        let mut doc = TantivyDocument::new();
        doc.add_text(s.id, ent.id.to_string());
        doc.add_text(s.doctype, Doctype::AuditLogEntry);
        doc.add_text(s.room_id, ent.room_id.to_string());
        doc.add_text(s.author_id, ent.user_id.to_string());
        doc.add_date(s.created_at, TantivyDT::from_utc(*ent.started_at));

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        meta_fast.insert("status".to_string(), format!("{:?}", ent.status).into());
        meta_fast.insert("audit_event".to_string(), format!("{:?}", ent.ty).into());

        if let Some(app_id) = ent.application_id {
            meta_fast.insert("application_id".to_string(), app_id.to_string().into());
        }

        doc.add_object(s.metadata_fast, meta_fast);

        doc
    }
}

impl SearchDocument for SearchRoomMember<'_> {
    fn to_tantivy(&self) -> TantivyDocument {
        let s = &*SCHEMA;
        let member = self.member;

        let mut doc = TantivyDocument::new();
        doc.add_text(s.id, format!("{}:{}", member.user_id, member.room_id));
        doc.add_text(s.doctype, Doctype::RoomMember);
        doc.add_text(s.room_id, member.room_id.to_string());
        doc.add_text(s.author_id, member.user_id.to_string());

        doc.add_date(s.created_at, TantivyDT::from_utc(*member.joined_at));

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        meta_fast.insert("mute".to_string(), member.mute.into());
        meta_fast.insert("deaf".to_string(), member.deaf.into());
        meta_fast.insert("quarantined".to_string(), member.quarantined.into());

        if let Some(o) = &member.override_name {
            doc.add_text(s.name, o);
        }

        if let Some(o) = &member.override_description {
            doc.add_text(s.content, o);
        }

        if let Some(o) = &member.origin {
            let val = serde_json::to_value(o.clone()).unwrap();
            meta_fast.insert("origin".to_string(), val.into());
        }

        if !member.roles.is_empty() {
            let roles: Vec<OwnedValue> =
                member.roles.iter().map(|r| r.to_string().into()).collect();
            meta_fast.insert("roles".to_string(), OwnedValue::Array(roles));
        }

        if let Some(timeout) = &member.timeout_until {
            meta_fast.insert(
                "timeout_until".to_string(),
                TantivyDT::from_utc(**timeout).into(),
            );
        }

        doc.add_object(s.metadata_fast, meta_fast);

        doc
    }
}

// TODO: split each resource apart into submodules?
// pub mod message;
// pub mod user;
// pub mod channel;
// pub mod media;
// etc...

// TODO: add more transformers
// mod next {
//     /// transform an analytics event to a tantivy document
//     pub fn transform_analytics_event(&self, event: &AnalyticsEvent) -> Result<TantivyDocument> {
//         todo!()
//     }

//     /// transform a serialized lamprey document to a tantivy document
//     pub fn transform_document(
//         &self,
//         document: &Channel,
//         serialized: &Serdoc,
//     ) -> Result<TantivyDocument> {
//         todo!()
//     }

//     /// transform a single lamprey document change to a tantivy document
//     pub fn transform_document_change(
//         &self,
//         change: &DocumentChange,
//         document: &Channel,
//     ) -> Result<TantivyDocument> {
//         todo!()
//     }

//     /// transform a call into a tantivy document
//     ///
//     /// for public broadcasts
//     pub fn transform_call(&self, call: &Call, channel: &Channel) -> Result<TantivyDocument> {
//         todo!()
//     }

//     pub fn transform_custom_emoji(&self, emoji: EmojiCustom) -> Result<TantivyDocument> {
//         todo!()
//     }

//     pub fn transform_forum_tag(&self, tag: &Tag, channel: &Channel) -> Result<TantivyDocument> {
//         todo!()
//     }

//     pub fn transform_application(&self, app: &Application) -> Result<TantivyDocument> {
//         todo!()
//     }

//     pub fn transform_room_template(&self, template: &RoomTemplate) -> Result<TantivyDocument> {
//         todo!()
//     }

//     pub fn transform_calendar_event(
//         &self,
//         event: &CalendarEvent,
//         channel: &Channel,
//     ) -> Result<TantivyDocument> {
//         todo!()
//     }
// }
