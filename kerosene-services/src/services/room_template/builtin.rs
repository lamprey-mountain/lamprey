use common::{
    v1::types::{
        Permission,
        defaults::{SERVER_EVERYONE, SERVER_REGISTERED},
    },
    v2::types::{SERVER_ADMIN_ROLE_ID, SERVER_REGISTERED_ROLE_ID, SERVER_ROOM_ID},
};

use super::*;

pub fn public_room() -> RoomTemplateSnapshot {
    room_snapshot(true)
}

pub fn private_room() -> RoomTemplateSnapshot {
    room_snapshot(false)
}

fn room_snapshot(public: bool) -> RoomTemplateSnapshot {
    let everyone_id = Uuid::now_v7();
    let general_id = Uuid::now_v7();

    RoomTemplateSnapshot {
        roles: vec![
            RoomTemplateRole {
                id: Uuid::now_v7(),
                inner: RoleCreate::new("admin").allow(ADMIN_ROOM),
                position: 2,
            },
            RoomTemplateRole {
                id: Uuid::now_v7(),
                inner: RoleCreate::new("moderator").allow(MODERATOR),
                position: 1,
            },
            RoomTemplateRole {
                id: everyone_id,
                inner: RoleCreate::new("everyone")
                    .description("Default role")
                    .allow(if public {
                        EVERYONE_UNTRUSTED.to_vec()
                    } else {
                        EVERYONE_TRUSTED.to_vec()
                    }),
                position: 0,
            },
        ],
        channels: vec![RoomTemplateChannel {
            id: general_id,
            inner: ChannelCreate {
                name: "general".to_string(),
                ty: ChannelType::Text,
                ..Default::default()
            },
            position: 0,
        }],
        welcome_channel_id: Some(general_id.into()),
        afk_channel_id: None,
        afk_channel_timeout: 300000,
    }
}

pub fn server_room() -> RoomTemplateSnapshot {
    RoomTemplateSnapshot {
        channels: vec![],
        roles: vec![
            RoomTemplateRole {
                id: *SERVER_ADMIN_ROLE_ID,
                inner: RoleCreate::new("admin")
                    .description("server-wide administrator")
                    .allow([Permission::Admin]),
                position: 3,
            },
            // this role uses a normal uuid since it doesn't have any special behaviors
            RoomTemplateRole {
                id: Uuid::now_v7(),
                inner: RoleCreate::new("moderator")
                    .description("server-wide administrator")
                    .allow(MODERATOR),
                position: 2,
            },
            // TODO: rework how "guest"/"registered" users work. it's kind of confusing and error prone.
            RoomTemplateRole {
                id: *SERVER_REGISTERED_ROLE_ID,
                inner: RoleCreate::new("registered").allow(SERVER_REGISTERED),
                position: 1,
            },
            RoomTemplateRole {
                id: *SERVER_ROOM_ID,
                inner: RoleCreate::new("everyone")
                    .description("Default role")
                    .allow(SERVER_EVERYONE),
                position: 0,
            },
        ],
        welcome_channel_id: None,
        afk_channel_id: None,
        afk_channel_timeout: 300000,
    }
}
