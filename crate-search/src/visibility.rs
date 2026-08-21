//! controlling which search results are returned

use common::v2::types::{ChannelId, RoomId, UserId};
use tantivy::{
    Term,
    query::{AllQuery, BooleanQuery, Query, TermQuery, TermSetQuery},
    schema::IndexRecordOption,
};

use crate::{schema::SCHEMA, util::BqBuilder};

/// Trait for converting visibility constraints into Tantivy queries.
pub trait TantivyVisibility {
    /// Convert the visibility constraint into a Tantivy query.
    fn into_query(self) -> Box<dyn Query>;
}

/// visibility for a single channel
#[derive(Debug, Clone)]
pub struct ChannelVisibility {
    /// the id of the channel
    pub id: ChannelId,

    /// whether to include private threads
    ///
    /// should be set to true if the `ThreadsManage` permission is enabled
    pub can_view_private_threads: bool,
}

// TODO: rename SearchFooVisibility to FooFilter, FooVisibility, something shorter

/// what messages to include in the search
#[derive(Debug, Clone)]
pub enum SearchMessagesVisibility {
    /// all messages
    Everything,

    /// only messages in these filtered channels
    Filtered(Vec<ChannelVisibility>),

    /// public messages + these channels
    Public(Vec<ChannelVisibility>),

    /// only public messages
    PublicOnly,
}

/// what channels to include in the search
#[derive(Debug, Clone)]
pub enum SearchChannelsVisibility {
    /// all channels
    Everything,

    /// only channels in these rooms or owned by these users
    Filtered {
        /// for dms/gdms
        user_ids: Vec<UserId>,

        /// for regular channels
        room_ids: Vec<RoomId>,
    },

    /// public channels + these rooms/users
    Public {
        /// for dms/gdms
        user_ids: Vec<UserId>,

        /// for regular channels
        room_ids: Vec<RoomId>,
    },

    /// only public channels
    PublicOnly,
}

/// what rooms to include in the search
#[derive(Debug, Clone)]
pub enum SearchRoomsVisibility {
    /// public rooms + these rooms
    Public(Vec<RoomId>),

    /// only public rooms
    PublicOnly,

    /// all rooms
    Everything,
}

/// what applications to include in the search
#[derive(Debug, Clone)]
pub enum SearchApplicationsVisibility {
    /// all applications
    Everything,

    /// only public applications
    PublicOnly,

    /// only applications owned by this user
    Owner(UserId),

    /// public applications or applications owned by this user
    PublicOrOwner(UserId),
}

/// what media to include in the search
#[derive(Debug, Clone)]
pub enum SearchMediaVisibility {
    /// all media
    Everything,

    /// only media from these users
    ///
    /// eg. a user and all their bots
    Users(Vec<UserId>),
}

/// which users to include in the search
#[derive(Debug, Clone)]
pub enum SearchUserVisibility {
    /// all users
    Everything,

    /// only these users
    Filtered {
        /// friends and applications/bots
        user_ids: Vec<UserId>,

        /// rooms the user is in (for mutual rooms)
        room_ids: Vec<RoomId>,
    },
}

#[derive(Debug, Clone)]
pub enum SearchAuditLogVisibility {
    /// all media
    Everything,

    /// only entries from this room
    Room(RoomId),
}

impl TantivyVisibility for SearchMessagesVisibility {
    fn into_query(self) -> Box<dyn Query> {
        match self {
            SearchMessagesVisibility::Everything => Box::new(AllQuery),
            SearchMessagesVisibility::PublicOnly => SCHEMA.query_public(),
            SearchMessagesVisibility::Filtered(items) => Self::filtered_query(items),
            SearchMessagesVisibility::Public(items) => {
                let mut q = BqBuilder::new();
                q.should(SCHEMA.query_public());
                q.should(Self::filtered_query(items));
                Box::new(q.build())
            }
        }
    }
}

impl SearchMessagesVisibility {
    fn filtered_query(items: Vec<ChannelVisibility>) -> Box<dyn Query> {
        let mut channel_terms = vec![];
        let mut parent_channel_terms = vec![];
        for item in items {
            let id = item.id;
            let can_view_private_threads = item.can_view_private_threads;
            let id_str = id.to_string();
            channel_terms.push(Term::from_field_text(SCHEMA.channel_id, &id_str));

            if can_view_private_threads {
                parent_channel_terms.push(Term::from_field_text(SCHEMA.parent_channel_id, &id_str));
            }
        }

        let mut q = BqBuilder::new();

        if !channel_terms.is_empty() {
            q.should(Box::new(TermSetQuery::new(channel_terms)));
        }

        if !parent_channel_terms.is_empty() {
            q.should(Box::new(TermSetQuery::new(parent_channel_terms)));
        }

        Box::new(q.build())
    }
}

impl TantivyVisibility for SearchChannelsVisibility {
    fn into_query(self) -> Box<dyn Query> {
        match self {
            SearchChannelsVisibility::Everything => Box::new(AllQuery),
            SearchChannelsVisibility::PublicOnly => SCHEMA.query_public(),
            SearchChannelsVisibility::Filtered { user_ids, room_ids } => {
                Self::filtered_query(user_ids, room_ids)
            }
            SearchChannelsVisibility::Public { user_ids, room_ids } => {
                let mut q = BqBuilder::new();
                q.should(SCHEMA.query_public());
                q.should(Self::filtered_query(user_ids, room_ids));
                Box::new(q.build())
            }
        }
    }
}

impl SearchChannelsVisibility {
    fn filtered_query(user_ids: Vec<UserId>, room_ids: Vec<RoomId>) -> Box<dyn Query> {
        let mut q = BqBuilder::new();

        if !room_ids.is_empty() {
            let terms: Vec<_> = room_ids
                .iter()
                .map(|id| Term::from_field_text(SCHEMA.room_id, &id.to_string()))
                .collect();
            q.should(Box::new(TermSetQuery::new(terms)));
        }

        if !user_ids.is_empty() {
            let terms: Vec<_> = user_ids
                .iter()
                .map(|id| Term::from_field_text(SCHEMA.author_id, &id.to_string()))
                .collect();
            q.should(Box::new(TermSetQuery::new(terms)));
        }

        Box::new(q.build())
    }
}

impl TantivyVisibility for SearchRoomsVisibility {
    fn into_query(self) -> Box<dyn Query> {
        match self {
            SearchRoomsVisibility::Everything => Box::new(AllQuery),
            SearchRoomsVisibility::PublicOnly => SCHEMA.query_public(),
            SearchRoomsVisibility::Public(ids) => {
                let mut q = BqBuilder::new();

                q.should(SCHEMA.query_public());

                if !ids.is_empty() {
                    let terms: Vec<_> = ids
                        .iter()
                        .map(|id| Term::from_field_text(SCHEMA.id, &id.to_string()))
                        .collect();
                    q.should(Box::new(TermSetQuery::new(terms)));
                }

                Box::new(q.build())
            }
        }
    }
}

impl TantivyVisibility for SearchApplicationsVisibility {
    fn into_query(self) -> Box<dyn Query> {
        match self {
            SearchApplicationsVisibility::Everything => Box::new(AllQuery),
            SearchApplicationsVisibility::PublicOnly => SCHEMA.query_public(),
            SearchApplicationsVisibility::Owner(user_id) => {
                let term = Term::from_field_text(SCHEMA.author_id, &user_id.to_string());
                Box::new(TermQuery::new(term, IndexRecordOption::Basic))
            }
            SearchApplicationsVisibility::PublicOrOwner(user_id) => {
                let mut q = BqBuilder::new();
                q.should(Box::new(SCHEMA.query_public()));
                q.should(Box::new(SCHEMA.query_author_id(user_id)));
                Box::new(q.build())
            }
        }
    }
}

impl TantivyVisibility for SearchMediaVisibility {
    fn into_query(self) -> Box<dyn Query> {
        match self {
            SearchMediaVisibility::Everything => Box::new(AllQuery),
            SearchMediaVisibility::Users(user_ids) => {
                if user_ids.is_empty() {
                    return Box::new(BooleanQuery::new(vec![]));
                }
                let terms: Vec<_> = user_ids
                    .iter()
                    .map(|id| SCHEMA.term_author_id(*id))
                    .collect();
                Box::new(TermSetQuery::new(terms))
            }
        }
    }
}

impl TantivyVisibility for SearchAuditLogVisibility {
    fn into_query(self) -> Box<dyn Query> {
        match self {
            SearchAuditLogVisibility::Everything => Box::new(AllQuery),
            SearchAuditLogVisibility::Room(room_id) => SCHEMA.query_room_id(room_id),
        }
    }
}

impl TantivyVisibility for SearchUserVisibility {
    fn into_query(self) -> Box<dyn Query> {
        match self {
            SearchUserVisibility::Everything => Box::new(AllQuery),
            SearchUserVisibility::Filtered { user_ids, room_ids } => {
                let mut q = BqBuilder::new();

                if !user_ids.is_empty() {
                    let terms: Vec<_> = user_ids
                        .iter()
                        .map(|id| Term::from_field_text(SCHEMA.id, &id.to_string()))
                        .collect();
                    q.should(Box::new(TermSetQuery::new(terms)));
                }

                if !room_ids.is_empty() {
                    let terms: Vec<_> = room_ids
                        .iter()
                        .map(|id| Term::from_field_text(SCHEMA.room_id, &id.to_string()))
                        .collect();
                    q.should(Box::new(TermSetQuery::new(terms)));
                }

                Box::new(q.build())
            }
        }
    }
}

// PERF: impl tantivy query directly?
// impl tantivy::query::Query for SearchMediaVisibility {}
// impl tantivy::query::Weight for ??? {}
