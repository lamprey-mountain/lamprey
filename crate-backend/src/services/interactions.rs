use std::sync::Arc;
use std::time::Duration;

use common::v1::types::interactions::{
    Interaction, InteractionCreate, InteractionCreateType, InteractionErrorCode,
    InteractionResponse, InteractionResponseCreate, InteractionResponseCreateType, InteractionType,
};
use common::v1::types::{InteractionId, MessageSync, Permission, UserId};
use dashmap::DashMap;
use lamprey_backend_core::Error;
use lamprey_backend_data_postgres::ApplicationId;
use tokio::task::JoinHandle;

use crate::{Result, ServerStateInner};

/// how long applications have to respond to an interaction
const INTERACTION_LIFETIME: Duration = Duration::from_secs(30);

/// how long applications have to send follow messages after responding to an interaction
const INTERACTION_FOLLOWUP_LIFETIME: Duration = Duration::from_secs(60 * 15);

/// how many followup messages applications can send
const INTERACTION_FOLLOWUP_LIMIT: usize = 10;

pub struct ServiceInteractions {
    state: Arc<ServerStateInner>,

    // TODO: support multiple server instances
    // maybe use nats jetstream or redis or whatever for this
    interactions: DashMap<InteractionId, InteractionEntry>,
    interaction_nonce_to_id: DashMap<String, InteractionId>,
}

struct InteractionEntry {
    nonce: Option<String>,
    interaction: Interaction,
    state: InteractionEntryState,
}

enum InteractionEntryState {
    /// interaction created, waiting for response
    Created {
        expire_handle: JoinHandle<Result<()>>,
    },

    /// interaction responded to, can have followups sent
    Responded {
        expire_handle: JoinHandle<Result<()>>,
        deferred: bool,
    },
}

impl ServiceInteractions {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            interactions: DashMap::new(),
            interaction_nonce_to_id: DashMap::new(),
        }
    }

    pub async fn create(
        &self,
        user_id: UserId,
        nonce: Option<String>,
        create: InteractionCreate,
    ) -> Result<Interaction> {
        let id = InteractionId::new();

        let srv = self.state.services();

        let inter = Interaction {
            id,
            application_id: create.application_id,
            token: Some(uuid::Uuid::new_v4().to_string()),
            version: 1,
            ty: match create.ty {
                InteractionCreateType::Button {
                    channel_id,
                    message_id,
                    custom_id,
                } => {
                    let channel = srv.channels.get(channel_id, Some(user_id)).await?;
                    let room = if let Some(room_id) = channel.room_id {
                        Some(srv.rooms.get(room_id, Some(user_id)).await?)
                    } else {
                        None
                    };
                    let message = srv
                        .messages
                        .get(channel_id, message_id, Some(user_id))
                        .await?;
                    let user = srv.users.get(user_id, Some(user_id)).await?;
                    let room_member = if let Some(room_id) = room.as_ref().map(|r| r.id) {
                        let mut data = srv.state.begin_read().await?;
                        Some(data.room_member_get(room_id, user_id).await?)
                    } else {
                        None
                    };
                    let user_permissions: Vec<Permission> = srv
                        .perms
                        .for_channel(user_id, channel_id)
                        .await?
                        .perms()
                        .into();
                    let application_permissions: Vec<Permission> = srv
                        .perms
                        .for_channel((*create.application_id).into(), channel_id)
                        .await?
                        .perms()
                        .into();

                    InteractionType::Button {
                        room,
                        channel,
                        message,
                        user,
                        room_member,
                        user_permissions,
                        application_permissions,
                        custom_id,
                    }
                }
            },
        };

        self.state.broadcast(MessageSync::InteractionCreate {
            interaction: Box::new(inter.clone()),
            user_id: user_id,
            nonce: nonce.clone(),
        })?;

        let id_copy = id;
        let expire_handle = tokio::spawn(async move {
            tokio::time::sleep(INTERACTION_LIFETIME).await;
            srv.interactions
                .fail(id_copy, InteractionErrorCode::Timeout)?;
            Result::Ok(())
        });

        let entry = InteractionEntry {
            interaction: inter.clone(),
            nonce: nonce.clone(),
            state: InteractionEntryState::Created { expire_handle },
        };
        self.interactions.insert(id, entry);
        if let Some(nonce) = nonce.clone() {
            self.interaction_nonce_to_id.insert(nonce, id);
        }

        Ok(inter)
    }

    pub async fn create_ping(&self, application_id: ApplicationId) -> Result<Interaction> {
        let _inter = Interaction {
            id: InteractionId::new(),
            application_id,
            token: Some(uuid::Uuid::new_v4().to_string()),
            version: 1,
            ty: InteractionType::Ping,
        };

        // TODO: for webhooks, disable them if the Ping times out

        todo!()
    }

    pub async fn respond(
        &self,
        id: InteractionId,
        token: String,
        respond: InteractionResponseCreate,
    ) -> Result<InteractionResponse> {
        let Some((_, entry)) = self.interactions.remove(&id) else {
            // interaction already responded to or expired
            return Err(Error::BadStatic("interaction not found"));
        };

        if entry
            .interaction
            .token
            .as_deref()
            .map(|t| t != &token)
            .unwrap_or(true)
        {
            // token didn't match, put it back
            self.interactions.insert(id, entry);
            return Err(Error::BadStatic("invalid token"));
        }

        let srv = self.state.services();
        let deferred = match respond.ty {
            InteractionResponseCreateType::Pong => return Err(Error::Unimplemented),
            InteractionResponseCreateType::Reply { message } => {
                let channel_id = match &entry.interaction.ty {
                    InteractionType::Button { channel, .. } => channel.id,
                    InteractionType::Ping => {
                        return Err(Error::BadStatic("cannot reply to ping interaction"))
                    }
                    InteractionType::Unfurl { channel, .. } => channel.id,
                };

                let original_message_id = match &entry.interaction.ty {
                    InteractionType::Button { message, .. } => Some(message.id),
                    InteractionType::Ping => unreachable!(),
                    InteractionType::Unfurl { message, .. } => Some(message.id),
                };

                let mut reply_message = message;
                if reply_message.reply_id.is_none() {
                    reply_message.reply_id = original_message_id;
                }

                let user = match &entry.interaction.ty {
                    InteractionType::Button { user, .. } => user.clone(),
                    InteractionType::Ping => unreachable!(),
                    InteractionType::Unfurl { user, .. } => user.clone(),
                };

                let _message = srv
                    .messages
                    .create_as_webhook(channel_id, user.id, reply_message)
                    .await?;

                // TODO: return message

                false
            }
            InteractionResponseCreateType::MessageUpdate { patch } => {
                let channel_id = match &entry.interaction.ty {
                    InteractionType::Button { channel, .. } => channel.id,
                    InteractionType::Ping => {
                        return Err(Error::BadStatic("cannot edit message in ping interaction"))
                    }
                    InteractionType::Unfurl { channel, .. } => channel.id,
                };

                let message_id = match &entry.interaction.ty {
                    InteractionType::Button { message, .. } => message.id,
                    InteractionType::Ping => unreachable!(),
                    InteractionType::Unfurl { message, .. } => message.id,
                };

                let webhook_user_id = match &entry.interaction.ty {
                    InteractionType::Button { user, .. } => user.id,
                    InteractionType::Ping => unreachable!(),
                    InteractionType::Unfurl { user, .. } => user.id,
                };

                let (_, _message) = srv
                    .messages
                    .edit_as_webhook(channel_id, message_id, webhook_user_id, patch)
                    .await?;

                false
            }
            // InteractionResponseCreateType::ReplyDefer => true,
            // InteractionResponseCreateType::Defer => true,
            InteractionResponseCreateType::ReplyDefer => return Err(Error::Unimplemented),
            InteractionResponseCreateType::Defer => return Err(Error::Unimplemented),
            InteractionResponseCreateType::Unfurl { .. } => return Err(Error::Unimplemented),
        };

        let interaction_user_id = match &entry.interaction.ty {
            InteractionType::Button { user, .. } => user.id,
            InteractionType::Ping => {
                return Err(Error::BadStatic(
                    "probably should design types to avoid this",
                ))
            }
            InteractionType::Unfurl { user, .. } => user.id,
        };

        let nonce = entry.nonce.clone();
        self.state.broadcast(MessageSync::InteractionSuccess {
            user_id: interaction_user_id,
            interaction_id: entry.interaction.id,
            nonce,
        })?;

        let id_copy = id;
        let expire_handle = tokio::spawn(async move {
            tokio::time::sleep(INTERACTION_FOLLOWUP_LIFETIME).await;
            srv.interactions.remove(id_copy);
            Result::Ok(())
        });

        self.interactions.insert(
            id,
            InteractionEntry {
                nonce: entry.nonce,
                interaction: entry.interaction,
                state: InteractionEntryState::Responded {
                    expire_handle,
                    deferred,
                },
            },
        );

        let resp = InteractionResponse {
            // nothing yet...
        };

        Ok(resp)
    }

    fn fail(&self, id: InteractionId, error_code: InteractionErrorCode) -> Result<()> {
        let Some(i) = self.remove(id) else {
            // probably already responded
            return Ok(());
        };

        let interaction_user_id = match &i.interaction.ty {
            InteractionType::Button { user, .. } => user.id,
            InteractionType::Ping => return Err(Error::BadStatic("what do i do here?")),
            InteractionType::Unfurl { user, .. } => user.id,
        };

        self.state.broadcast(MessageSync::InteractionFailure {
            user_id: interaction_user_id,
            interaction_id: i.interaction.id,
            nonce: i.nonce,
            error_code,
        })?;

        Ok(())
    }

    fn remove(&self, id: InteractionId) -> Option<InteractionEntry> {
        let it = self.interactions.remove(&id);
        if let Some(nonce) = it.as_ref().and_then(|(_, i)| i.nonce.as_ref()) {
            self.interaction_nonce_to_id.remove(nonce);
        }

        it.map(|i| i.1)
    }
}
