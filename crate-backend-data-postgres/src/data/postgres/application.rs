use async_trait::async_trait;
use common::v1::types::{
    application::Application,
    error::{ApiError, ErrorCode},
    ApplicationId, PaginationDirection, PaginationQuery, PaginationResponse, UserId,
};
use lamprey_backend_core::Error;
use sqlx::{query, query_scalar, Acquire};

use crate::{
    data::{postgres::Pagination, DataApplication},
    gen_paginate,
};

use super::Postgres;
use crate::Result;

#[async_trait]
impl DataApplication for Postgres {
    async fn application_insert(&self, app: Application) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        query!(
            r#"
            insert into application (id, owner_id, name, description, public, oauth_secret, oauth_redirect_uris, oauth_confidential)
            values ($1, $2, $3, $4, $5, $6, $7, $8)
            "#,
            *app.id,
            *app.owner_id,
            app.name,
            app.description,
            app.public,
            app.oauth_secret,
            serde_json::to_value(app.oauth_redirect_uris).unwrap(),
            app.oauth_confidential,
        )
        .execute(&mut *tx)
        .await?;

        if let Some(bridge) = app.bridge {
            query!(
                r#"
                insert into application_bridge (application_id, platform_name, platform_url, platform_description)
                values ($1, $2, $3, $4)
                "#,
                *app.id,
                bridge.platform_name,
                bridge.platform_url,
                bridge.platform_description,
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn application_update(&self, app: Application) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        query!(
            r#"
            update application set
                name = $2,
                description = $3,
                public = $4,
                oauth_secret = $5,
                oauth_redirect_uris = $6,
                oauth_confidential = $7
            where id = $1
            "#,
            *app.id,
            app.name,
            app.description,
            app.public,
            app.oauth_secret,
            serde_json::to_value(app.oauth_redirect_uris).unwrap(),
            app.oauth_confidential,
        )
        .execute(&mut *tx)
        .await?;

        if let Some(bridge) = app.bridge {
            query!(
                r#"
                insert into application_bridge (application_id, platform_name, platform_url, platform_description)
                values ($1, $2, $3, $4)
                on conflict (application_id) do update set
                    platform_name = $2,
                    platform_url = $3,
                    platform_description = $4
                "#,
                *app.id,
                bridge.platform_name,
                bridge.platform_url,
                bridge.platform_description,
            )
            .execute(&mut *tx)
            .await?;
        } else {
            query!(
                "delete from application_bridge where application_id = $1",
                *app.id
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn application_delete(&self, id: ApplicationId) -> Result<()> {
        query!(
            "UPDATE application SET deleted_at = now() WHERE id = $1",
            *id
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn application_get(&self, id: ApplicationId) -> Result<Application> {
        let app = query!(
            r#"
        	SELECT
                a.id, a.owner_id, a.name, a.description, a.public, a.oauth_secret, a.oauth_redirect_uris, a.oauth_confidential,
                b.application_id as "bridge_id?", b.platform_name, b.platform_url, b.platform_description
            FROM application a
            LEFT JOIN application_bridge b ON a.id = b.application_id
        	WHERE a.id = $1
            "#,
            *id
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => Error::ApiError(ApiError::from_code(
                ErrorCode::UnknownApplication,
            )),
            e => Error::Sqlx(e),
        })?;

        let bridge = if app.bridge_id.is_some() {
            Some(common::v1::types::application::Bridge {
                platform_name: app.platform_name,
                platform_url: app.platform_url,
                platform_description: app.platform_description,
            })
        } else {
            None
        };

        Ok(Application {
            id: app.id.into(),
            owner_id: app.owner_id.into(),
            name: app.name,
            description: app.description,
            bridge,
            public: app.public,
            oauth_secret: app.oauth_secret,
            oauth_redirect_uris: serde_json::from_value(app.oauth_redirect_uris)
                .unwrap_or_default(),
            oauth_confidential: app.oauth_confidential,
        })
    }

    async fn application_list(
        &self,
        owner_id: UserId,
        q: PaginationQuery<ApplicationId>,
    ) -> Result<PaginationResponse<Application>> {
        let p: Pagination<_> = q.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query!(
                r#"
            	SELECT
                    a.id, a.owner_id, a.name, a.description, a.public, a.oauth_secret, a.oauth_redirect_uris, a.oauth_confidential,
                    b.application_id as "bridge_id?", b.platform_name, b.platform_url, b.platform_description
                FROM application a
                LEFT JOIN application_bridge b ON a.id = b.application_id
            	WHERE a.owner_id = $1 AND a.id > $2 AND a.id < $3
            	ORDER BY (CASE WHEN $4 = 'f' THEN a.id END), a.id DESC LIMIT $5
                "#,
                *owner_id,
                *p.after,
                *p.before,
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM application WHERE owner_id = $1",
                *owner_id
            ),
            |row| {
                let bridge = if row.bridge_id.is_some() {
                    Some(common::v1::types::application::Bridge {
                        platform_name: row.platform_name,
                        platform_url: row.platform_url,
                        platform_description: row.platform_description,
                    })
                } else {
                    None
                };

                Application {
                    id: row.id.into(),
                    owner_id: row.owner_id.into(),
                    name: row.name,
                    description: row.description,
                    bridge,
                    public: row.public,
                    oauth_secret: row.oauth_secret,
                    oauth_redirect_uris: serde_json::from_value(row.oauth_redirect_uris)
                        .unwrap_or_default(),
                    oauth_confidential: row.oauth_confidential,
                }
            },
            |i: &Application| i.id.to_string()
        )
    }
}
