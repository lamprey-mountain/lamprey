use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use common::v1::types::components::ComponentCreate;
use common::v1::types::document::serialized::Serdoc;
use common::v1::types::document::{Changeset, DocumentTag, HistoryParams};
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::{ChannelId, ConnectionId, DocumentBranchId, UserId};
use common::v2::types::DocumentId;
use dashmap::DashMap;
use futures::StreamExt;
use futures::stream::FuturesUnordered;
use kameo::actor::Spawn;
use kerosene_core::types::documents::EditContextId;
use lamprey_backend_data_postgres::DocumentUpdateSummary;
use tokio::sync::broadcast;
use tracing::{debug, error};
use yrs::ReadTxn;
use yrs::{Doc, StateVector, Transact, Update, updates::decoder::Decode};

use crate::prelude::*;
use crate::services::documents::actor::{
    ApplyUpdate, BroadcastPresence, DocumentActor, DocumentHandle, GetDiff, GetSnapshot,
    GetStateVector, PersistAndUnload, PresenceDelete, PresenceGet, SerdocGet, SerdocPut,
    ShouldUnload, Subscribe,
};
use crate::services::documents::syncer::DocumentSyncer;
use crate::services::documents::util::{DOCUMENT_ROOT_NAME, HistoryPaginationSummary};

mod actor;
mod compact;
mod history;
mod serialized;
mod syncer;
mod util;

pub struct ServiceDocuments {
    globals: Globals,
    handles: DashMap<EditContextId, DocumentHandle>,
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

pub use actor::PendingChange;

// TODO: better error handling (add yrs errors to to crate::Error)
impl ServiceDocuments {
    pub fn new(globals: Globals) -> Self {
        Self {
            globals,
            handles: DashMap::new(),
        }
    }

    pub fn start_background_tasks(&self) {
        let state = self.globals.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));
            loop {
                interval.tick().await;
                let services = state.services();
                let documents = &services.documents;

                // capture actors to avoid holding dashmap locks across await points
                let actors: Vec<_> = documents
                    .handles
                    .iter()
                    .map(|entry| (*entry.key(), entry.value().clone()))
                    .collect();

                let mut to_unload = Vec::new();
                for (id, handle) in actors {
                    if let Ok(true) = handle.should_unload().await {
                        to_unload.push(id);
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

    /// get a loaded document
    pub fn get(&self, context_id: EditContextId) -> Option<DocumentHandle> {
        self.handles.get(&context_id).map(|h| h.value().clone())
    }

    /// load a document. reads from postgres if its not already in memory
    pub async fn load(
        &self,
        context_id: EditContextId,
        maybe_author: Option<UserId>,
    ) -> Result<DocumentHandle> {
        if let Some(handle) = self.handles.get(&context_id) {
            return Ok(handle.clone());
        }

        debug!(context_id = ?context_id, maybe_author = ?maybe_author, "load document");
        let mut txn = self.globals.begin().await?;
        let loaded = txn.document_load(context_id).await;

        let actor_ref = match loaded {
            Ok(dehydrated) => {
                // load an existing document
                let doc = Doc::new();

                if context_id.is_prose() {
                    doc.get_or_insert_xml_fragment(DOCUMENT_ROOT_NAME);
                } else {
                    doc.get_or_insert_text(DOCUMENT_ROOT_NAME);
                }

                let mut tx = doc.transact_mut();

                let snapshot = Update::decode_v1(&dehydrated.last_snapshot)?;
                tx.apply_update(snapshot)?;

                for change in &dehydrated.changes {
                    let update = Update::decode_v1(change)?;
                    tx.apply_update(update)?;
                }
                drop(tx);
                txn.commit().await?;

                let (update_tx, _) = broadcast::channel(100);

                let actor = DocumentActor {
                    context_id,
                    state: self.globals.clone(),
                    doc,
                    changes_since_last_snapshot: dehydrated.changes.len() as u64,
                    pending_changes: VecDeque::new(),
                    last_seq: dehydrated.snapshot_seq,
                    update_tx,
                    presence: HashMap::new(),
                    last_snapshot: Instant::now(),
                    last_flush: Instant::now(),
                    last_active: Instant::now(),
                };
                DocumentActor::spawn(actor)
            }
            Err(Error::ApiError(ApiError {
                code: ErrorCode::UnknownDocumentBranch,
                ..
            })) => {
                if let Some(author_id) = maybe_author {
                    let doc = Doc::new();

                    if context_id.is_prose() {
                        doc.get_or_insert_xml_fragment(DOCUMENT_ROOT_NAME);
                    } else {
                        doc.get_or_insert_text(DOCUMENT_ROOT_NAME);
                    }

                    let snapshot = doc
                        .transact()
                        .encode_state_as_update_v1(&StateVector::default());

                    txn.document_create(context_id, author_id, snapshot).await?;
                    txn.commit().await?;

                    let (update_tx, _) = broadcast::channel(100);

                    let actor = DocumentActor {
                        context_id,
                        state: self.globals.clone(),
                        doc,
                        changes_since_last_snapshot: 0,
                        pending_changes: VecDeque::new(),
                        last_seq: 0,
                        update_tx,
                        presence: HashMap::new(),
                        last_snapshot: Instant::now(),
                        last_flush: Instant::now(),
                        last_active: Instant::now(),
                    };
                    DocumentActor::spawn(actor)
                } else {
                    return Err(Error::ApiError(ApiError::from_code(
                        ErrorCode::UnknownDocumentBranch,
                    )));
                }
            }
            Err(e) => return Err(e),
        };

        let handle = DocumentHandle::new(actor_ref);

        match self.handles.entry(context_id) {
            // PERF: avoid double loading document (race condition)
            dashmap::Entry::Occupied(o) => Ok(o.get().clone()),
            dashmap::Entry::Vacant(v) => {
                v.insert(handle.clone());
                Ok(handle)
            }
        }
    }

    /// unload a document from memory
    // TODO: automatically unload unused documents
    pub async fn unload(&self, context_id: EditContextId) -> Result<()> {
        if let Some((_, handle)) = self.handles.remove(&context_id) {
            handle.persist_and_unload().await?;
        }

        Ok(())
    }

    /// unload all documents, for shutting down
    pub async fn unload_all(&self) {
        // collect edit contexts before unloading to prevent deadlock
        let ids: Vec<EditContextId> = self.handles.iter().map(|e| *e.key()).collect();

        let mut futures = FuturesUnordered::new();
        for id in ids {
            futures.push(self.unload(id));
        }

        while let Some(r) = futures.next().await {
            if let Err(err) = r {
                error!("failed to unload document: {err:?}");
            }
        }
    }

    /// apply a change to a document
    #[tracing::instrument(skip(self, update_bytes))]
    pub async fn apply_update(
        &self,
        context_id: EditContextId,
        author_id: UserId,
        origin_conn_id: Option<ConnectionId>,
        update_bytes: &[u8],
    ) -> Result<()> {
        let handle = self.load(context_id, Some(author_id)).await?;
        handle
            .apply_update(author_id, origin_conn_id, update_bytes.to_vec())
            .await
    }

    pub async fn get_content(&self, context_id: EditContextId) -> Result<Serdoc> {
        let handle = self.load(context_id, None).await?;
        handle.serdoc_get().await
    }

    pub async fn get_content_at_seq(&self, context_id: EditContextId, seq: u64) -> Result<Serdoc> {
        let dehydrated = self
            .globals
            .begin_read()
            .await?
            .document_load_at_seq(context_id, seq as u32)
            .await?;

        let doc = yrs::Doc::new();
        doc.get_or_insert_xml_fragment(DOCUMENT_ROOT_NAME);

        let mut txn = doc.transact_mut();

        // start with the last snapshot
        let snapshot = yrs::Update::decode_v1(&dehydrated.last_snapshot)?;
        txn.apply_update(snapshot)?;

        // replay updates
        for update_data in dehydrated.changes {
            let update = yrs::Update::decode_v1(&update_data)?;
            txn.apply_update(update)?;
        }
        drop(txn);

        Ok(serialized::doc_to_serdoc(&doc))
    }

    pub async fn set_content(
        &self,
        context_id: EditContextId,
        author_id: UserId,
        components: Vec<ComponentCreate>,
    ) -> Result<()> {
        let handle = self.load(context_id, Some(author_id)).await?;
        handle.serdoc_put(author_id, components).await
    }

    pub async fn broadcast_presence(
        &self,
        context_id: EditContextId,
        user_id: UserId,
        origin_conn_id: Option<ConnectionId>,
        cursor_head: String,
        cursor_tail: Option<String>,
    ) -> Result<()> {
        if let Some(handle) = self.handles.get(&context_id) {
            let handle = handle.value();
            handle
                .presence_upsert(user_id, origin_conn_id, cursor_head, cursor_tail)
                .await?;
        }

        Ok(())
    }

    pub async fn remove_presence(
        &self,
        context_id: EditContextId,
        user_id: UserId,
        conn_id: ConnectionId,
    ) -> Result<()> {
        if let Some(handle) = self.handles.get(&context_id) {
            let handle = handle.value();
            handle.presence_delete(user_id, conn_id).await?;
        }

        Ok(())
    }

    pub async fn get_presence(
        &self,
        context_id: EditContextId,
    ) -> Result<Vec<(UserId, String, Option<String>, ConnectionId)>> {
        if let Some(handle) = self.handles.get(&context_id) {
            let handle = handle.value();
            handle.presence_list().await
        } else {
            // TODO: return error if document does not exist
            Ok(vec![])
        }
    }

    pub async fn diff(
        &self,
        context_id: EditContextId,
        maybe_author: Option<UserId>,
        state_vector: &[u8],
    ) -> Result<Vec<u8>> {
        let s = if state_vector.is_empty() {
            StateVector::default()
        } else {
            StateVector::decode_v1(state_vector)?
        };
        let handle = self.load(context_id, maybe_author).await?;
        handle.get_diff(s).await
    }

    pub async fn get_snapshot(&self, context_id: EditContextId) -> Result<Vec<u8>> {
        let handle = self.load(context_id, None).await?;
        handle.get_snapshot().await
    }

    pub async fn get_state_vector(&self, context_id: EditContextId) -> Result<Vec<u8>> {
        let handle = self.load(context_id, None).await?;
        handle.get_state_vector().await
    }

    pub async fn subscribe(
        &self,
        context_id: EditContextId,
        maybe_author: Option<UserId>,
    ) -> Result<broadcast::Receiver<DocumentEvent>> {
        let handle = self.load(context_id, maybe_author).await?;
        handle.subscribe().await
    }
}
