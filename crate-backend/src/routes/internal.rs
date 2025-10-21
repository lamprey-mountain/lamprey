use std::{collections::HashSet, sync::Arc};

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use common::v1::types::{
    voice::{SfuCommand, SfuEvent, SignallingMessage},
    ChannelId,
};
use common::v1::types::{MessageSync, SfuId};
use http::{HeaderMap, StatusCode};
use tokio::select;
use tracing::{debug, error, warn};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::error::Result;
use crate::{Error, ServerState};

// NOTE: does the backend<->sfu protocol count as an implementation detail, or
// should it be moved to common?

/// Internal rpc
#[utoipa::path(
    get,
    path = "/internal/rpc",
    tags = ["internal"],
    responses((status = 101, description = "Switching Protocols")),
)]
async fn internal_rpc(
    ws: WebSocketUpgrade,
    headers: HeaderMap,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let auth = headers
        .get("authorization")
        .ok_or(Error::MissingAuth)?
        .to_str()?;
    if auth != format!("Server {}", s.config.sfu_token) {
        return Err(Error::MissingAuth);
    }

    Ok(ws.on_upgrade(move |socket| SfuConnection::new(s).spawn(socket)))
}

/// a websocket connection to a selective forwarding unit
#[derive(Clone)]
struct SfuConnection {
    id: SfuId,
    s: Arc<ServerState>,
}

impl SfuConnection {
    fn new(s: Arc<ServerState>) -> Self {
        Self {
            s,
            id: SfuId::new(),
        }
    }

    async fn spawn(self, mut socket: WebSocket) {
        self.s.sfus.insert(self.id, ());

        let mut outbox = self.s.sushi_sfu.subscribe();

        if let Err(e) = self
            .handle_command(SfuCommand::Ready { sfu_id: self.id }, &mut socket)
            .await
        {
            error!("Failed to send message to websocket: {:?}", e);
            self.shutdown().await;
            return;
        }

        loop {
            select! {
                msg = outbox.recv() => {
                    if let Err(e) = self.handle_command(msg.unwrap(), &mut socket).await {
                        error!("Failed to send message to websocket: {:?}", e);
                        break;
                    }
                }
                msg = socket.recv() => {
                    if let Some(Ok(Message::Text(text))) = msg {
                        if let Ok(json) = serde_json::from_str::<SfuEvent>(&text) {
                            if let Err(e) = self.handle_event(json).await {
                                error!("Error processing SFU command: {:?}", e);
                            }
                        }
                    } else {
                        break;
                    }
                }
            }
        }

        self.shutdown().await;
    }

    async fn shutdown(&self) {
        // when this sfu gets shut down all clients connected to it need to reconnect
        self.s.sfus.remove(&self.id);
        let mut needs_reconnect = HashSet::new();
        self.s.thread_to_sfu.retain(|thread_id, sfu_id| {
            if sfu_id == &self.id {
                needs_reconnect.insert(*thread_id);
                false
            } else {
                true
            }
        });
        for thread_id in &needs_reconnect {
            if self.s.alloc_sfu(*thread_id).await.is_err() {
                warn!("no sfu exists");
                // clients will be told to reconnect anyways to trigger a client error
            }
        }
        for state in self.s.services.users.voice_states_list() {
            if needs_reconnect.contains(&state.thread_id) {
                if let Err(err) = self.s.broadcast(MessageSync::VoiceDispatch {
                    user_id: state.user_id,
                    payload: SignallingMessage::Reconnect,
                }) {
                    error!("failed to broadcast reconnect {err}");
                };
            }
        }
    }

    async fn handle_event(&self, json: SfuEvent) -> Result<()> {
        match json {
            SfuEvent::VoiceDispatch { user_id, payload } => self
                .s
                .broadcast(MessageSync::VoiceDispatch { user_id, payload })?,
            SfuEvent::VoiceState {
                user_id,
                old,
                state,
            } => {
                debug!("change voice state {user_id} {old:?} {state:?}");
                let srv = self.s.services();
                if let Some(state) = &state {
                    srv.users.voice_state_put(state.clone());
                } else {
                    srv.users.voice_state_remove(&user_id);
                }

                self.s.broadcast(MessageSync::VoiceState {
                    user_id,
                    state,
                    old_state: old,
                })?;
            }
        }

        Ok(())
    }

    async fn handle_command(&self, msg: SfuCommand, socket: &mut WebSocket) -> Result<()> {
        let should_send = match &msg {
            SfuCommand::Ready { .. } => true,
            SfuCommand::Signalling { user_id, inner: _ } => {
                let state = self.s.services.users.voice_state_get(*user_id);
                state.is_some_and(|s| self.is_ours(s.thread_id))
            }
            SfuCommand::VoiceState {
                user_id,
                state,
                permissions: _,
            } => {
                let old = self.s.services.users.voice_state_get(*user_id);
                let old_is_ours = old.is_some_and(|s| self.is_ours(s.thread_id));
                let new_is_ours = state.as_ref().is_some_and(|s| self.is_ours(s.thread_id));
                old_is_ours || new_is_ours
            }
            SfuCommand::Thread { thread } => self.is_ours(thread.id),
        };

        if should_send {
            let stringified = serde_json::to_string(&msg).unwrap();
            socket.send(Message::text(stringified)).await?;
        }

        Ok(())
    }

    /// if this thread is managed by us
    fn is_ours(&self, thread_id: ChannelId) -> bool {
        self.s.thread_to_sfu.get(&thread_id).map(|i| *i) == Some(self.id)
    }
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new().routes(routes!(internal_rpc))
}
