use clap::Parser;
use common::{
    v1::types::{util::Time, AuditLogEntry, AuditLogEntryType},
    v2::types::{AuditLogEntryId, SERVER_USER_ID},
};
use figment::providers::{Env, Format, Toml};
use lamprey_backend_core::types::admin::AdminCollectGarbageTarget;
use tracing::info;

use lamprey_backend::{
    cli, config, error,
    serve::server::{Server, gc, setup_otel},
    types::{self,  RoomMemberPut, SERVER_ROOM_ID},
};

use config::Config;
use error::Result;

#[tokio::main]
async fn main() -> Result<()> {
    #[cfg(feature = "debug")]
    unsafe {
        backtrace_on_stack_overflow::enable()
    }

    let _ = dotenvy::dotenv();

    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    let args = cli::Args::parse();

    let config: Config = figment::Figment::new()
        .merge(Toml::file(args.config))
        // .merge(Toml::file("config.toml"))
        .merge(Env::raw().only(&["RUST_LOG"]))
        .extract()?;

    setup_otel(&config)?;

    let server = Server::init_from_config(config).await?;
    let state = server.state();

    match &args.command {
        cli::Command::Serve {} => server.serve().await?,
        cli::Command::GcMedia {} => gc(state, &[AdminCollectGarbageTarget::Media]).await?,
        cli::Command::GcMessages {} => gc(state, &[AdminCollectGarbageTarget::Messages]).await?,
        cli::Command::GcSession {} => gc(state, &[AdminCollectGarbageTarget::Session]).await?,
        cli::Command::GcAuditLog {} => gc(state, &[AdminCollectGarbageTarget::AuditLog]).await?,
        cli::Command::GcRoomAnalytics {} => {
            gc(state, &[AdminCollectGarbageTarget::RoomAnalytics]).await?
        }
        cli::Command::GcAll {} => {
            gc(
                state,
                &[
                    AdminCollectGarbageTarget::Media,
                    AdminCollectGarbageTarget::Messages,
                    AdminCollectGarbageTarget::Session,
                    AdminCollectGarbageTarget::AuditLog,
                    AdminCollectGarbageTarget::RoomAnalytics,
                ],
            )
            .await?
        }
        cli::Command::Register { user_id, reason } => {
            let mut txn = state.acquire_data().await?;
            txn.user_set_registered(*user_id, Some(Time::now_utc()), None)
                .await?;
            txn.room_member_put(SERVER_ROOM_ID, *user_id, None, RoomMemberPut::default())
                .await?;
            // TODO: append audit log in same txn
            // only broadcast on successful commit
            state
                .audit_log_append(AuditLogEntry {
                    id: AuditLogEntryId::new(),
                    room_id: SERVER_ROOM_ID,
                    user_id: SERVER_USER_ID,
                    session_id: None,
                    reason: reason.to_owned(),
                    ty: AuditLogEntryType::UserRegistered { user_id: *user_id },
                    status: common::v1::types::AuditLogEntryStatus::Success,
                    started_at: Time::now_utc(),
                    ended_at: Time::now_utc(),
                    ip_addr: None,
                    user_agent: None,
                    application_id: None,
                })
                .await?;
            txn.commit().await?;
            // TODO: invalidate cache
            // right now i'd need to restart backend or it would think the user is still a guest
            info!("registered!");
        }
        cli::Command::MakeAdmin { user_id } => {
            let mut txn = state.acquire_data().await?;
            txn.room_member_put(
                SERVER_ROOM_ID,
                *user_id,
                None,
                types::RoomMemberPut::default(),
            )
            .await?;
            let roles = txn.role_list(SERVER_ROOM_ID).await?;
            txn.role_member_put(SERVER_ROOM_ID, *user_id, roles[1].id)
                .await?;
            txn.commit().await?;
        }
    }

    Ok(())
}
