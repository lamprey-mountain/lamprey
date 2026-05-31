use common::v1::types::{ChannelId, RoomId, UserId};
use tantivy::query::Query;

// TODO: impl TantivyVisibility for everything
// TODO: add doc comments

/// what messages to include in the search
#[derive(Debug, Clone)]
pub enum SearchMessagesVisibility {
    /// all messages
    Everything,

    /// only messages in these filtered channels
    Filtered(Vec<ChannelVisibility>),
}

#[derive(Debug, Clone)]
pub struct ChannelVisibility {
    /// the id of the channel
    pub id: ChannelId,

    /// whether to include private threads
    ///
    /// should be set to true if the `ThreadsManage` permission is enabled
    pub can_view_private_threads: bool,
}

/// what channels to include in the search
#[derive(Debug, Clone)]
pub enum SearchChannelsVisibility {
    /// all channels
    Everything,

    Filtered {
        // for dms/gdms
        user_ids: Vec<UserId>,
        room_ids: Vec<RoomId>,
    },
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
    Everything,
    PublicOnly,
    Owner(UserId),
    PublicOrOwner(UserId),
}

#[derive(Debug, Clone)]
pub enum SearchMediaVisibility {
    Everything,

    /// only media from these users
    ///
    /// eg. a user and all their bots
    Users(Vec<UserId>),
}

pub trait TantivyVisibility {
    fn into_query(self) -> Box<dyn Query>;
}

impl TantivyVisibility for SearchMessagesVisibility {
    fn into_query(self) -> Box<dyn Query> {
        match self {
            SearchMessagesVisibility::Everything => tantivy::query::AllQuery,
            SearchMessagesVisibility::Filtered(items) => {
                let mut channel_terms = vec![];
                let mut parent_channel_terms = vec![];
                for item in items {
                    let id = item.id;
                    let can_view_private_threads = item.can_view_private_threads;
                    let id_str = id.to_string();
                    channel_terms.push(Term::from_field_text(SCHEMA.channel_id, &id_str));

                    if *can_view_private_threads {
                        parent_channel_terms
                            .push(Term::from_field_text(SCHEMA.parent_channel_id, &id_str));
                    }
                }

                let mut vis_queries: Vec<(Occur, Box<dyn Query>)> = vec![];

                if !channel_terms.is_empty() {
                    vis_queries.push((Occur::Should, Box::new(TermSetQuery::new(channel_terms))));
                }

                if !parent_channel_terms.is_empty() {
                    vis_queries.push((
                        Occur::Should,
                        Box::new(TermSetQuery::new(parent_channel_terms)),
                    ));
                }

                Box::new(BooleanQuery::new(vis_queries))
            }
        }
    }
}
