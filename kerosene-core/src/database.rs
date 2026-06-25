use async_trait::async_trait;
use lamprey::{v1::types::Room, v2::types::RoomId};

pub use crate::prelude::*;

/// a handle to the database
#[async_trait]
pub trait Database: Send + Sync {
    /// apply all pending migrations
    async fn migrate(&self) -> ServerResult<()>;

    /// check if the database is reachable and healthy
    async fn check_database(&self) -> ServerResult<bool>;

    /// create a new unit of work in a new transaction
    async fn begin(&self) -> ServerResult<AnyTransaction>;

    /// start a new unit of work with no transaction
    async fn begin_read(&self) -> ServerResult<AnyTransaction>;
}

pub type AnyTransaction = Box<dyn Transaction>;

/// a transaction or connection that can be used to query or update the database
#[async_trait]
pub trait Transaction: Send + Sync {
    /// mark this transaction to be rolled back
    ///
    /// not doing this will still implicitly rollback, but will also log warnings
    fn rollback(self: Box<Self>);

    /// wait until this transaction is fully rolled back
    async fn rollback_full(self: Box<Self>) -> ServerResult<()>;

    /// wait until this transaction is fully committed
    async fn commit(self: Box<Self>) -> ServerResult<()>;
}

#[async_trait]
pub trait DataFoo {
    // TODO
}

#[async_trait]
pub trait DataRoom {
    async fn room_insert(&mut self, room: Room) -> ServerResult<()>;
    async fn room_get(&mut self, room_id: RoomId) -> ServerResult<Room>;
    // etc...
}
