use clap::Parser;
use common::{
    v1::types::{AuditLogEntry, AuditLogEntryType, util::Time},
    v2::types::{AuditLogEntryId, SERVER_USER_ID},
};
use figment::providers::{Env, Format, Toml};
use kerosene_core::types::admin::{AdminCollectGarbage, AdminCollectGarbageMode};
use lamprey_backend_core::types::admin::AdminCollectGarbageTarget;
use tracing::info;

use lamprey_backend::{
    Error, cli, config, error,
    server::Server,
    state::Globals,
    types::{self, MessageSync, RoomMemberPut, SERVER_ROOM_ID},
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

    kerosene_core::observability::init(&config).map_err(|e| Error::Internal(e.to_string()))?;

    let mut server = Server::init_from_config(config).await?;
    let globals = server.globals();

    match &args.command {
        cli::Command::Serve {} => server.serve().await?,
        cli::Command::Config {} => println!("{:#?}", globals.config()),
        cli::Command::GcMedia {} => gc(globals, &[AdminCollectGarbageTarget::Media]).await?,
        cli::Command::GcMessages {} => gc(globals, &[AdminCollectGarbageTarget::Messages]).await?,
        cli::Command::GcSession {} => gc(globals, &[AdminCollectGarbageTarget::Session]).await?,
        cli::Command::GcAuditLog {} => gc(globals, &[AdminCollectGarbageTarget::AuditLog]).await?,
        cli::Command::GcRoomAnalytics {} => {
            gc(globals, &[AdminCollectGarbageTarget::RoomAnalytics]).await?
        }
        cli::Command::GcAll {} => {
            gc(
                globals,
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
            // TODO: move this into services
            let mut txn = globals.begin().await?;
            txn.user_set_registered(*user_id, Some(Time::now_utc()), None)
                .await?;
            txn.room_member_put(SERVER_ROOM_ID, *user_id, None, RoomMemberPut::default())
                .await?;
            // TODO: append audit log in same txn
            // only broadcast on successful commit
            let entry = AuditLogEntry {
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
            };
            txn.audit_logs_room_append(entry.clone()).await?;
            txn.commit().await?;
            globals
                .messaging()
                .broadcast_room(entry.room_id, MessageSync::AuditLogEntryCreate { entry })
                .await?;
            // TODO: invalidate cache
            // right now i'd need to restart backend or it would think the user is still a guest
            info!("registered!");
        }
        cli::Command::MakeAdmin { user_id } => {
            // TODO: move this into services
            let mut txn = globals.begin().await?;
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

pub async fn gc(globals: Globals, targets: &[AdminCollectGarbageTarget]) -> Result<()> {
    let srv = globals.services();

    info!("starting garbage collection");

    let res = srv
        .admin
        .collect_garbage(AdminCollectGarbage {
            targets: targets.to_vec(),
            mode: AdminCollectGarbageMode::Mark,
            async_mode: false,
        })
        .await?;

    for s in res.stats {
        info!(
            "marked {} items of type {:?} for deletion",
            s.rows_deleted, s.target
        );
    }

    info!("deleting...");

    srv.admin
        .collect_garbage(AdminCollectGarbage {
            targets: targets.to_vec(),
            mode: AdminCollectGarbageMode::Sweep,
            async_mode: false,
        })
        .await?;

    info!("done!");

    Ok(())
}
