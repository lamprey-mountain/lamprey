use async_trait::async_trait;
use common::v1::types::{
    application::Application, ApplicationId, PaginationDirection, PaginationQuery,
    PaginationResponse, UserId,
};
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
        query!(
            r#"
            insert into application (id, owner_id, name, description, bridge, public, oauth_secret, oauth_redirect_uris, oauth_confidential)
            values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            "#,
            *app.id,
            *app.owner_id,
            app.name,
            app.description,
            app.bridge,
            app.public,
            app.oauth_secret,
            serde_json::to_value(app.oauth_redirect_uris).unwrap(),
            app.oauth_confidential,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn application_update(&self, app: Application) -> Result<()> {
        query!(
            r#"
            update application set
                name = $2,
                description = $3,
                bridge = $4,
                public = $5,
                oauth_secret = $6,
                oauth_redirect_uris = $7,
                oauth_confidential = $8
            where id = $1
            "#,
            *app.id,
            app.name,
            app.description,
            app.bridge,
            app.public,
            app.oauth_secret,
            serde_json::to_value(app.oauth_redirect_uris).unwrap(),
            app.oauth_confidential,
        )
        .execute(&self.pool)
        .await?;
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
        	SELECT id, owner_id, name, description, bridge, public, oauth_secret, oauth_redirect_uris, oauth_confidential
            FROM application
        	WHERE id = $1
            "#,
            *id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Application {
            id: app.id.into(),
            owner_id: app.owner_id.into(),
            name: app.name,
            description: app.description,
            bridge: app.bridge,
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
            	SELECT id, owner_id, name, description, bridge, public, oauth_secret, oauth_redirect_uris, oauth_confidential
                FROM application
            	WHERE owner_id = $1 AND id > $2 AND id < $3
            	ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC LIMIT $5
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
            |row| Application {
                id: row.id.into(),
                owner_id: row.owner_id.into(),
                name: row.name,
                description: row.description,
                bridge: row.bridge,
                public: row.public,
                oauth_secret: row.oauth_secret,
                oauth_redirect_uris: serde_json::from_value(row.oauth_redirect_uris)
                    .unwrap_or_default(),
                oauth_confidential: row.oauth_confidential,
            },
            |i: &Application| i.id.to_string()
        )
    }
}
