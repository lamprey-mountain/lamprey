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
                inner: RoleCreate {
                    name: "admin".to_string(),
                    description: None,
                    allow: ADMIN_ROOM.to_vec(),
                    deny: vec![],
                    is_self_applicable: false,
                    is_mentionable: false,
                    hoist: false,
                    sticky: false,
                },
                position: 2,
            },
            RoomTemplateRole {
                id: Uuid::now_v7(),
                inner: RoleCreate {
                    name: "moderator".to_string(),
                    description: None,
                    allow: MODERATOR.to_vec(),
                    deny: vec![],
                    is_self_applicable: false,
                    is_mentionable: false,
                    hoist: false,
                    sticky: false,
                },
                position: 1,
            },
            RoomTemplateRole {
                id: everyone_id,
                inner: RoleCreate {
                    name: "everyone".to_string(),
                    description: Some("Default role".to_string()),
                    allow: if public {
                        EVERYONE_UNTRUSTED.to_vec()
                    } else {
                        EVERYONE_TRUSTED.to_vec()
                    },
                    deny: vec![],
                    is_self_applicable: false,
                    is_mentionable: false,
                    hoist: false,
                    sticky: false,
                },
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
