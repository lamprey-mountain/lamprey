use crate::services::search::schema::Doctype;
use crate::services::search::{util::SCHEMA, ServiceSearch};
use crate::Result;
use common::v1::types::{ChannelId, RoomId};
use lamprey_backend_core::types::data::SearchReindexQueueTarget;
use tantivy::Term;

/// request to reindex some stuff
pub enum Reindex {
    /// reindex **everything** on the server
    Everything,

    /// reset reindex queue for a channel's messages
    ///
    /// reindexes messages inside a channel
    QueueMessages(ChannelId),

    /// reset reindex queue for rooms
    ///
    /// reindexes all rooms on the server
    QueueRooms,

    /// reset reindex queue for media
    ///
    /// reindexes all media on the server
    QueueMedia,

    /// reset reindex queue for channels
    ///
    /// reindexes all channels on the server
    QueueChannels,

    /// reset reindex queue for users
    ///
    /// reindexes all users on the server
    QueueUsers,

    /// reset reindex queues for everything inside a room
    ///
    /// reindexes all of a room's channels, audit log entries, and the room itself
    InsideRoom(RoomId),

    /// reset reindex queues for everything inside a channel
    ///
    /// reindexes all of a channel's messages and the channel itself
    InsideChannel(ChannelId),
}

// TODO: maybe "reindex by what term gets deleted" is better
// pub enum Reindex2 {
//     /// reindex **everything** on the server
//     Everything,

//     /// reindex all documents with this room_id
//     ///
//     /// reindexes all of a room's channels, audit log entries, and the room itself
//     Room(RoomId),

//     /// reindex all documents with this channel_id
//     ///
//     /// reindexes all of a channel's messages and the channel itself
//     Channel(ChannelId),

//     /// reindex all documents with doctype == room
//     Rooms,

//     /// reindex all documents with doctype == media
//     QueueMedia,

//     /// reindex all documents with doctype == channel
//     QueueChannels,

//     /// reindex all documents with doctype == user
//     QueueUsers,
// }

impl ServiceSearch {
    /// reindex some content
    pub async fn reindex(&self, reindex: Reindex) -> Result<()> {
        let index = self.get_index().await?;

        match reindex {
            Reindex::Everything => {
                index.delete_all_documents().await?;
                let mut data = self.state.acquire_data().await?;
                data.search_reindex_queue_reset_all_messages().await?;
                data.search_reindex_queue_upsert(SearchReindexQueueTarget::Users, None)
                    .await?;
                data.search_reindex_queue_upsert(SearchReindexQueueTarget::Channels, None)
                    .await?;
                data.search_reindex_queue_upsert(SearchReindexQueueTarget::Media, None)
                    .await?;
                data.commit().await;
            }
            Reindex::QueueMessages(id) => {
                let mut data = self.state.data();
                index
                    .delete_term(Term::from_field_text(SCHEMA.channel_id, &id.to_string()))
                    .await?;
                data.search_reindex_queue_upsert(SearchReindexQueueTarget::Messages(id), None)
                    .await?
            }
            Reindex::QueueMedia => {
                let mut data = self.state.data();
                index
                    .delete_term(Term::from_field_text(
                        SCHEMA.doctype,
                        Doctype::Media.as_str(),
                    ))
                    .await?;
                data.search_reindex_queue_upsert(SearchReindexQueueTarget::Media, None)
                    .await?
            }
            Reindex::QueueChannels => {
                let mut data = self.state.data();
                index
                    .delete_term(Term::from_field_text(
                        SCHEMA.doctype,
                        Doctype::Channel.as_str(),
                    ))
                    .await?;
                data.search_reindex_queue_upsert(SearchReindexQueueTarget::Channels, None)
                    .await?
            }
            Reindex::QueueRooms => {
                let mut data = self.state.data();
                index
                    .delete_term(Term::from_field_text(
                        SCHEMA.doctype,
                        Doctype::Room.as_str(),
                    ))
                    .await?;
                data.search_reindex_queue_upsert(SearchReindexQueueTarget::Rooms, None)
                    .await?
            }
            Reindex::QueueUsers => {
                let mut data = self.state.data();
                index
                    .delete_term(Term::from_field_text(
                        SCHEMA.doctype,
                        Doctype::User.as_str(),
                    ))
                    .await?;
                data.search_reindex_queue_upsert(SearchReindexQueueTarget::Users, None)
                    .await?
            }
            Reindex::InsideRoom(id) => {
                let mut data = self.state.data();
                index
                    .delete_term(Term::from_field_text(SCHEMA.room_id, &id.to_string()))
                    .await?;
                data.search_reindex_queue_reset_room(id).await?;
                // TODO: reindex audit log entries
                // TODO: reindex room
            }
            Reindex::InsideChannel(id) => {
                let mut data = self.state.data();
                index
                    .delete_term(Term::from_field_text(SCHEMA.channel_id, &id.to_string()))
                    .await?;
                data.search_reindex_queue_upsert(SearchReindexQueueTarget::Messages(id), None)
                    .await?
                // TODO: reindex channel
            }
        }
        Ok(())
    }
}
