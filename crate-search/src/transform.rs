use common::{
    v1::types::{AuditLogEntry, Channel, Message, MessageAttachmentType, MessageType, Room, User},
    v2::types::{ChannelId, RoomId, media::Media},
};
use std::collections::BTreeMap;
use tantivy::schema::OwnedValue;
use tantivy::{DateTime as TantivyDT, TantivyDocument};

use crate::{
    schema::{SCHEMA, UnifiedSchema},
    util::doctype::Doctype,
};

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
    }

    pub struct SearchMedia<'a> {
        pub media: &'a Media,
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

        doc.add_text(
            s.subtype,
            match &message.latest_version.message_type {
                MessageType::DefaultMarkdown(..) => "DefaultMarkdown",
                MessageType::MessagePinned(..) => "MessagePinned",
                MessageType::MemberAdd(..) => "MemberAdd",
                MessageType::MemberRemove(..) => "MemberRemove",
                MessageType::MemberJoin => "MemberJoin",
                MessageType::Call(..) => "Call",
                MessageType::ChannelRename(..) => "ChannelRename",
                MessageType::ChannelPingback(..) => "ChannelPingback",
                MessageType::ChannelMoved(..) => "ChannelMoved",
                MessageType::ChannelIcon(..) => "ChannelIcon",
                MessageType::ThreadCreated(..) => "ThreadCreated",
                MessageType::AutomodExecution(..) => "AutomodExecution",
            },
        );

        let mut meta_fast: BTreeMap<String, OwnedValue> = BTreeMap::new();
        let mut meta_text: BTreeMap<String, OwnedValue> = BTreeMap::new();

        let reply = match &message.latest_version.message_type {
            MessageType::DefaultMarkdown(m) => m.reply_id,
            MessageType::MessagePinned(p) => Some(p.pinned_message_id),
            MessageType::ThreadCreated(m) => m.source_message_id,
            _ => None,
        };

        doc.add_text(s.content, message.latest_version.message_type.to_string());

        if let MessageType::DefaultMarkdown(ref m) = message.latest_version.message_type {
            if !m.attachments.is_empty() {
                meta_fast.insert("has_attachment".to_string(), true.into());

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

        // FIXME: link extraction (use lamprey-markdown)
        // let mut has_links = false;
        // if let MessageType::DefaultMarkdown(ref m) = message.latest_version.message_type {
        //     if let Some(ref content) = m.content {
        //         let mut hostnames = Vec::new();
        //         for url in crate::services::messages::links::extract_links(content) {
        //             if let Some(host) = url.host_str() {
        //                 let reversed_hostname = host.split('.').rev().collect::<Vec<_>>().join(".");
        //                 hostnames.push(reversed_hostname.into());
        //                 has_links = true;
        //             }
        //         }
        //         if !hostnames.is_empty() {
        //             meta_fast.insert("link_hostname".to_string(), OwnedValue::Array(hostnames));
        //         }
        //     }
        // }
        // meta_fast.insert("has_link".to_string(), has_links.into());

        doc.add_object(s.metadata_fast, meta_fast);
        doc.add_object(s.metadata_text, meta_text);

        doc
    }
}

impl SearchDocument for SearchUser<'_> {
    fn to_tantivy(&self) -> TantivyDocument {
        todo!()
    }
}

impl SearchDocument for SearchRoom<'_> {
    fn to_tantivy(&self) -> TantivyDocument {
        todo!()
    }
}

impl SearchDocument for SearchChannel<'_> {
    fn to_tantivy(&self) -> TantivyDocument {
        todo!()
    }
}

impl SearchDocument for SearchMedia<'_> {
    fn to_tantivy(&self) -> TantivyDocument {
        todo!()
    }
}

impl SearchDocument for SearchAuditLogEntry<'_> {
    fn to_tantivy(&self) -> TantivyDocument {
        todo!()
    }
}

// TODO: split each resource apart into submodules
// pub mod message;
// pub mod user;
// pub mod channel;
// pub mod media;
// etc...
