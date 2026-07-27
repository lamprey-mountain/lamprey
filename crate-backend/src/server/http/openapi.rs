use crate::types;
use serde_json::json;
use utoipa::{Modify, OpenApi, openapi::extensions::Extensions};

#[derive(OpenApi)]
#[openapi(
components(schemas(
    types::Room,
    types::RoomPatch,
    types::User,
    types::Channel,
    types::ChannelPatch,
    types::Message,
    types::RoomMember,
    types::Role,
    types::RolePatch,
    // utoipa seems to forget to add these types specifically
    types::UserIdReq,
    common::v1::types::misc::ApplicationIdReq,
    types::UserListParams,
    types::UserListFilter,
    common::v1::types::MessageSync,
    common::v1::types::MessageClient,
    common::v1::types::PaginationQuery<common::v1::types::MessageId>,
    common::v1::types::pagination::PaginationResponse<types::Message>,
    types::emoji::EmojiCustom,
    types::emoji::EmojiOwner,
    types::reaction::ReactionKey,
    common::v1::types::document::DocumentStateVector,
    common::v1::types::document::DocumentUpdate,
    common::v1::types::document::DocumentBranch,
    common::v1::types::document::DocumentBranchState,
    common::v1::types::document::DocumentBranchListParams,
    common::v1::types::document::DocumentBranchCreate,
    common::v1::types::document::DocumentBranchPatch,
    common::v1::types::document::DocumentBranchMerge,
    common::v1::types::document::DocumentRevisionRef,
    common::v1::types::document::DocumentRevisionId,
    common::v1::types::document::DocumentMediaAttach,
    common::v1::types::document::DocumentTag,
    common::v1::types::document::DocumentTagCreate,
    common::v1::types::document::DocumentTagPatch,
    common::v1::types::document::HistoryParams,
    common::v1::types::document::Changeset,
    common::v1::types::document::HistoryPagination,
    common::v1::types::document::SerdocPut,
    common::v1::types::document::DocumentPatch,
    common::v1::types::document::Wiki,
    common::v1::types::document::WikiPatch,
    common::v1::types::document::serialized::Serdoc,
    // ack types
    common::v1::types::ack::AckCreate,
    common::v1::types::ack::AckBulk,
    common::v1::types::ack::AckBulkItem,
    // session types
    types::SessionToken,
    // auth types
    common::v1::types::auth::WebauthnAuthenticator,
    common::v1::types::auth::TotpRecoveryCode,
    // reaction types
    common::v1::types::reaction::ReactionListItem,
    // message types
    common::v1::types::message::PinsReorderItem,
    common::v1::types::message::RepliesResponse,
    // push types
    common::v1::types::push::PushCreate,
    common::v1::types::push::PushInfo,
    common::v1::types::push::PushCreateKeys,
    // room template types
    common::v1::types::room_template::RoomTemplate,
    common::v1::types::room_template::RoomTemplateCode,
    common::v1::types::room_template::RoomTemplateSnapshot,
    common::v1::types::room_template::RoomTemplateChannel,
    common::v1::types::room_template::RoomTemplateRole,
    // search types
    common::v1::types::search::RoomSearchOrderField,
    common::v1::types::search::MessageSearchOrderField,
    common::v1::types::search::ChannelSearchOrderField,
    common::v1::types::search::MediaSearchOrderField,
    common::v1::types::search::AuditLogSearchOrderField,
    common::v1::types::search::UserSearchOrderField,
    common::v1::types::search::Order,
    // room analytics types
    common::v1::types::room_analytics::Aggregation,
    common::v1::types::room_analytics::AnalyticsInvitesOrigin,
    common::v1::types::room_analytics::AnalyticsChannel,
    common::v1::types::room_analytics::AnalyticsInvites,
    common::v1::types::room_analytics::AnalyticsMembersCount,
    common::v1::types::room_analytics::AnalyticsMembersJoin,
    common::v1::types::room_analytics::AnalyticsMembersLeave,
    common::v1::types::room_analytics::AnalyticsOverview,
    // application/integration types
    common::v1::types::application::Integration,
    // moderation types
    common::v1::types::moderation::ReportReason,
    common::v1::types::moderation::ReportDestination,
    // automod types
    common::v1::types::automod::AutomodRule,
    common::v1::types::automod::AutomodRuleCreate,
    common::v1::types::automod::AutomodTrigger,
    common::v1::types::automod::AutomodAction,
    common::v1::types::automod::AutomodTarget,
    // tag types
    common::v1::types::tag::Tag,
    common::v1::types::tag::TagCreate,
    common::v1::types::tag::TagPatch,
    // server types
    common::v1::types::server::ServerAutomodList,
    common::v1::types::server::ServerMediaScanner,
    // federation types
    common::v1::types::federation::ServerKey,
    // user connection types
    common::v1::types::user_connection::ConnectionMetadata,
    common::v1::types::user_connection::ConnectionValue,
    common::v1::types::user_connection::ConnectionVisibility,
    // user relationship types
    types::Relationship,
    common::v1::types::user::Ignore,
    common::v1::types::user::RelationshipType,
    // room member types
    types::RoomMemberOrigin,
    common::v1::types::room_member::RoomMemberSearchResponse,
    // harvest types
    common::v1::types::harvest::Harvest,
    common::v1::types::harvest::HarvestCreateUser,
    common::v1::types::harvest::HarvestCreateRoom,
    common::v1::types::harvest::HarvestStatus,
    // auth password types
    common::v1::types::auth::PasswordExec,
    common::v1::types::auth::PasswordExecIdent,
    // user search types
    common::v1::types::user::UserSearch,
    common::v1::types::user::UserSearchSortField,
    // relationship types
    common::v1::types::user::RelationshipWithUserId,
    common::v1::types::user::UserWithRelationship,
    // component types
    common::v1::types::components::ComponentId,
    common::v1::types::components::ComponentCustomId,
    common::v1::types::components::ButtonStyle,
    common::v1::types::components::Component<common::v1::types::components::Create>,
    common::v1::types::components::Component<common::v1::types::components::Canonical>,
    common::v1::types::components::Component<common::v1::types::components::Encrypted>,
    common::v1::types::components::ComponentType<common::v1::types::components::Create>,
    common::v1::types::components::ComponentType<common::v1::types::components::Canonical>,
    common::v1::types::components::ComponentType<common::v1::types::components::Encrypted>,
    common::v1::types::components::Components<common::v1::types::components::Create>,
    common::v1::types::components::Components<common::v1::types::components::Canonical>,
    common::v1::types::components::Components<common::v1::types::components::Encrypted>,
    // flume types
    common::v1::types::message::flume::FlumeCreate,
    common::v1::types::message::flume::FlumeDelta,
    common::v1::types::message::flume::FlumeAppend,
    common::v1::types::message::flume::FlumeReplace,
    common::v1::types::message::flume::FlumeState,
    common::v1::types::message::flume::MessageFlume,
    // script types
    common::v1::types::redex::Redex,
    common::v1::types::redex::RedexCreate,
    common::v1::types::redex::RedexContentUpdate,
    common::v1::types::redex::RedexVersion,
    common::v1::types::redex::RedexStatus,
    common::v1::types::redex::RedexFormat,
    common::v1::types::redex::RedexLocation,
    common::v1::types::redex::RedexLocationUpdate,
    common::v1::types::redex::RedexMetadata,
    common::v1::types::redex::RedexHandler,
    common::v1::types::redex::RedexHandlerType,
    common::v1::types::redex::RedexCapability,
    common::v1::types::redex::RedexPermission,
    common::v1::types::redex::RedexPermissionGrant,
    common::v1::types::redex::RedexVersionStatus,
    common::v1::types::redex::Eval,
    common::v1::types::redex::EvalStatus,
    common::v1::types::redex::EvalLogEntry,
    common::v1::types::redex::EvalCreateManual,
    common::v1::types::redex::RedexDependency,
    common::v1::types::redex::RedexDependencyLink,
    common::v1::types::redex::RedexDependencyGraph,
    common::v1::types::redex::RedexDependenciesUpdate,
    common::v1::types::redex::EvalLogLevel,
    common::v1::types::redex::EvalLogSource,
    // media types
    common::v2::types::media::Media,
    common::v2::types::media::MediaReference,
    common::v2::types::media::MediaStatus,
    common::v2::types::media::MediaMetadata,
    common::v2::types::media::MediaScan,
    common::v2::types::media::MediaQuarantine,
    common::v2::types::media::MediaCreate,
    // interactions
    common::v1::types::interactions::InteractionCreate,
    common::v1::types::interactions::InteractionCreateType,
    common::v1::types::interactions::Interaction,
    common::v1::types::interactions::InteractionType,
    common::v1::types::interactions::InteractionResponseCreate,
    common::v1::types::interactions::InteractionResponseCreateType,
    common::v1::types::interactions::InteractionResponse,
    // voice types
    common::v1::types::voice::messages::SignallingEvent,
    common::v1::types::voice::messages::SignallingCommand,
)),
modifiers(&BadgeModifier, &NestedTags),
info(
    title = "api doccery",
    description = include_str!("../../../docs/index.md"),
),
tags(
    (name = "sync", description = include_str!("../../../docs/sync.md")),
    (name = "auth", description = include_str!("../../../docs/auth.md")),
),
)]
pub struct ApiDoc;

pub struct BadgeModifier;

pub struct NestedTags;

impl Modify for BadgeModifier {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        for (_path, path_item) in openapi.paths.paths.iter_mut() {
            let mut ops = vec![];
            if let Some(op) = &mut path_item.head {
                ops.push(op);
            }
            if let Some(op) = &mut path_item.get {
                ops.push(op);
            }
            if let Some(op) = &mut path_item.post {
                ops.push(op);
            }
            if let Some(op) = &mut path_item.put {
                ops.push(op);
            }
            if let Some(op) = &mut path_item.patch {
                ops.push(op);
            }
            if let Some(op) = &mut path_item.delete {
                ops.push(op);
            }

            for op in ops {
                let mut badges = Vec::new();
                let mut perms = Vec::new();
                let mut optional_perms = Vec::new();
                let mut server_perms = Vec::new();
                let mut server_optional_perms = Vec::new();
                let mut scopes = Vec::new();
                let mut optional_scopes = Vec::new();
                let mut audit_log_entry_types = Vec::new();

                if let Some(tags) = &mut op.tags {
                    tags.retain(|tag| {
                        if tag == "badge.admin_only" {
                            badges.push("server admins".to_string());
                            false
                        } else if tag == "badge.sudo" {
                            badges.push("requires sudo".to_string());
                            false
                        } else if tag == "badge.room-mfa" {
                            badges.push("requires mfa".to_string());
                            false
                        } else if tag == "badge.room-mfa-opt" {
                            badges.push("optional mfa".to_string());
                            false
                        } else if tag == "badge.room-sudo" {
                            badges.push("optional sudo".to_string());
                            false
                        } else if let Some(perm) = tag.strip_prefix("badge.perm.") {
                            perms.push(perm.to_string());
                            false
                        } else if tag == "badge.internal" {
                            badges.push("internal".to_string());
                            false
                        } else if tag == "badge.public" {
                            badges.push("public".to_string());
                            false
                        } else if tag == "badge.unauthenticated" {
                            badges.push("unauthenticated".to_string());
                            false
                        } else if let Some(perm) = tag.strip_prefix("badge.perm-opt.") {
                            optional_perms.push(perm.to_string());
                            false
                        } else if let Some(server_perm_req) = tag.strip_prefix("badge.server-perm.")
                        {
                            server_perms.push(server_perm_req.to_string());
                            false
                        } else if let Some(server_perm_opt) =
                            tag.strip_prefix("badge.server-perm-opt.")
                        {
                            server_optional_perms.push(server_perm_opt.to_string());
                            false
                        } else if let Some(scope) = tag.strip_prefix("badge.scope.") {
                            scopes.push(scope.to_string());
                            false
                        } else if let Some(scope) = tag.strip_prefix("badge.scope-opt.") {
                            optional_scopes.push(scope.to_string());
                            false
                        } else if let Some(audit_log_type) = tag.strip_prefix("badge.audit-log.") {
                            audit_log_entry_types.push(audit_log_type.to_string());
                            false
                        } else {
                            true
                        }
                    });
                }

                let x_badges = op
                    .extensions
                    .get_or_insert_with(|| {
                        utoipa::openapi::extensions::Extensions::builder().build()
                    })
                    .entry("x-badges".to_string())
                    .or_insert_with(|| json!([]))
                    .as_array_mut()
                    .unwrap();

                for badge in badges {
                    x_badges.push(json!({
                        "name": badge,
                        "position": "before",
                    }));
                }

                let mut requirements_formatted = vec![];

                for perm in perms {
                    requirements_formatted.push(format!(
                        r#"<div class="markdown-alert-permission-required">{perm}</div>"#
                    ));
                }

                for perm in optional_perms {
                    requirements_formatted.push(format!(
                        r#"<div class="markdown-alert-permission-optional">{perm}</div>"#
                    ));
                }

                for server_perm in server_perms {
                    requirements_formatted.push(format!(
                        r#"<div class="markdown-alert-server-permission-required">server:{server_perm}</div>"#
                    ));
                }

                for server_perm in server_optional_perms {
                    requirements_formatted.push(format!(
                        r#"<div class="markdown-alert-server-permission-optional">server:{server_perm}</div>"#
                    ));
                }

                for scope in scopes {
                    requirements_formatted.push(format!(
                        r#"<div class="markdown-alert-scope-required">{scope}</div>"#
                    ));
                }

                for scope in optional_scopes {
                    requirements_formatted.push(format!(
                        r#"<div class="markdown-alert-scope-optional">{scope}</div>"#
                    ));
                }

                for audit_log_type in audit_log_entry_types {
                    requirements_formatted.push(format!(
                        r#"<div class="markdown-alert-audit-log">creates audit log entry of type: {audit_log_type}</div>"#
                    ));
                }

                let desc = op.description.get_or_insert_default();
                *desc = format!("{}\n\n{desc}", requirements_formatted.join("\n"));
            }
        }
    }
}

impl Modify for NestedTags {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let tag_groups = json!([
            {
                "name": "auth",
                "description": "authentication and session management",
                "tags": ["session", "auth"],
            },
            {
                "name": "room",
                "description": "working with rooms",
                "tags": ["room", "room_member", "room_template", "room_analytics", "role", "emoji", "automod"],
            },
            {
                "name": "channel",
                "description": "working with channels",
                "tags": ["channel", "thread", "tag", "message", "reaction", "voice", "calendar", "document", "flume", "redex"],
            },
            {
                "name": "user",
                "description": "working with users",
                "tags": ["user", "user_email", "preferences", "relationship", "dm", "inbox", "push"],
            },
            {
                "name": "integrations",
                "description": "working with third party services",
                "tags": ["application", "oauth", "webhook", "user_connection"],
            },
            {
                "name": "other",
                "description": "the rest of the routes",
                "tags": ["debug", "invite", "media", "moderation", "sync", "search", "public", "admin", "ack", "e2ee", "federation", "server"],
            },
        ]);

        if let Some(tags) = &mut openapi.tags {
            for tag in tags {
                tag.extensions
                    .get_or_insert_with(|| {
                        utoipa::openapi::extensions::Extensions::builder().build()
                    })
                    .insert(
                        "x-displayName".to_string(),
                        tag.name.replace("_", " ").into(),
                    );
            }
        }

        openapi
            .extensions
            .get_or_insert_default()
            .merge(Extensions::builder().add("x-tagGroups", tag_groups).build());
    }
}
