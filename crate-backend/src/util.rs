use http::header;
use serde_json::json;
use tower_http::cors::CorsLayer;
use utoipa::{openapi::extensions::Extensions, Modify};

pub struct BadgeModifier;

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

                if let Some(tags) = &mut op.tags {
                    tags.retain(|tag| {
                        if tag == "badge.admin_only" {
                            badges.push("server admins".to_string());
                            false
                        } else if tag == "badge.sudo" {
                            badges.push("requires sudo".to_string());
                            false
                        } else if let Some(perm) = tag.strip_prefix("badge.perm.") {
                            perms.push(perm.to_string());
                            false
                        } else if let Some(perm) = tag.strip_prefix("badge.perm-opt.") {
                            optional_perms.push(perm.to_string());
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

                let mut perms_formatted = vec![];

                for perm in perms {
                    perms_formatted.push(format!(
                        r#"<div class="markdown-alert-permission-required">{perm}</div>"#
                    ));
                }

                for perm in optional_perms {
                    perms_formatted.push(format!(
                        r#"<div class="markdown-alert-permission-optional">{perm}</div>"#
                    ));
                }

                let desc = op.description.get_or_insert_default();
                *desc = format!("{}\n\n{desc}", perms_formatted.join("\n"));
            }
        }
    }
}

pub struct NestedTags;

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
                "tags": ["room", "room_member", "room_template", "room_analytics", "role", "emoji", "tag", "automod"],
            },
            {
                "name": "channel",
                "description": "working with channels",
                "tags": ["channel", "thread", "tag", "message", "reaction", "voice", "calendar"],
            },
            {
                "name": "user",
                "description": "working with users",
                "tags": ["user", "user_email", "user_config", "relationship", "dm", "inbox"],
            },
            {
                "name": "integrations",
                "description": "working with third party services",
                "tags": ["application", "oauth", "webhook"],
            },
            {
                "name": "other",
                "description": "the rest of the routes",
                "tags": ["debug", "invite", "media", "moderation", "sync", "search", "public", "admin"],
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

pub fn cors() -> CorsLayer {
    use header::{HeaderName, AUTHORIZATION, CONTENT_TYPE};
    const UPLOAD_OFFSET: HeaderName = HeaderName::from_static("upload-offset");
    const UPLOAD_LENGTH: HeaderName = HeaderName::from_static("upload-length");
    const IDEMPOTENCY_KEY: HeaderName = HeaderName::from_static("idempotency-key");
    const REASON: HeaderName = HeaderName::from_static("x-reason");
    const PUPPET_ID: HeaderName = HeaderName::from_static("x-puppet-id");
    CorsLayer::very_permissive()
        .expose_headers([CONTENT_TYPE, UPLOAD_OFFSET, UPLOAD_LENGTH])
        .allow_headers([
            AUTHORIZATION,
            CONTENT_TYPE,
            UPLOAD_OFFSET,
            UPLOAD_LENGTH,
            IDEMPOTENCY_KEY,
            REASON,
            PUPPET_ID,
        ])
}
