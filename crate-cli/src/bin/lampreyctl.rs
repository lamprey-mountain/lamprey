use std::sync::Arc;

use clap::Parser;
use figment::providers::{Env, Format, Toml};
use lamprey_backend::config::Config;
use lamprey_backend_core::types::admin::{
    AdminCollectGarbage, AdminCollectGarbageMode, AdminCollectGarbageTarget,
};
use lamprey_cli::args::lampreyctl::{
    Args, Command, GcMode, GcTarget, MaintenenceCommand, ServeCommand, UserCommand,
};

#[tokio::main]
async fn main() {
    if let Err(err) = main_inner().await {
        eprintln!("error: {err}");
    }
}

async fn main_inner() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let config: Config = figment::Figment::new()
        .merge(Toml::file(args.config))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    // TODO: share setup_otel function between crates
    lamprey_backend::serve::server::setup_otel(&config)?;

    match args.command {
        Command::Serve { target } => match target {
            ServeCommand::Api => {
                let server =
                    lamprey_backend::serve::server::Server::init_from_config(config).await?;
                server.serve().await?;
            }
            ServeCommand::Media { media_config } => {
                let server = lamprey_media::server::MediaServer::init_from_config(config).await?;
                server.serve().await?;
            }
            ServeCommand::Voice { sfu_config } => {
                lamprey_voice::sfu::Sfu::run(Arc::new(config)).await;
            }
        },
        Command::Maintenence { target } => match target {
            MaintenenceCommand::Gc {
                target,
                mode,
                run_async,
            } => {
                // TODO: clean up code? theres probably a better way than manually writing out entire `match`es
                let token = if config.enable_admin_token {
                    config
                        .admin_token
                        .as_deref()
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "admin token not configured (enable_admin_token is true but admin_token is None)"
                            )
                        })?
                        .to_owned()
                } else {
                    args.token
                        .ok_or_else(|| anyhow::anyhow!("missing --token"))?
                };

                let client = lamprey_sdk::Client::new(token.into());

                let targets: Vec<AdminCollectGarbageTarget> =
                    if target.is_empty() || target.contains(&GcTarget::All) {
                        vec![
                            AdminCollectGarbageTarget::Media,
                            AdminCollectGarbageTarget::Messages,
                            AdminCollectGarbageTarget::Session,
                            AdminCollectGarbageTarget::AuditLog,
                            AdminCollectGarbageTarget::RoomAnalytics,
                        ]
                    } else {
                        target
                            .iter()
                            .map(|t| match t {
                                GcTarget::Media => AdminCollectGarbageTarget::Media,
                                GcTarget::Messages => AdminCollectGarbageTarget::Messages,
                                GcTarget::Session => AdminCollectGarbageTarget::Session,
                                GcTarget::AuditLog => AdminCollectGarbageTarget::AuditLog,
                                GcTarget::RoomAnalytics => AdminCollectGarbageTarget::RoomAnalytics,
                                GcTarget::All => unreachable!(),
                            })
                            .collect()
                    };

                let gc_mode = match mode {
                    GcMode::Mark => AdminCollectGarbageMode::Mark,
                    GcMode::Sweep => AdminCollectGarbageMode::Sweep,
                    GcMode::Dry => AdminCollectGarbageMode::Dry,
                    GcMode::Full => AdminCollectGarbageMode::Mark,
                };

                let res = client
                    .http
                    .admin_collect_garbage(&AdminCollectGarbage {
                        targets,
                        mode: gc_mode,
                        async_mode: run_async,
                    })
                    .await?;

                if res.stats.is_empty() {
                    println!("no targets specified");
                } else {
                    println!("garbage collection complete:");
                    for stat in &res.stats {
                        let target_name = match stat.target {
                            AdminCollectGarbageTarget::Media => "Media",
                            AdminCollectGarbageTarget::Messages => "Messages",
                            AdminCollectGarbageTarget::Session => "Session",
                            AdminCollectGarbageTarget::AuditLog => "AuditLog",
                            AdminCollectGarbageTarget::RoomAnalytics => "RoomAnalytics",
                        };
                        print!("  {}: {} rows deleted", target_name, stat.rows_deleted);
                        if let Some(bytes) = stat.bytes_deleted {
                            print!(", {} bytes", bytes);
                        }
                        println!(" ({} ms elapsed)", stat.ms_elapsed);
                    }
                }
            }
            MaintenenceCommand::Broadcast {} => todo!(),
            MaintenenceCommand::PurgeCache { target } => todo!(),
            MaintenenceCommand::SearchIndex { command } => todo!(),
            MaintenenceCommand::UnloadRoom { room_id } => todo!(),
            MaintenenceCommand::ReloadRoom { room_id } => todo!(),
            MaintenenceCommand::Whisper {} => todo!(),
        },
        Command::Check => todo!(),
        Command::User { command } => match command {
            UserCommand::Create { name } => todo!(),
            UserCommand::Register { reason, user_id } => todo!(),
            UserCommand::MakeAdmin { user_id } => todo!(),
        },
    }

    Ok(())
}
