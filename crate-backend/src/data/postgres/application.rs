use async_trait::async_trait;
use common::v1::types::{
    application::Application, ApplicationId, PaginationDirection, PaginationQuery,
    PaginationResponse, UserId,
};
use sqlx::{query, query_as, query_scalar, Acquire};

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
            insert into application (id, owner_id, name, description, bridge, public)
            values ($1, $2, $3, $4, $5, $6)
            "#,
            *app.id,
            *app.owner_id,
            app.name,
            app.description,
            app.bridge,
            app.public,
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
                public = $5
            where id = $1
            "#,
            *app.id,
            app.name,
            app.description,
            app.bridge,
            app.public,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn application_delete(&self, id: ApplicationId) -> Result<()> {
        query_as!(Application, r#"DELETE FROM application WHERE id = $1"#, *id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn application_get(&self, id: ApplicationId) -> Result<Application> {
        let app = query_as!(
            Application,
            r#"
        	SELECT id, owner_id, name, description, bridge, public
            FROM application
        	WHERE id = $1
            "#,
            *id
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(app)
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
            query_as!(
                Application,
                r#"
            	SELECT id, owner_id, name, description, bridge, public
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
            |i: &Application| i.id.to_string()
        )
    }
}
