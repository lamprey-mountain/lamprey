use async_trait::async_trait;
use common::v1::types::{
    application::{Application, Connection, Scopes},
    ApplicationId, PaginationDirection, PaginationQuery, PaginationResponse, UserId,
};
use sqlx::{query, query_as, query_scalar, Acquire};

use crate::{
    data::{postgres::Pagination, DataConnection},
    error::Result,
    gen_paginate,
};

use super::Postgres;

struct DbConnection {
    application_id: uuid::Uuid,
    scopes: serde_json::Value,
    created_at: time::PrimitiveDateTime,
    app_owner_id: uuid::Uuid,
    app_name: String,
    app_description: Option<String>,
    app_public: bool,
    app_oauth_secret: Option<String>,
    app_oauth_redirect_uris: serde_json::Value,
    app_oauth_confidential: bool,
    bridge_platform_name: Option<String>,
    bridge_platform_url: Option<String>,
    bridge_platform_description: Option<String>,
}

impl From<DbConnection> for Connection {
    fn from(val: DbConnection) -> Self {
        let bridge = if val.bridge_platform_name.is_some() {
            Some(common::v1::types::application::Bridge {
                platform_name: val.bridge_platform_name,
                platform_url: val.bridge_platform_url,
                platform_description: val.bridge_platform_description,
            })
        } else {
            None
        };

        Connection {
            application: Application {
                id: val.application_id.into(),
                owner_id: val.app_owner_id.into(),
                name: val.app_name,
                description: val.app_description,
                bridge,
                public: val.app_public,
                oauth_secret: val.app_oauth_secret,
                oauth_redirect_uris: serde_json::from_value(val.app_oauth_redirect_uris)
                    .unwrap_or_default(),
                oauth_confidential: val.app_oauth_confidential,
            },
            scopes: serde_json::from_value(val.scopes).unwrap_or_default(),
            created_at: val.created_at.into(),
        }
    }
}

#[async_trait]
impl DataConnection for Postgres {
    async fn connection_create(
        &self,
        user_id: UserId,
        application_id: ApplicationId,
        scopes: Scopes,
    ) -> Result<()> {
        query!(
            r#"
            insert into connection (user_id, application_id, scopes, created_at)
            values ($1, $2, $3, now())
            on conflict (user_id, application_id) do update set
                scopes = excluded.scopes
            "#,
            *user_id,
            *application_id,
            serde_json::to_value(&scopes).unwrap(),
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn connection_get(
        &self,
        user_id: UserId,
        application_id: ApplicationId,
    ) -> Result<Connection> {
        let conn = query_as!(
            DbConnection,
            r#"
            select
                c.application_id, c.scopes as scopes, c.created_at,
                a.owner_id as app_owner_id, a.name as app_name, a.description as app_description,
                a.public as app_public, a.oauth_secret as app_oauth_secret,
                a.oauth_redirect_uris as app_oauth_redirect_uris, a.oauth_confidential as app_oauth_confidential,
                b.platform_name as "bridge_platform_name?", b.platform_url as "bridge_platform_url?", b.platform_description as "bridge_platform_description?"
            from connection c
            join application a on c.application_id = a.id
            left join application_bridge b on a.id = b.application_id
            where c.user_id = $1 and c.application_id = $2
            "#,
            *user_id,
            *application_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(conn.into())
    }

    async fn connection_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<ApplicationId>,
    ) -> Result<PaginationResponse<Connection>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbConnection,
                r#"
                select
                    c.application_id, c.scopes as scopes, c.created_at,
                    a.owner_id as app_owner_id, a.name as app_name, a.description as app_description,
                    a.public as app_public, a.oauth_secret as app_oauth_secret,
                    a.oauth_redirect_uris as app_oauth_redirect_uris, a.oauth_confidential as app_oauth_confidential,
                    b.platform_name as "bridge_platform_name?", b.platform_url as "bridge_platform_url?", b.platform_description as "bridge_platform_description?"
                from connection c
                join application a on c.application_id = a.id
                left join application_bridge b on a.id = b.application_id
                where c.user_id = $1 and c.application_id > $2 and c.application_id < $3
                order by (case when $4 = 'f' then c.application_id end), c.application_id desc limit $5
                "#,
                *user_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "select count(*) from connection where user_id = $1",
                *user_id
            ),
            |i: &Connection| i.application.id.to_string()
        )
    }

    async fn connection_delete(
        &self,
        user_id: UserId,
        application_id: ApplicationId,
    ) -> Result<()> {
        query!(
            "delete from connection where user_id = $1 and application_id = $2",
            *user_id,
            *application_id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
