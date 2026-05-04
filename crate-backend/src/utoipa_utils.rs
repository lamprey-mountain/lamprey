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
                "tags": ["room", "room_member", "room_template", "room_analytics", "role", "emoji", "automod"],
            },
            {
                "name": "channel",
                "description": "working with channels",
                "tags": ["channel", "thread", "tag", "message", "reaction", "voice", "calendar", "document", "flume"],
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

pub struct ComponentModifier;

impl Modify for ComponentModifier {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::schema::{ArrayBuilder, ObjectBuilder, Ref, SchemaType, Type};
        use utoipa::openapi::RefOr;

        if let Some(components) = openapi.components.as_mut() {
            let empty_obj = ObjectBuilder::new()
                .schema_type(SchemaType::Type(Type::Object))
                .build();

            // Injections
            let component_type_create = ObjectBuilder::new().build();
            components.schemas.insert(
                "ComponentType_Create".to_string(),
                RefOr::T(component_type_create.into()),
            );
            let component_type_canonical = ObjectBuilder::new().build();
            components.schemas.insert(
                "ComponentType_Canonical".to_string(),
                RefOr::T(component_type_canonical.into()),
            );
            let component_type_thin = ObjectBuilder::new().build();
            components.schemas.insert(
                "ComponentType_Thin".to_string(),
                RefOr::T(component_type_thin.into()),
            );

            let component_create = ObjectBuilder::new()
                .property("id", Ref::new("#/components/schemas/ComponentId"))
                .property("ty", Ref::new("#/components/schemas/ComponentType_Create"))
                .required("ty")
                .build();
            components.schemas.insert("Component_Create".to_string(), RefOr::T(component_create.into()));

            let component_canonical = ObjectBuilder::new()
                .property("id", Ref::new("#/components/schemas/ComponentId"))
                .property(
                    "ty",
                    Ref::new("#/components/schemas/ComponentType_Canonical"),
                )
                .required("id")
                .required("ty")
                .build();
            components.schemas.insert(
                "Component_Canonical".to_string(),
                RefOr::T(component_canonical.into()),
            );

            let component_thin = ObjectBuilder::new()
                .property("id", Ref::new("#/components/schemas/ComponentId"))
                .property("ty", Ref::new("#/components/schemas/ComponentType_Thin"))
                .required("id")
                .required("ty")
                .build();
            components
                .schemas
                .insert("Component_Thin".to_string(), RefOr::T(component_thin.into()));

            let components_create = ArrayBuilder::new()
                .items(Ref::new("#/components/schemas/Component_Create"))
                .build();
            components.schemas.insert(
                "Components_Create".to_string(),
                RefOr::T(components_create.into()),
            );

            let components_canonical = ArrayBuilder::new()
                .items(Ref::new("#/components/schemas/Component_Canonical"))
                .build();
            components.schemas.insert(
                "Components_Canonical".to_string(),
                RefOr::T(components_canonical.into()),
            );

            let components_thin = ArrayBuilder::new()
                .items(Ref::new("#/components/schemas/Component_Thin"))
                .build();
            components.schemas.insert(
                "Components_Thin".to_string(),
                RefOr::T(components_thin.into()),
            );

            components.schemas.insert(
                "BrokenGenericPlaceholder".to_string(),
                RefOr::T(empty_obj.into()),
            );
        }
    }
}
