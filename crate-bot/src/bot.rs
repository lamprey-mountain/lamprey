use crate::commands::{RemindCommand, VoiceCommand};
use crate::prelude::*;
use crate::store::Store;
use crate::{commands::Command, config::Config};
use clap::Parser;
use common::v1::types::MessageSync;
use common::v1::types::components::{ComponentCreate, ComponentType, ComponentsCreate};
use common::v1::types::flume::{FlumeAppendCreate, FlumeDeltaCreate, FlumeReplaceCreate};
use common::v1::types::misc::duration::Duration;
use common::v1::types::misc::{Time, UserIdReq};
use common::v1::types::{
    Message, MessageClient,
    presence::{Activity, Presence, Status},
};
use common::v1::types::{MessagePayload, MessageType};
use common::v2::types::ChannelId;
use common::v2::types::UserId;
use common::v2::types::components::ComponentId;
use futures::StreamExt;
use sdk::Client;
use sdk::syncer::SyncerEvent;
use sdk::voice::{FileSource, Player, Voice};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info, warn};

use crate::llm::Llm;
use crate::reminders;
use crate::util::BotResponse;

pub struct Bot {
    config: Config,
    user_id: Option<UserId>, // surely there's a better way to do this?
    client: Client,
    store: Store,
    voice: Option<Voice>,
    player: Option<Player>,
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

// TODO: re-add logging from old bot code
// info!("joined voice channel: {:?}", author_voice_state.channel_id);

impl Bot {
    /// initialize a bot from some config
    pub async fn from_config(config: Config) -> Result<Self> {
        let store = Store::open(&config.database_url).await?;

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
            config,
            user_id: None,
            client,
            store,
            voice: None,
            player: None,
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
                        self.user_id = Some(user.id);
                    } else {
                        error!("no user for this token!");
                        // TODO: handle missing token better
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
            MessageType::DefaultMarkdown(m) | MessageType::ThreadInitial(m) => m.content.as_deref(),
            _ => None,
        };

        if let Some(content) = content {
            debug!("message from {}: {}", message.author_id, content);

            if content.contains("egg") {
                let http = self.client.http();
                let channel_id = message.channel_id;
                let message_id = message.id;
                tokio::spawn(async move {
                    _ = http
                        .reaction_create(
                            channel_id,
                            message_id,
                            "t:\u{1F95A}".to_string(),
                            UserIdReq::UserSelf,
                        )
                        .await;
                });
            }
        } else {
            debug!("message from {} without content", message.author_id);
        }

        if let Some(command) = content.and_then(|c: &str| c.strip_prefix("!")) {
            debug!("got raw command {command:?}");
            let command = Command::try_parse_from(
                std::iter::once("bot".to_string())
                    .chain(command.split_whitespace().map(|s| s.to_string())),
            );
            let resp: BotResponse = match command {
                Ok(command) => {
                    debug!("got command {command:?}");
                    self.handle_command(&message, command)
                        .await
                        .unwrap_or_else(|e| e.to_string().into())
                }
                Err(err) => err.to_string().into(),
            };
            if let Some(resp) = resp.0 {
                self.client
                    .http()
                    .message_create(message.channel_id, &resp)
                    .await?;
            }
        }

        self.handle_llm(&message);

        Ok(())
    }

    /// get the voice channel the message's author is connected to
    fn sender_voice_channel(&self, message: &Message) -> Result<ChannelId> {
        self.client
            .cache()
            .and_then(|cache| cache.user_voice_states(message.author_id).next())
            .map(|state| state.channel_id)
            .ok_or_else(|| anyhow::anyhow!("you aren't in a voice channel"))
    }

    async fn handle_command(&mut self, message: &Message, cmd: Command) -> Result<BotResponse> {
        let resp: BotResponse = match cmd {
            Command::Ping => "pong!".into(),
            Command::Voice(v) => self.handle_command_voice(message, v).await?,
            Command::Remind(cmd) => self.handle_command_remind(message, cmd).await?,
        };
        Ok(resp)
    }

    async fn handle_command_voice(
        &mut self,
        message: &Message,
        cmd: VoiceCommand,
    ) -> Result<BotResponse> {
        let resp: BotResponse = match cmd {
            VoiceCommand::Join => {
                if self.voice.is_some() {
                    "already joined to a voice channel".into()
                } else {
                    let channel_id = self.sender_voice_channel(message)?;
                    let voice = self.client.voice(channel_id).await?;
                    let (player, producer) = Player::new_audio();
                    voice.publish(producer).await?;
                    self.player = Some(player);
                    self.voice = Some(voice);
                    "joined".into()
                }
            }
            VoiceCommand::Leave => {
                if let Some(voice) = self.voice.take() {
                    if let Some(player) = self.player.take() {
                        player.stop();
                    }
                    voice.disconnect().await?;
                    "left".into()
                } else {
                    "not joined".into()
                }
            }
            VoiceCommand::Play => {
                if let Some(player) = self.player.clone() {
                    player.set_paused(false);
                    "playing".into()
                } else {
                    let voice = match self.voice.clone() {
                        Some(voice) => voice,
                        None => {
                            let channel_id = self.sender_voice_channel(message)?;
                            let voice = self.client.voice(channel_id).await?;
                            self.voice = Some(voice.clone());
                            voice
                        }
                    };
                    let source = FileSource::from_path(&self.config.music_path)?;
                    let (player, producer) = Player::new_audio();
                    player.play(source);
                    voice.publish(producer).await?;
                    self.player = Some(player);
                    "playing".into()
                }
            }
            VoiceCommand::Pause { paused } => match &self.player {
                Some(player) => {
                    let paused = paused.unwrap_or(!player.paused());
                    player.set_paused(paused);
                    if paused { "paused" } else { "resumed" }.into()
                }
                None => "no player".into(),
            },
            VoiceCommand::Stop => {
                if let Some(player) = self.player.take() {
                    player.stop();
                    "stopped".into()
                } else {
                    "no player".into()
                }
            }
        };
        Ok(resp)
    }

    async fn handle_command_remind(
        &mut self,
        message: &Message,
        cmd: RemindCommand,
    ) -> Result<BotResponse> {
        let resp: BotResponse = match cmd {
            RemindCommand::Add { duration, text } => {
                let text = text.join(" ");
                let duration: Duration = duration
                    .parse()
                    .map_err(|_| anyhow::anyhow!("invalid duration (try e.g. 5m, 1h, 2d)"))?;
                let scheduled_at = Time::now_utc() + duration;
                let reminder =
                    reminders::add(&self.store, message.author_id, text, scheduled_at).await?;
                format!(
                    "Reminder #{} set for {}: {}",
                    reminder.id,
                    reminder.scheduled_at.to_string(),
                    reminder.text
                )
                .into()
            }
            RemindCommand::Remove { id } => {
                reminders::remove(&self.store, message.author_id, id).await?;
                format!("Removed reminder {id}").into()
            }
            RemindCommand::RemoveAll => {
                reminders::remove_all(&self.store, message.author_id).await?;
                "Removed all your reminders".into()
            }
            RemindCommand::List => {
                let reminders = reminders::list(&self.store, message.author_id).await?;

                if reminders.is_empty() {
                    "No reminders".into()
                } else {
                    let mut output = String::new();
                    for reminder in &reminders {
                        output.push_str(&format!(
                            "[{}] {}",
                            reminder.id,
                            reminder.scheduled_at.to_string()
                        ));
                        if !reminder.text.is_empty() {
                            output.push_str(&format!(": {}", reminder.text));
                        }
                        output.push('\n');
                    }
                    output.into()
                }
            }
        };
        Ok(resp)
    }

    fn handle_llm(&self, message: &Message) {
        let Some(llm_config) = &self.config.llm else {
            return;
        };

        let Some(current_user_id) = self.user_id else {
            return;
        };

        if message.author_id == current_user_id {
            return;
        }

        if !llm_config.channel_ids.contains(&message.channel_id) {
            return;
        }

        let content = match &message.latest_version.message_type {
            MessageType::DefaultMarkdown(m) | MessageType::ThreadInitial(m) => m.content.as_deref(),
            _ => None,
        };

        let Some(prompt) = content else {
            return;
        };

        let client = self.client.clone();
        let llm = Llm::new(llm_config.clone());
        let channel_id = message.channel_id;
        let reply_id = message.id;
        let prompt = prompt.to_string();

        tokio::spawn(async move {
            let flume = match client
                .flume(channel_id)
                .reply_id(reply_id)
                .components(ComponentsCreate {
                    inner: vec![ComponentCreate {
                        id: Some(ComponentId(0)),
                        ty: ComponentType::Container {
                            components: vec![ComponentCreate {
                                id: Some(ComponentId(1)),
                                ty: ComponentType::Text {
                                    content: "*loading...*".to_string(),
                                },
                                allow: None,
                            }],
                            color: "#33454f".parse().ok(),
                        },
                        allow: None,
                    }],
                })
                .await
            {
                Ok(flume) => flume,
                Err(e) => {
                    eprintln!("Failed to create flume: {:?}", e);
                    return;
                }
            };

            flume
                .update(FlumeDeltaCreate::default().replace(FlumeReplaceCreate {
                    target: ComponentId(0),
                    components: vec![ComponentCreate {
                        id: Some(ComponentId(0)),
                        ty: ComponentType::Container {
                            components: vec![ComponentCreate {
                                id: Some(ComponentId(1)),
                                ty: ComponentType::Text {
                                    content: "*loading...*".to_string(),
                                },
                                allow: None,
                            }],
                            color: "#517082".parse().ok(),
                        },
                        allow: None,
                    }],
                }))
                .await;

            match llm.complete(prompt).await {
                Ok(stream) => {
                    futures::pin_mut!(stream);
                    let mut buffer = String::new();
                    let mut first = true;

                    while let Some(chunk) = stream.next().await {
                        let Some(content) = chunk.text() else {
                            continue;
                        };
                        buffer.push_str(content);

                        let delta = if first {
                            first = false;
                            FlumeDeltaCreate::default().replace(FlumeReplaceCreate {
                                target: ComponentId(1),
                                components: vec![ComponentCreate {
                                    id: Some(ComponentId(1)),
                                    ty: ComponentType::Text {
                                        content: buffer.clone(),
                                    },
                                    allow: None,
                                }],
                            })
                        } else {
                            FlumeDeltaCreate::default().append(FlumeAppendCreate {
                                target: ComponentId(1),
                                components: vec![ComponentCreate {
                                    id: Some(ComponentId(1)),
                                    ty: ComponentType::Text {
                                        content: content.to_string(),
                                    },
                                    allow: None,
                                }],
                            })
                        };
                        flume.update(delta).await;
                    }
                }
                Err(e) => {
                    eprintln!("Error calling LLM: {:?}", e);
                }
            }

            flume.commit().await;
        });
    }
}

impl BotHandle {
    /// shutdown the bot
    pub fn shutdown(self) {
        self.shutdown_token.cancel();
        info!("Bot shutdown requested");
    }
}
