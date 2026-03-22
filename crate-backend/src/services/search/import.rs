use std::{collections::HashSet, ops::ControlFlow, sync::Arc};

use common::v1::types::{ChannelId, PaginationDirection, PaginationQuery};
use kameo::{
    actor::{ActorRef, Spawn},
    prelude::{Context, Message},
    Actor,
};
use lamprey_backend_core::Error;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::{
    services::search::{
        index::{AddDocument, CommitIndex, IndexActorRef},
        schema::{content::ContentSchema, tantivy_document_from_message},
    },
    Result, ServerStateInner,
};

/// Internal message to trigger polling the DB
struct AssignWork;

/// Internal message sent by a worker to the Manager when it finishes
struct WorkerFinished {
    pub worker_id: usize,
    pub channel_id: ChannelId, // Need the channel ID to remove it from active_channels
}

/// an actor to manage channel reindexers
pub struct ChannelReindexerManager {
    s: Arc<ServerStateInner>,
    index_writer: IndexActorRef,
    concurrency_limit: usize,

    workers: Vec<ActorRef<ChannelReindexer>>,
    active_channels: HashSet<ChannelId>,
    idle_workers: Vec<usize>,
}

impl Actor for ChannelReindexerManager {
    type Args = (Arc<ServerStateInner>, IndexActorRef, usize); // (State, Writer, Concurrency)
    type Error = Error;

    async fn on_start(args: Self::Args, actor_ref: kameo::prelude::ActorRef<Self>) -> Result<Self> {
        let (s, index_writer, concurrency_limit) = args;

        let mut workers = Vec::with_capacity(concurrency_limit);
        let mut idle_workers = Vec::with_capacity(concurrency_limit);

        // start up the worker pool
        for worker_id in 0..concurrency_limit {
            let worker = ChannelReindexer {
                s: s.clone(),
                index_writer: index_writer.clone(),
                schema: ContentSchema::default(),
                state: ChannelReindexerState::Idle,
                worker_id,
                manager: actor_ref.clone(),
            };

            workers.push(ChannelReindexer::spawn(worker));
            idle_workers.push(worker_id);
        }

        let polling_actor_ref = actor_ref.clone();
        tokio::spawn(async move {
            // TODO: hoist 10 to new const
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(10));
            loop {
                interval.tick().await;
                let _ = polling_actor_ref.tell(AssignWork).await;
            }
        });

        let _ = actor_ref.tell(AssignWork).await;

        Ok(Self {
            s,
            index_writer,
            concurrency_limit,
            workers,
            active_channels: HashSet::new(),
            idle_workers,
        })
    }

    async fn on_link_died(
        &mut self,
        _actor_ref: kameo::prelude::WeakActorRef<Self>,
        _id: kameo::prelude::ActorId,
        _reason: kameo::prelude::ActorStopReason,
    ) -> Result<ControlFlow<kameo::prelude::ActorStopReason>> {
        // TODO: handle reindexer deaths
        Ok(ControlFlow::Continue(()))
    }
}

impl ChannelReindexerManager {
    /// query the database to find channels in the reindex queue
    async fn assign_work(&mut self) -> Result<()> {
        if self.idle_workers.is_empty() {
            return Ok(());
        }

        let data = self.s.data();
        let queue = data
            .search_reindex_queue_list(self.concurrency_limit as u32)
            .await?;

        for (channel_id, _last_id) in queue {
            if self.idle_workers.is_empty() {
                break;
            }

            if self.active_channels.contains(&channel_id) {
                continue;
            }

            let worker_id = self.idle_workers.pop().unwrap();
            self.active_channels.insert(channel_id);

            if let Some(worker) = self.workers.get(worker_id) {
                let _ = worker.tell(StartReindexing(channel_id)).await;
            }
        }

        Ok(())
    }
}

impl Message<AssignWork> for ChannelReindexerManager {
    type Reply = ();

    async fn handle(&mut self, _msg: AssignWork, _ctx: &mut Context<Self, Self::Reply>) {
        if let Err(e) = self.assign_work().await {
            error!("Failed to find reindex work: {}", e);
        }
    }
}

impl Message<WorkerFinished> for ChannelReindexerManager {
    type Reply = ();

    async fn handle(&mut self, msg: WorkerFinished, _ctx: &mut Context<Self, Self::Reply>) {
        self.active_channels.remove(&msg.channel_id);
        self.idle_workers.push(msg.worker_id);

        if let Err(e) = self.assign_work().await {
            error!("Failed to find reindex work after worker finished: {}", e);
        }
    }
}

/// an actor to backfill messages for a channel from postgres to tantivy
#[derive(Actor)]
pub struct ChannelReindexer {
    s: Arc<ServerStateInner>,
    index_writer: IndexActorRef,
    schema: ContentSchema,
    state: ChannelReindexerState,

    worker_id: usize,
    manager: ActorRef<ChannelReindexerManager>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ChannelReindexerState {
    Idle,
    Active(ChannelId),
}

/// Start backfilling this channel
pub struct StartReindexing(pub ChannelId);

struct ContinueProcessing;

impl ChannelReindexer {
    async fn finish(&mut self) {
        let data = self.s.data();

        let ChannelReindexerState::Active(channel_id) = self.state else {
            unreachable!("finish() is only called when the reindex is active");
        };

        if let Err(err) = data.search_reindex_queue_delete(channel_id).await {
            error!("failed to delete reindex queue: {err}")
        }

        if let Err(err) = self.index_writer.tell(CommitIndex).await {
            warn!("failed to commit: {err}")
        }

        self.state = ChannelReindexerState::Idle;
        info!("reindex complete for channel {}", channel_id);

        let _ = self
            .manager
            .tell(WorkerFinished {
                worker_id: self.worker_id,
                channel_id,
            })
            .await;
    }
}

impl Message<StartReindexing> for ChannelReindexer {
    type Reply = ();

    async fn handle(&mut self, msg: StartReindexing, ctx: &mut Context<Self, Self::Reply>) {
        if self.state != ChannelReindexerState::Idle {
            warn!("channel reindexer already running");
            return;
        }

        let data = self.s.data();
        let channel_id = msg.0;

        self.state = ChannelReindexerState::Active(channel_id);
        info!("Reindexing channel {channel_id}");

        let _ = ctx.actor_ref().tell(ContinueProcessing).await;
    }
}

impl Message<ContinueProcessing> for ChannelReindexer {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        _msg: ContinueProcessing,
        ctx: &mut Context<Self, Self::Reply>,
    ) -> Result<()> {
        let srv = self.s.services();
        let data = self.s.data();

        let ChannelReindexerState::Active(channel_id) = self.state else {
            warn!("reindex halted halfway through?");
            self.state = ChannelReindexerState::Idle;
            return Ok(());
        };

        let last_id = data.search_reindex_queue_get(channel_id).await?;

        let messages = srv
            .messages
            .list(
                channel_id,
                None,
                PaginationQuery {
                    // it probably already defaults to nil, but explicitly set just in case
                    from: Some(last_id.unwrap_or(Uuid::nil().into())),
                    to: None,
                    dir: Some(PaginationDirection::F),
                    limit: Some(1024),
                },
            )
            .await?;

        let Some(last_id) = messages.items.last().map(|i| i.id) else {
            self.finish().await;
            return Ok(());
        };

        let chan = srv.channels.get(channel_id, None).await?;

        for message in messages.items {
            let doc =
                tantivy_document_from_message(&self.schema, message, chan.room_id, chan.parent_id);

            let _ = self.index_writer.tell(AddDocument(doc)).await;
        }

        if messages.has_more {
            data.search_reindex_queue_upsert(channel_id, Some(last_id))
                .await?;

            let actor_ref = ctx.actor_ref().to_owned();

            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                let _ = actor_ref.tell(ContinueProcessing).await;
            });
        } else {
            self.finish().await;
        }

        Ok(())
    }
}
