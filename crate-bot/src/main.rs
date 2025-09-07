use std::sync::Arc;

use anyhow::anyhow;
use clap::Parser;
use common::v1::types::{
    voice::{SignallingMessage, VoiceState, VoiceStateUpdate},
    Message, MessageClient, MessageCreate, MessageType, Session, User, UserId,
};
use figment::providers::{Env, Format, Toml};
use sdk::{Client, EventHandler, Http};
use tokio::sync::{mpsc::Sender, Mutex};
use tracing::{debug, error, info, warn};

use crate::{
    config::Config,
    rtc::{Player, PlayerCommand},
};

mod config;
mod rtc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let config: Config = figment::Figment::new()
        .merge(Toml::file("bot.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    tracing_subscriber::fmt()
        .with_env_filter(&config.rust_log)
        .init();

    info!("config {config:#?}");

    let mut client = Client::new(config.token.clone().into());
    client.http = if let Some(base_url) = config.base_url.clone() {
        client.http.with_base_url(base_url.parse()?)
    } else {
        client.http
    };
    client.syncer = if let Some(ws_url) = config.ws_url.clone() {
        client.syncer.with_base_url(ws_url.parse()?)
    } else {
        client.syncer
    };

    let (control, control_events) = tokio::sync::mpsc::channel(100);
    let handle = Handle {
        http: client.http,
        voice_states: vec![],
        control,
        user: None,
        player: Arc::new(Mutex::new(None)),
        config,
    };
    client
        .syncer
        .with_handler(Box::new(handle))
        .with_controller(control_events)
        .connect()
        .await?;

    Ok(())
}

#[derive(Debug, clap::Parser)]
pub enum Command {
    /// ping the bot to see if its online
    Ping,

    /// commands for voice/music management
    #[command(subcommand, alias = "vc")]
    Voice(VoiceCommand),
}

#[derive(Debug, clap::Subcommand)]
pub enum VoiceCommand {
    /// join the voice thread you're in
    Join,

    /// leave the voice thread the bot is in
    Leave,

    /// play or resume music
    Play,

    /// toggle pause state
    Pause {
        #[arg(short)]
        paused: Option<bool>,
    },

    /// stop current music
    Stop,
}

struct Handle {
    http: Http,
    voice_states: Vec<VoiceState>,
    control: Sender<MessageClient>,
    user: Option<User>,
    player: Arc<Mutex<Option<Sender<PlayerCommand>>>>,
    config: Config,
}

impl Handle {
    async fn send_signalling(&self, msg: SignallingMessage) -> anyhow::Result<()> {
        if let Some(user) = &self.user {
            self.control
                .send(MessageClient::VoiceDispatch {
                    user_id: user.id,
                    payload: serde_json::to_value(msg)?,
                })
                .await?;
        } else {
            warn!("no user for this connection");
        }
        Ok(())
    }

    async fn handle_command(&mut self, message: &Message, cmd: Command) -> anyhow::Result<String> {
        let resp = match cmd {
            Command::Ping => "pong!".to_string(),
            Command::Voice(v) => match v {
                VoiceCommand::Join => {
                    self.join_voice(message).await?;
                    "joined".to_string()
                }
                VoiceCommand::Leave => {
                    self.send_signalling(SignallingMessage::VoiceState { state: None })
                        .await?;
                    *self.player.lock().await = None;
                    "left".to_string()
                }
                VoiceCommand::Play => {
                    let _ = self.join_voice(message).await;
                    if let Some(p) = &*self.player.lock().await {
                        p.send(PlayerCommand::Play(self.config.music_path.clone()))
                            .await?;
                        "playing".to_string()
                    } else {
                        "no player".to_string()
                    }
                }
                VoiceCommand::Pause { paused } => {
                    if let Some(p) = &*self.player.lock().await {
                        p.send(PlayerCommand::Pause(paused)).await?;
                        "(un)paused".to_string()
                    } else {
                        "no player".to_string()
                    }
                }
                VoiceCommand::Stop => {
                    if let Some(p) = &*self.player.lock().await {
                        p.send(PlayerCommand::Stop).await?;
                        "stopped".to_string()
                    } else {
                        "no player".to_string()
                    }
                }
            },
        };

        Ok(resp)
    }

    async fn join_voice(&mut self, message: &Message) -> anyhow::Result<()> {
        let Some(user) = &self.user else {
            return Err(anyhow!("no user for this connection!?"));
        };

        let author_voice_state = self
            .voice_states
            .iter()
            .find(|s| s.user_id == message.author_id);
        let Some(author_voice_state) = author_voice_state else {
            return Err(anyhow!("you aren't in a voice thread"));
        };

        if self.player.lock().await.is_some() {
            return Err(anyhow!("already playing music"));
        }

        self.send_signalling(SignallingMessage::VoiceState {
            state: Some(VoiceStateUpdate {
                thread_id: author_voice_state.thread_id,
            }),
        })
        .await?;

        let (commands_send, commands_recv) = tokio::sync::mpsc::channel(100);
        let (events_send, mut events_recv) = tokio::sync::mpsc::channel(100);

        {
            let self_control = self.control.clone();
            let user_id = user.id;
            let thread_id = message.thread_id;
            let http = self.http.clone();
            let player = self.player.clone();
            tokio::spawn(async move {
                while let Some(ev) = events_recv.recv().await {
                    match ev {
                        rtc::PlayerEvent::Signalling(msg) => {
                            info!("sending signalling mesage: {msg:?}");
                            self_control
                                .send(MessageClient::VoiceDispatch {
                                    user_id,
                                    payload: serde_json::to_value(msg).unwrap(),
                                })
                                .await
                                .expect("controller is dead!");
                        }
                        rtc::PlayerEvent::Dead => {
                            *player.lock().await = None;
                            info!("cleaned up dead player");
                        }
                        rtc::PlayerEvent::Finished => {
                            let msg = MessageCreate {
                                content: Some("song finished".to_string()),
                                attachments: vec![],
                                metadata: None,
                                reply_id: None,
                                override_name: None,
                                nonce: None,
                                embeds: vec![],
                                created_at: None,
                            };
                            if let Err(err) = http.message_create(thread_id, &msg).await {
                                error!("couldn't send message: {err}");
                            }
                        }
                    }
                }
            });
        }

        match Player::new(commands_recv, events_send).await {
            Ok(player) => {
                debug!("created player");
                tokio::spawn(player.run());
                debug!("spawned player");
                *self.player.lock().await = Some(commands_send);
                Ok(())
            }
            Err(err) => {
                error!("failed to create player: {err}");
                Err(anyhow!("failed to create player: {err}"))
            }
        }
    }
}

impl EventHandler for Handle {
    type Error = anyhow::Error;

    async fn ready(&mut self, user: Option<User>, _session: Session) -> Result<(), Self::Error> {
        if let Some(user) = user {
            info!("logged in as {}!", user.name);
            self.user = Some(user);
        } else {
            error!("no user for this token!");
            anyhow::bail!("no user for this token!");
        }

        Ok(())
    }

    async fn message_create(&mut self, message: Message) -> Result<(), Self::Error> {
        let content = match &message.message_type {
            MessageType::DefaultMarkdown(m) => m.content.as_deref(),
            // MessageType::MessagePinned(message_pin) => todo!(),
            // MessageType::MessageUnpinned(message_pin) => todo!(),
            // MessageType::MemberAdd(message_member) => todo!(),
            // MessageType::MemberRemove(message_member) => todo!(),
            // MessageType::MemberJoin(message_member) => todo!(),
            // MessageType::Call(message_call) => todo!(),
            // MessageType::ThreadRename(message_thread_rename) => todo!(),
            // MessageType::ThreadPingback(message_thread_pingback) => todo!(),
            _ => None,
        };

        if let Some(content) = content {
            debug!("message from {}: {}", message.author_id, content);
        } else {
            debug!("message from {} without content", message.author_id);
        }

        if let Some(command) = content.and_then(|c| c.strip_prefix("!")) {
            debug!("got raw command {command:?}");
            let command =
                Command::try_parse_from(std::iter::once("bot").chain(command.split_whitespace()));
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
                content: Some(resp.into()),
                attachments: vec![],
                metadata: None,
                reply_id: Some(message.id),
                override_name: None,
                nonce: None,
                embeds: vec![],
                created_at: None,
            };
            self.http.message_create(message.thread_id, &resp).await?;
        }

        Ok(())
    }

    async fn voice_state(
        &mut self,
        user_id: UserId,
        state: Option<VoiceState>,
    ) -> Result<(), Self::Error> {
        debug!("got voice state for {user_id}: {state:?}");
        if let Some(user) = &self.user {
            if user.id == user_id && state == None {
                if let Some(p) = &*self.player.lock().await {
                    p.send(PlayerCommand::Stop).await?;
                }
            }
        };

        self.voice_states.retain(|s| s.user_id != user_id);
        if let Some(state) = state {
            self.voice_states.push(state);
        }
        Ok(())
    }

    async fn voice_dispatch(
        &mut self,
        user_id: UserId,
        payload: SignallingMessage,
    ) -> Result<(), Self::Error> {
        debug!("got voice dispatch for {user_id}: {payload:?}");
        if let Some(p) = &*self.player.lock().await {
            p.send(PlayerCommand::Signalling(payload)).await?;
        }
        Ok(())
    }
}
