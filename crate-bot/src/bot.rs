use crate::database::BotDatabase;
use crate::prelude::*;
use crate::{commands::Command, config::Config};
use clap::Parser;
use common::v1::types::{
    Message, MessageClient,
    presence::{Activity, Presence, Status},
};
use common::v1::types::{MessageCreate, MessageSync};
use common::v1::types::{MessagePayload, MessageType};
use futures::StreamExt;
use sdk::Client;
use sdk::syncer::SyncerEvent;
use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

pub struct Bot {
    pub(crate) client: Client,
    pub(crate) db: BotDatabase,
    control_rx: mpsc::Receiver<BotCommand>,
    control_tx: mpsc::Sender<BotCommand>,
    shutdown_token: CancellationToken,
}

pub struct BotHandle {
    control: mpsc::Sender<BotCommand>,
    shutdown_token: CancellationToken,
}

pub enum BotCommand {
    Send(MessageClient),
}

impl Bot {
    /// initialize a bot from some config
    pub async fn from_config(config: Config) -> Result<Self> {
        let pool = SqlitePool::connect(&config.database_url).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        let db = BotDatabase::new(pool.clone());

        let (control_tx, control_rx) = mpsc::channel(100);
        let shutdown_token = CancellationToken::new();

        let mut client = Client::builder()
            .token(config.token.clone().into())
            .presence(Presence {
                status: Status::Online,
                activities: vec![Activity::Custom {
                    text: "Hello, world!".to_string(), // TODO: make configurable
                    clear_at: None,
                }],
            });

        if let Some(api_url) = config.api_url.clone() {
            client = client.api_url(api_url);
        }

        if let Some(cdn_url) = config.cdn_url.clone() {
            client = client.cdn_url(cdn_url);
        }

        if let Some(sync_url) = config.sync_url.clone() {
            client = client.sync_url(sync_url);
        }

        let client = client.build().await?;

        Ok(Self {
            client,
            db,
            control_rx,
            control_tx,
            shutdown_token,
        })
    }

    /// get a handle to control the bot with
    pub fn handle(&self) -> BotHandle {
        BotHandle {
            control: self.control_tx.clone(),
            shutdown_token: self.shutdown_token.clone(),
        }
    }

    /// start the bot
    pub async fn start(mut self) {
        info!("Bot started");

        let syncer = self.client.syncer();
        let jh = tokio::spawn(async move {
            let mut sub = self.client.syncer().subscribe();
            loop {
                tokio::select! {
                    _ = self.shutdown_token.cancelled() => {
                        info!("Shutdown signal received");
                        drop(self.control_tx); // Drop to allow control_rx to close
                        return;
                    }
                    event = sub.next() => {
                        if let Some(event) = event {
                            self.handle_event(&event).await;
                        } else {
                            warn!("Syncer stream closed unexpectedly");
                            break;
                        }
                    }
                    command = self.control_rx.recv() => {
                        if let Some(command) = command {
                            self.handle_control(&command).await;
                        } else {
                            warn!("Control channel closed unexpectedly");
                            break;
                        }
                    }
                }
            }
        });

        syncer.connect();
        jh.await.unwrap();
    }

    async fn handle_event(&mut self, event: &SyncerEvent) {
        match event {
            SyncerEvent::Message(msg) => match &msg.payload {
                MessagePayload::Ready { user, .. } => {
                    if let Some(user) = user {
                        info!("logged in as {}!", user.name);
                    } else {
                        error!("no user for this token!");
                        // TODO: handle this better
                        std::process::exit(1);
                    }
                }
                _ => {}
            },
            SyncerEvent::Sync(sync) => match &**sync {
                MessageSync::Ambient { .. } => {
                    // TODO: do something with this data?
                }
                MessageSync::MessageCreate { message } => {
                    if let Err(e) = self.handle_message(message.clone()).await {
                        error!("failed to handle message: {e}");
                    }
                }
                MessageSync::VoiceState {
                    user_id,
                    state,
                    old_state: _,
                } => {
                    debug!(%user_id, "got voice state: {state:?}");
                    // if let Some(user) = &self.user {
                    //     if user.id == user_id && state == None {
                    //         if let Some(p) = &*self.player.lock().await {
                    //             p.send(PlayerCommand::Stop).await?;
                    //         }
                    //     }
                    // };

                    // self.voice_states.retain(|s| s.user_id != user_id);
                    // if let Some(state) = state {
                    //     self.voice_states.push(state);
                    // }
                    // TODO: handle this
                }
                MessageSync::VoiceDispatch {
                    user_id,
                    channel_id,
                    payload,
                } => {
                    debug!(%channel_id, "got voice dispatch: {payload:?}");
                    // if let Some(p) = &*self.player.lock().await {
                    //     p.send(PlayerCommand::Signalling(payload)).await?;
                    // }
                    // TODO: handle this
                }
                _ => {}
            },
            SyncerEvent::StateChanged => {
                debug!("State changed");
            }
        }
    }

    async fn handle_control(&mut self, command: &BotCommand) {
        match command {
            BotCommand::Send(msg) => {
                // TODO: implement sending message
                debug!("received send command: {msg:?}");
            }
        }
    }

    async fn handle_message(&mut self, message: Message) -> Result<()> {
        let content = match &message.latest_version.message_type {
            MessageType::DefaultMarkdown(m) => m.content.as_deref(),
            _ => None,
        };

        if let Some(content) = content {
            debug!("message from {}: {}", message.author_id, content);
        } else {
            debug!("message from {} without content", message.author_id);
        }

        if let Some(command) = content.and_then(|c: &str| c.strip_prefix("!")) {
            debug!("got raw command {command:?}");
            let command = Command::try_parse_from(
                std::iter::once("bot".to_string())
                    .chain(command.split_whitespace().map(|s| s.to_string())),
            );
            let resp = match command {
                Ok(command) => {
                    debug!("got command {command:?}");
                    match self.handle_command(&message, command).await {
                        Ok(s) => s,
                        Err(e) => e.to_string(),
                    }
                }
                Err(err) => err.to_string(),
            };
            let resp = MessageCreate {
                content: Some(resp),
                ..Default::default()
            };
            self.client
                .http()
                .message_create(message.channel_id, &resp)
                .await?;
        }

        Ok(())
    }
}

impl BotHandle {
    /// shutdown the bot
    pub fn shutdown(self) {
        self.shutdown_token.cancel();
        info!("Bot shutdown requested");
    }
}
