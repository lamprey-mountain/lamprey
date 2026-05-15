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

const INTERACTION_LIFETIME: Duration = Duration::from_secs(30);

pub struct ServiceInteractions {
    state: Arc<ServerStateInner>,
    interactions: DashMap<InteractionId, InteractionEntry>,
    interaction_nonce_to_id: DashMap<String, InteractionId>,
}

struct InteractionEntry {
    expire_handle: JoinHandle<Result<()>>,
    nonce: Option<String>,
    interaction: Interaction,
}

impl ServiceInteractions {
    pub fn create(state: Arc<ServerStateInner>) -> Self {
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

        let id_copy = id;
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
                    let room = channel
                        .room_id
                        .map(|room_id| srv.rooms.get(room_id, Some(user_id)))
                        .transpose()
                        .await?;
                    let message = srv
                        .messages
                        .get(channel_id, message_id, Some(user_id))
                        .await?;
                    let user = srv.users.get(user_id, Some(user_id)).await?;
                    let room_member = room
                        .as_ref()
                        .map(|room| room.id)
                        .map(|room_id| srv.state.data().room_member_get(room_id, user_id))
                        .transpose()
                        .await?
                        .flatten();
                    let user_permissions: Vec<Permission> = srv
                        .perms
                        .for_channel(user_id, channel_id)
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
                        application_permissions: todo!(),
                        custom_id,
                    }
                }
            },
        };

        self.state.broadcast(MessageSync::InteractionCreate {
            interaction: inter.clone(),
            user_id,
            nonce: nonce.clone(),
        });

        let expire_handle = tokio::spawn(async move {
            tokio::time::sleep(INTERACTION_LIFETIME).await;
            srv.interactions
                .fail(id_copy, InteractionErrorCode::Timeout);
            Result::Ok(())
        });

        let entry = InteractionEntry {
            interaction: inter.clone(),
            expire_handle,
            nonce: nonce.clone(),
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

    pub fn respond(
        &self,
        id: InteractionId,
        token: String,
        respond: InteractionResponseCreate,
    ) -> Result<InteractionResponse> {
        self.interactions.get(&id);

        // TODO: verify token
        // return Err(Error::BadStatic("invalid token"));

        // TODO: remove interaction if it exists

        match respond.ty {
            InteractionResponseCreateType::Pong => todo!(),
            InteractionResponseCreateType::Reply { message } => {
                todo!()
            }
            InteractionResponseCreateType::ReplyDefer => todo!(),
            InteractionResponseCreateType::MessageUpdate { patch } => todo!(),
            InteractionResponseCreateType::Defer => todo!(),
            InteractionResponseCreateType::Unfurl {
                include_default,
                embeds,
            } => todo!(),
        }

        self.state.broadcast(MessageSync::InteractionSuccess {
            interaction_id: i.id,
            nonce: i.nonce,
        });

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

        self.state.broadcast(MessageSync::InteractionFailure {
            interaction_id: i.id,
            nonce: i.nonce,
            error_code,
        });

        Ok(())
    }

    fn remove(&self, id: InteractionId) -> Option<InteractionEntry> {
        let it = self.interactions.remove(id);
        if let Some(nonce) = it.as_ref().map(|(_, i)| i.nonce) {
            self.interaction_nonce_to_id.remove(nonce);
        }

        it.map(|i| i.1)
    }
}
