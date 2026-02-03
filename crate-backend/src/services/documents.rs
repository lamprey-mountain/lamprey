use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use common::v1::types::document::serialized::Serdoc;
use common::v1::types::document::{Changeset, DocumentTag, HistoryParams};
use common::v1::types::{
    document::{DocumentStateVector, DocumentUpdate},
    ConnectionId, ChannelId, DocumentBranchId, MessageSync, UserId,
};
use dashmap::DashMap;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use tokio::sync::{broadcast, RwLock};
use tracing::error;
use uuid::Uuid;
use yrs::updates::encoder::Encode;
use yrs::DeepObservable;
use yrs::{updates::decoder::Decode, Doc, ReadTxn, StateVector, Transact, Update};

use crate::{Error, Result, ServerStateInner};

// mod validate;
mod serdoc;

const DOCUMENT_ROOT_NAME: &'static str = "doc";

pub type EditContextId = (ChannelId, DocumentBranchId);

pub struct ServiceDocuments {
    state: Arc<ServerStateInner>,
    edit_contexts: DashMap<EditContextId, Arc<RwLock<EditContext>>>,
}

#[derive(Clone, Debug)]
pub enum DocumentEvent {
    Update {
        origin_conn_id: Option<ConnectionId>,
        update: Vec<u8>,
    },
    Presence {
        user_id: UserId,
        origin_conn_id: Option<ConnectionId>,
        cursor_head: String,
        cursor_tail: Option<String>,
    },
}

#[derive(Clone, Debug)]
struct PresenceData {
    conn_id: ConnectionId,
    cursor_head: String,
    cursor_tail: Option<String>,
}

pub struct EditContext {
    /// the live crdt document
    doc: Doc,

    /// the number of changes since the last snapshot
    changes_since_last_snapshot: u64,

    /// changes that have not been persisted yet
    pending_changes: Vec<PendingChange>,

    /// the sequence number of the last persisted update or snapshot
    last_seq: u32,

    update_tx: broadcast::Sender<DocumentEvent>,

    presence: HashMap<UserId, PresenceData>,

    last_snapshot: Instant,
    last_flush: Instant,
    last_active: Instant,
}

struct PendingChange {
    author_id: UserId,
    change: Vec<u8>,
    stat_added: u32,
    stat_removed: u32,
}

pub struct HistoryPaginationSummary {
    pub changesets: Vec<Changeset>,
    pub tags: Vec<DocumentTag>,
}

impl HistoryPaginationSummary {
    /// get a list of all referenced users
    pub fn user_ids(&self) -> Vec<UserId> {
        let mut ids = std::collections::HashSet::new();
        for cs in &self.changesets {
            for author in &cs.authors {
                ids.insert(*author);
            }
        }
        ids.into_iter().collect()
    }
}

// TODO: better error handling (add yrs errors to to crate::Error)
impl ServiceDocuments {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            edit_contexts: DashMap::new(),
        }
    }

    pub fn start_background_tasks(&self) {
        let state = self.state.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let services = state.services();
                let documents = &services.documents;

                let mut to_unload = Vec::new();
                for entry in documents.edit_contexts.iter() {
                    let (id, ctx) = entry.pair();
                    if ctx.read().await.should_unload() {
                        to_unload.push(*id);
                    }
                }

                for id in to_unload {
                    if let Err(e) = documents.unload(id).await {
                        error!("failed to unload document {:?}: {}", id, e);
                    }
                }
            }
        });
    }

    /// load a document. reads from postgres if its not already in memory
    pub async fn load(
        &self,
        context_id: EditContextId,
        maybe_author: Option<UserId>,
    ) -> Result<Arc<RwLock<EditContext>>> {
        if let Some(ctx) = self.edit_contexts.get(&context_id) {
            return Ok(Arc::clone(&ctx));
        }

        let data = self.state.data();
        let loaded = data.document_load(context_id).await;

        let ctx = match loaded {
            Ok(dehydrated) => {
                // load an existing document
                let doc = Doc::new();
                doc.get_or_insert_xml_fragment(DOCUMENT_ROOT_NAME);
                let mut tx = doc.transact_mut();

                let snapshot = Update::decode_v1(&dehydrated.last_snapshot)?;
                tx.apply_update(snapshot)?;

                for change in &dehydrated.changes {
                    let update = Update::decode_v1(change)?;
                    tx.apply_update(update)?;
                }
                drop(tx);

                let (update_tx, _) = broadcast::channel(100);

                Arc::new(RwLock::new(EditContext {
                    doc,
                    changes_since_last_snapshot: dehydrated.changes.len() as u64,
                    pending_changes: vec![],
                    last_seq: dehydrated.snapshot_seq,
                    update_tx,
                    presence: HashMap::new(),
                    last_snapshot: Instant::now(),
                    last_flush: Instant::now(),
                    last_active: Instant::now(),
                }))
            }
            Err(Error::NotFound) => {
                if let Some(author_id) = maybe_author {
                    let doc = Doc::new();
                    doc.get_or_insert_xml_fragment(DOCUMENT_ROOT_NAME);

                    let snapshot = doc
                        .transact()
                        .encode_state_as_update_v1(&StateVector::default());

                    data.document_create(context_id, author_id, snapshot)
                        .await?;

                    let (update_tx, _) = broadcast::channel(100);

                    Arc::new(RwLock::new(EditContext {
                        doc,
                        changes_since_last_snapshot: 0,
                        pending_changes: vec![],
                        last_seq: 0,
                        update_tx,
                        presence: HashMap::new(),
                        last_snapshot: Instant::now(),
                        last_flush: Instant::now(),
                        last_active: Instant::now(),
                    }))
                } else {
                    return Err(Error::NotFound);
                }
            }
            Err(e) => return Err(e),
        };

        match self.edit_contexts.entry(context_id) {
            dashmap::Entry::Occupied(o) => Ok(Arc::clone(o.get())),
            dashmap::Entry::Vacant(v) => {
                v.insert(Arc::clone(&ctx));
                Ok(ctx)
            }
        }
    }

    /// unload a document from memory
    // TODO: automatically unload unused documents
    pub async fn unload(&self, context_id: EditContextId) -> Result<()> {
        if let Some((_, ctx)) = self.edit_contexts.remove(&context_id) {
            let mut ctx = ctx.write().await;
            let data = self.state.data();

            // flush changes
            let changes: Vec<_> = ctx.pending_changes.drain(..).collect();
            for change in changes {
                let new_seq = data
                    .document_update(
                        context_id,
                        change.author_id,
                        change.change,
                        change.stat_added,
                        change.stat_removed,
                    )
                    .await?;
                ctx.last_seq = new_seq;
            }

            // snapshot if needed
            if ctx.changes_since_last_snapshot > 0 {
                let snapshot = ctx
                    .doc
                    .transact()
                    .encode_state_as_update_v1(&StateVector::default());
                let snapshot_id = Uuid::now_v7();
                let seq = ctx.last_seq;

                data.document_compact(context_id, snapshot_id, seq, snapshot)
                    .await?;
            }
        }

        Ok(())
    }

    /// unload all documents, for shutting down
    pub async fn unload_all(&self) {
        let mut futures = FuturesUnordered::new();

        for ctx in &self.edit_contexts {
            let context_id = ctx.key();
            futures.push(self.unload(*context_id));
        }

        while let Some(r) = futures.next().await {
            if let Err(err) = r {
                error!("failed to unload document: {err:?}");
            }
        }
    }

    /// apply a change to a document
    pub async fn apply_update(
        &self,
        context_id: EditContextId,
        author_id: UserId,
        origin_conn_id: Option<ConnectionId>,
        update_bytes: &[u8],
    ) -> Result<()> {
        let update = Update::decode_v1(update_bytes)?;
        let ctx = self.load(context_id, Some(author_id)).await?;
        let mut ctx = ctx.write().await;

        // use a std mutex since i'm not using any async stuff here?
        let stats = Arc::new(std::sync::Mutex::new((0, 0)));
        let stats_inner = stats.clone();

        let xml = ctx.doc.get_or_insert_xml_fragment(DOCUMENT_ROOT_NAME);
        let _sub = xml.observe_deep(move |txn, events| {
            let mut stats = stats_inner.lock().unwrap();
            for e in events.iter() {
                match e {
                    yrs::types::Event::Text(text_event) => {
                        for change in text_event.delta(txn) {
                            match change {
                                yrs::types::Delta::Inserted(t, _) => {
                                    if let yrs::Out::Any(yrs::Any::String(s)) = t {
                                        stats.0 += s.chars().count();
                                    }
                                }
                                yrs::types::Delta::Deleted(len) => stats.1 += *len as usize,
                                yrs::types::Delta::Retain(_, _) => {}
                            }
                        }
                    }
                    yrs::types::Event::XmlText(text_event) => {
                        for change in text_event.delta(txn) {
                            match change {
                                yrs::types::Delta::Inserted(t, _) => {
                                    if let yrs::Out::Any(yrs::Any::String(s)) = t {
                                        stats.0 += s.chars().count();
                                    }
                                }
                                yrs::types::Delta::Deleted(len) => stats.1 += *len as usize,
                                yrs::types::Delta::Retain(_, _) => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        let mut txn = ctx.doc.transact_mut();
        txn.apply_update(update)?;
        drop(txn);
        drop(_sub);

        let (stat_inserted, stat_deleted) = {
            let s = stats.lock().unwrap();
            (s.0 as u32, s.1 as u32)
        };

        ctx.last_active = Instant::now();
        ctx.changes_since_last_snapshot += 1;
        ctx.pending_changes.push(PendingChange {
            author_id,
            change: update_bytes.to_vec(),
            stat_added: stat_inserted,
            stat_removed: stat_deleted,
        });

        let data = self.state.data();

        if ctx.should_flush() {
            let changes: Vec<_> = ctx.pending_changes.drain(..).collect();
            for change in changes {
                let new_seq = data
                    .document_update(
                        context_id,
                        change.author_id,
                        change.change,
                        change.stat_added,
                        change.stat_removed,
                    )
                    .await?;
                ctx.last_seq = new_seq;
            }
            ctx.last_flush = Instant::now();
        }

        if ctx.should_snapshot() {
            let snapshot = ctx
                .doc
                .transact()
                .encode_state_as_update_v1(&StateVector::default());
            let snapshot_id = Uuid::now_v7();
            let seq = ctx.last_seq;

            data.document_compact(context_id, snapshot_id, seq, snapshot)
                .await?;
            ctx.changes_since_last_snapshot = 0;
            ctx.last_snapshot = Instant::now();
        }

        let _ = ctx.update_tx.send(DocumentEvent::Update {
            origin_conn_id,
            update: update_bytes.to_vec(),
        });

        Ok(())
    }

    pub async fn get_content(&self, context_id: EditContextId) -> Result<Serdoc> {
        let ctx = self.load(context_id, None).await?;
        let ctx = ctx.read().await;
        Ok(serdoc::doc_to_serdoc(&ctx.doc))
    }

    pub async fn set_content(
        &self,
        context_id: EditContextId,
        author_id: UserId,
        content: Serdoc,
    ) -> Result<()> {
        let ctx = self.load(context_id, Some(author_id)).await?;
        let mut ctx = ctx.write().await;

        let stats = Arc::new(std::sync::Mutex::new((0, 0)));
        let stats_inner = stats.clone();

        let update_out = Arc::new(std::sync::Mutex::new(Vec::new()));
        let update_out_inner = update_out.clone();

        let xml = ctx.doc.get_or_insert_xml_fragment(DOCUMENT_ROOT_NAME);

        let _sub_stats = xml.observe_deep(move |txn, events| {
            let mut stats = stats_inner.lock().unwrap();
            for e in events.iter() {
                match e {
                    yrs::types::Event::Text(text_event) => {
                        for change in text_event.delta(txn) {
                            match change {
                                yrs::types::Delta::Inserted(t, _) => {
                                    if let yrs::Out::Any(yrs::Any::String(s)) = t {
                                        stats.0 += s.chars().count();
                                    }
                                }
                                yrs::types::Delta::Deleted(len) => stats.1 += *len as usize,
                                yrs::types::Delta::Retain(_, _) => {}
                            }
                        }
                    }
                    yrs::types::Event::XmlText(text_event) => {
                        for change in text_event.delta(txn) {
                            match change {
                                yrs::types::Delta::Inserted(t, _) => {
                                    if let yrs::Out::Any(yrs::Any::String(s)) = t {
                                        stats.0 += s.chars().count();
                                    }
                                }
                                yrs::types::Delta::Deleted(len) => stats.1 += *len as usize,
                                yrs::types::Delta::Retain(_, _) => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
        });

        let _sub_update = ctx.doc.observe_update_v1(move |_, event| {
            let mut u = update_out_inner.lock().unwrap();
            *u = event.update.to_vec();
        });

        serdoc::serdoc_apply_to_doc(&ctx.doc, &content);

        drop(_sub_stats);
        drop(_sub_update);

        let (stat_inserted, stat_deleted) = {
            let s = stats.lock().unwrap();
            (s.0 as u32, s.1 as u32)
        };

        let update_bytes = {
            let u = update_out.lock().unwrap();
            u.clone()
        };

        if update_bytes.is_empty() {
            return Ok(());
        }

        ctx.last_active = Instant::now();
        ctx.changes_since_last_snapshot += 1;
        ctx.pending_changes.push(PendingChange {
            author_id,
            change: update_bytes.clone(),
            stat_added: stat_inserted,
            stat_removed: stat_deleted,
        });

        let data = self.state.data();

        if ctx.should_flush() {
            let changes: Vec<_> = ctx.pending_changes.drain(..).collect();
            for change in changes {
                let new_seq = data
                    .document_update(
                        context_id,
                        change.author_id,
                        change.change,
                        change.stat_added,
                        change.stat_removed,
                    )
                    .await?;
                ctx.last_seq = new_seq;
            }
            ctx.last_flush = Instant::now();
        }

        if ctx.should_snapshot() {
            let snapshot = ctx
                .doc
                .transact()
                .encode_state_as_update_v1(&StateVector::default());
            let snapshot_id = Uuid::now_v7();
            let seq = ctx.last_seq;

            data.document_compact(context_id, snapshot_id, seq, snapshot)
                .await?;
            ctx.changes_since_last_snapshot = 0;
            ctx.last_snapshot = Instant::now();
        }

        let _ = ctx.update_tx.send(DocumentEvent::Update {
            origin_conn_id: None,
            update: update_bytes,
        });

        Ok(())
    }

    pub async fn broadcast_presence(
        &self,
        context_id: EditContextId,
        user_id: UserId,
        origin_conn_id: Option<ConnectionId>,
        cursor_head: String,
        cursor_tail: Option<String>,
    ) -> Result<()> {
        if let Some(ctx) = self.edit_contexts.get(&context_id) {
            let mut ctx = ctx.write().await;
            if let Some(conn_id) = origin_conn_id {
                ctx.presence.insert(
                    user_id,
                    PresenceData {
                        conn_id,
                        cursor_head: cursor_head.clone(),
                        cursor_tail: cursor_tail.clone(),
                    },
                );
                ctx.last_active = Instant::now();
            }
            let _ = ctx.update_tx.send(DocumentEvent::Presence {
                user_id,
                origin_conn_id,
                cursor_head,
                cursor_tail,
            });
        }
        Ok(())
    }

    pub async fn remove_presence(
        &self,
        context_id: EditContextId,
        user_id: UserId,
        conn_id: ConnectionId,
    ) -> Result<()> {
        if let Some(ctx) = self.edit_contexts.get(&context_id) {
            let mut ctx = ctx.write().await;
            if let Some(presence) = ctx.presence.get(&user_id) {
                if presence.conn_id == conn_id {
                    ctx.presence.remove(&user_id);
                    if ctx.presence.is_empty() {
                        ctx.last_active = Instant::now();
                    }
                    let _ = ctx.update_tx.send(DocumentEvent::Presence {
                        user_id,
                        origin_conn_id: Some(conn_id),
                        cursor_head: "".to_string(),
                        cursor_tail: None,
                    });
                }
            }
        }
        Ok(())
    }

    pub async fn get_presence(
        &self,
        context_id: EditContextId,
    ) -> Result<Vec<(UserId, String, Option<String>, ConnectionId)>> {
        let ctx = self.load(context_id, None).await?;
        let ctx = ctx.read().await;
        Ok(ctx
            .presence
            .iter()
            .map(|(uid, data)| {
                (
                    *uid,
                    data.cursor_head.clone(),
                    data.cursor_tail.clone(),
                    data.conn_id,
                )
            })
            .collect())
    }

    pub async fn diff(&self, context_id: EditContextId, state_vector: &[u8]) -> Result<Vec<u8>> {
        let s = StateVector::decode_v1(state_vector)?;
        let ctx = self.load(context_id, None).await?;
        let ctx = ctx.read().await;
        let serialized = ctx.doc.transact().encode_diff_v1(&s);
        Ok(serialized)
    }

    pub async fn get_snapshot(&self, context_id: EditContextId) -> Result<Vec<u8>> {
        let ctx = self.load(context_id, None).await?;
        let ctx = ctx.read().await;
        let snapshot = ctx
            .doc
            .transact()
            .encode_state_as_update_v1(&StateVector::default());
        Ok(snapshot)
    }

    pub async fn get_state_vector(&self, context_id: EditContextId) -> Result<Vec<u8>> {
        let ctx = self.load(context_id, None).await?;
        let ctx = ctx.read().await;
        let sv = ctx.doc.transact().state_vector().encode_v1();
        Ok(sv)
    }

    pub async fn subscribe(
        &self,
        context_id: EditContextId,
    ) -> Result<broadcast::Receiver<DocumentEvent>> {
        let ctx = self.load(context_id, None).await?;
        let ctx = ctx.read().await;
        Ok(ctx.update_tx.subscribe())
    }

    /// create a new DocumentSyncer for a session
    pub fn create_syncer(&self, conn_id: ConnectionId) -> DocumentSyncer {
        let (query_tx, query_rx) = tokio::sync::watch::channel(None);
        DocumentSyncer {
            s: self.state.clone(),
            query_tx,
            query_rx,
            current_rx: None,
            conn_id,
            pending_sync: VecDeque::new(),
        }
    }

    pub async fn query_history(
        &self,
        context_id: EditContextId,
        query: HistoryParams,
    ) -> Result<HistoryPaginationSummary> {
        let data = self.state.data();
        let (updates, tags) = data.document_history(context_id).await?;

        let by_author = query.by_author.unwrap_or(true);
        let by_tag = query.by_tag.unwrap_or(true);
        let by_time = query.by_time.unwrap_or(3600) as i64;
        let by_changes = query.by_changes.unwrap_or(100) as usize;

        let mut changesets = Vec::new();
        if updates.is_empty() {
            return Ok(HistoryPaginationSummary {
                changesets,
                tags: vec![],
            });
        }

        let mut current_authors = HashSet::new();
        let mut current_added = 0;
        let mut current_removed = 0;
        let mut current_start = updates[0].created_at;
        let mut current_end = updates[0].created_at;
        let mut current_count = 0;

        let mut tag_iter = tags.iter().peekable();

        for (i, update) in updates.iter().enumerate() {
            let mut split = false;

            if i > 0 {
                let prev = &updates[i - 1];

                if by_author && update.user_id != prev.user_id {
                    split = true;
                }

                let diff = (*update.created_at - *prev.created_at).whole_seconds();
                if diff > by_time {
                    split = true;
                }

                if current_count >= by_changes {
                    split = true;
                }

                if by_tag {
                    while let Some(tag) = tag_iter.peek() {
                        if tag.revision_seq < prev.seq as u64 {
                            tag_iter.next();
                            continue;
                        }
                        if tag.revision_seq == prev.seq as u64 {
                            split = true;
                        }
                        break;
                    }
                }
            }

            if split {
                changesets.push(Changeset {
                    start_time: current_start,
                    end_time: current_end,
                    authors: current_authors.drain().collect(),
                    stat_added: current_added,
                    stat_removed: current_removed,
                    document_id: Some(context_id.0),
                });
                current_added = 0;
                current_removed = 0;
                current_count = 0;
                current_start = update.created_at;
            }

            current_authors.insert(UserId::from(update.user_id));
            current_added += update.stat_added as u64;
            current_removed += update.stat_removed as u64;
            current_end = update.created_at;
            current_count += 1;
        }

        changesets.push(Changeset {
            start_time: current_start,
            end_time: current_end,
            authors: current_authors.drain().collect(),
            stat_added: current_added,
            stat_removed: current_removed,
            document_id: Some(context_id.0),
        });

        changesets.reverse();

        if let Some(limit) = query.limit {
            changesets.truncate(limit as usize);
        } else {
            changesets.truncate(20);
        }

        Ok(HistoryPaginationSummary { changesets, tags })
    }
}

pub struct DocumentSyncer {
    s: Arc<ServerStateInner>,
    query_tx: tokio::sync::watch::Sender<Option<(EditContextId, Option<Vec<u8>>)>>,
    query_rx: tokio::sync::watch::Receiver<Option<(EditContextId, Option<Vec<u8>>)>>,
    current_rx: Option<(EditContextId, broadcast::Receiver<DocumentEvent>)>,
    conn_id: ConnectionId,
    pending_sync: VecDeque<MessageSync>,
}

impl DocumentSyncer {
    /// set edit context id for this syncer
    pub async fn set_context_id(
        &self,
        context_id: EditContextId,
        state_vector: Option<DocumentStateVector>,
    ) -> Result<()> {
        self.query_tx
            .send(Some((context_id, state_vector.map(|sv| sv.0))))
            .map_err(|_| Error::Internal("query channel closed".to_string()))?;
        Ok(())
    }

    /// check if client is subscribed to this document
    pub fn is_subscribed(&self, context_id: &EditContextId) -> bool {
        self.query_rx
            .borrow()
            .as_ref()
            .map(|(current_id, _)| current_id == context_id)
            .unwrap_or(false)
    }

    pub async fn handle_disconnect(&self, user_id: UserId) -> Result<()> {
        if let Some((context_id, _)) = &self.current_rx {
            self.s
                .services()
                .documents
                .remove_presence(*context_id, user_id, self.conn_id)
                .await?;
        }
        Ok(())
    }

    pub async fn poll(&mut self) -> Result<MessageSync> {
        loop {
            if let Some(msg) = self.pending_sync.pop_front() {
                return Ok(msg);
            }

            if self.query_rx.has_changed().unwrap_or(false) {
                let _ = self.query_rx.borrow_and_update();
                let query = self.query_rx.borrow().clone();

                match query {
                    Some((context_id, state_vector)) => {
                        let rx = self.s.services().documents.subscribe(context_id).await?;
                        self.current_rx = Some((context_id, rx));

                        let srv = self.s.services();
                        let update = if let Some(sv) = state_vector {
                            srv.documents.diff(context_id, &sv).await?
                        } else {
                            let ctx = srv.documents.load(context_id, None).await?;
                            let ctx = ctx.read().await;
                            let update = ctx
                                .doc
                                .transact()
                                .encode_state_as_update_v1(&StateVector::default());
                            update
                        };

                        let presences = srv.documents.get_presence(context_id).await?;
                        for (user_id, cursor_head, cursor_tail, conn_id) in presences {
                            if conn_id != self.conn_id {
                                self.pending_sync.push_back(MessageSync::DocumentPresence {
                                    channel_id: context_id.0,
                                    branch_id: context_id.1,
                                    user_id,
                                    cursor_head,
                                    cursor_tail,
                                });
                            }
                        }

                        return Ok(MessageSync::DocumentEdit {
                            channel_id: context_id.0,
                            branch_id: context_id.1,
                            update: DocumentUpdate(update),
                        });
                    }
                    None => {
                        self.current_rx = None;
                        continue;
                    }
                }
            }

            if let Some((context_id, rx)) = &mut self.current_rx {
                tokio::select! {
                    res = rx.recv() => {
                        match res {
                            Ok(event) => match event {
                                DocumentEvent::Update { origin_conn_id, update } => {
                                    if origin_conn_id == Some(self.conn_id) {
                                        continue;
                                    }
                                    return Ok(MessageSync::DocumentEdit {
                                        channel_id: context_id.0,
                                        branch_id: context_id.1,
                                        update: DocumentUpdate(update),
                                    });
                                }
                                DocumentEvent::Presence {
                                    user_id,
                                    origin_conn_id,
                                    cursor_head,
                                    cursor_tail,
                                } => {
                                    if origin_conn_id == Some(self.conn_id) {
                                        continue;
                                    }
                                    return Ok(MessageSync::DocumentPresence {
                                        channel_id: context_id.0,
                                        branch_id: context_id.1,
                                        user_id,
                                        cursor_head,
                                        cursor_tail,
                                    });
                                }
                            },
                            Err(_) => continue,
                        }
                    }
                    _ = self.query_rx.changed() => continue,
                }
            } else {
                self.query_rx
                    .changed()
                    .await
                    .map_err(|_| Error::Internal("query channel closed".to_string()))?;
                continue;
            }
        }
    }
}

// TODO: fine tune these numbers
impl EditContext {
    /// whether we should create a new snapshot
    pub fn should_snapshot(&self) -> bool {
        if self.changes_since_last_snapshot > 256 {
            return true;
        } else if self.changes_since_last_snapshot == 0 {
            return false;
        }

        if self.last_snapshot.elapsed() > Duration::from_secs(30) {
            return true;
        }

        if self.presence.is_empty() && self.last_active.elapsed() > Duration::from_secs(15) {
            return true;
        }

        false
    }

    /// whether we should flush pending_changes to db
    pub fn should_flush(&self) -> bool {
        if self.pending_changes.is_empty() {
            return false;
        }

        self.last_flush.elapsed() > Duration::from_secs(10)
    }

    /// whether we should unload this document
    pub fn should_unload(&self) -> bool {
        self.presence.is_empty() && self.last_active.elapsed() > Duration::from_secs(60)
    }
}
