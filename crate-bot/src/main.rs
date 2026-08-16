use figment::providers::Format;
use lamprey_bot::{bot::Bot, config::Config};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dotenvy::dotenv();

    let config: Config = figment::Figment::new()
        .merge(figment::providers::Toml::file("bot.toml"))
        .merge(figment::providers::Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    tracing_subscriber::fmt()
        .with_env_filter(&config.rust_log)
        .init();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    info!("config {config:#?}");

    let bot = Bot::from_config(config).await?;
    bot.start().await;

    // TODO: proper shutdown handling
    futures::future::pending::<()>().await;

    Ok(())
}

// struct Handle {
//     http: Http,
//     voice_states: Vec<VoiceState>,
//     user: Option<common::v1::types::User>,
//     // Removed player/control fields as they were for the old RTC implementation
// }

// impl Handle {
//     async fn join_voice(&mut self, message: &Message, client: &Client) -> anyhow::Result<()> {
//         let author_voice_state = self
//             .voice_states
//             .iter()
//             .find(|s| s.user_id == message.author_id)
//             .ok_or_else(|| anyhow!("you aren't in a voice thread"))?;

//         // Use SDK voice
//         let _voice = client
//             .voice(author_voice_state.channel_id)
//             .connect()
//             .await?;

//         info!("joined voice channel: {:?}", author_voice_state.channel_id);

//         // TODO: Handle voice events and audio playback through the SDK voice client

//         Ok(())
//     }
// }
