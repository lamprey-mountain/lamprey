use clap::Parser;
use common::v1::types::{
    voice::VoiceState, Message, MessageCreate, MessageType, Session, User, UserId,
};
use figment::providers::{Env, Format, Toml};
use sdk::{Client, EventHandler, Http};
use tracing::{debug, error, info};

use crate::config::Config;

mod config;

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

    let mut client = Client::new(config.token.into());
    client.http = if let Some(base_url) = config.base_url {
        client.http.with_base_url(base_url.parse()?)
    } else {
        client.http
    };
    client.syncer = if let Some(ws_url) = config.ws_url {
        client.syncer.with_base_url(ws_url.parse()?)
    } else {
        client.syncer
    };

    let handle = Handle { http: client.http };
    client
        .syncer
        .with_handler(Box::new(handle))
        .connect()
        .await?;

    Ok(())
}

struct Handle {
    http: Http,
}

impl EventHandler for Handle {
    type Error = anyhow::Error;

    async fn ready(&mut self, user: Option<User>, _session: Session) -> Result<(), Self::Error> {
        if let Some(user) = user {
            info!("logged in as {}!", user.name);
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
            let command = dbg!(Command::try_parse_from(
                std::iter::once("bot").chain(command.split_whitespace())
            ))?;
            debug!("got command {command:?}");
            match command {
                Command::Ping => {
                    let resp = MessageCreate {
                        content: Some("pong!".into()),
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
            }
        }

        Ok(())
    }

    async fn voice_state(
        &mut self,
        user_id: UserId,
        state: Option<VoiceState>,
    ) -> Result<(), Self::Error> {
        debug!("got voice state for {user_id}: {state:?}");
        Ok(())
    }
}

#[derive(Debug, clap::Parser)]
enum Command {
    Ping,
}
