use std::time::Duration;

use axum::http::request::Parts;
use common::{
    v1::types::{
        Session, SessionImprint, SessionStatus, SessionToken, SessionType,
        oauth::{Scope, Scopes},
        util::Time,
    },
    v2::types::{SERVER_TOKEN_SESSION_ID, SERVER_USER_ID},
};
use kerosene_core::{error::ErrorCode, types::auth::Identity};

use crate::{prelude::*, util::headers::HeadersRequest};

pub async fn calculate(headers: &HeadersRequest, globals: &Globals) -> Result<Identity> {
    let srv = globals.services();

    // bearer authorization token
    if let Some(auth) = &headers.authorization {
        let token = auth.token();

        // static admin token, used for initial setup and maintenence
        let session = if srv.admin.verify_admin_token(token).await {
            let user = srv.users.get(SERVER_USER_ID, None).await.cast_internal()?;
            let session = Session {
                id: SERVER_TOKEN_SESSION_ID,
                status: SessionStatus::Sudo {
                    user_id: SERVER_USER_ID,
                    sudo_expires_at: Time::now_utc() + Duration::from_secs(3600),
                },
                name: Some("admin token".to_string()),
                ty: SessionType::User,
                expires_at: None,
                app_id: None,
                imprint: SessionImprint {
                    last_seen_at: Time::now_utc(),
                    ip_addr: None,
                    country_code: None,
                    country_name: None,
                    city_name: None,
                    user_agent: None,
                },
                authorized_at: Some(Time::now_utc()),
                deauthorized_at: None,
            };

            // TODO(?): puppetting another user as the server
            return Ok(Identity::User {
                user,
                session,
                scopes: Scopes(vec![Scope::Full]),
            });
        } else {
            // user session
            let session = srv
                .sessions
                .get_by_token(SessionToken(token.to_string()))
                .await
                .cast_internal()?;
            if session.expires_at.is_some_and(|t| t < Time::now_utc()) {
                return Err(ApiError::from_code(ErrorCode::MissingAuth).into());
            }

            // session imprint update
            if session.imprint.last_seen_at < Time::now_utc() - Duration::from_secs(60) {
                let geo = headers
                    .ip_addr
                    .and_then(|ip| srv.ips.lookup(ip).ok().flatten());

                // PERF: run this in the background, debounce
                let mut txn = globals.begin().await.cast_internal()?;
                txn.session_update_imprint(
                    session.id,
                    SessionImprint {
                        last_seen_at: Time::now_utc(),
                        ip_addr: headers.ip_addr.map(|i| i.to_string()),
                        country_code: geo.as_ref().and_then(|g| g.country_code.clone()),
                        country_name: geo.as_ref().and_then(|g| g.country_name.clone()),
                        city_name: geo.as_ref().and_then(|g| g.city_name.clone()),
                        user_agent: headers.user_agent.clone(),
                    },
                )
                .await
                .cast_internal()?;
                txn.commit().await.cast_internal()?;
            }

            let real_user = if let Some(user_id) = session.user_id() {
                srv.users.get(user_id, None).await.cast_internal()?
            } else {
                let scopes = if session.ty == SessionType::User {
                    Scopes(vec![Scope::Auth])
                } else {
                    Scopes::default()
                };

                return Ok(Identity::Guest {
                    session: (*session).to_owned(),
                    scopes,
                });
            };

            // FIXME: puppetting
            let acting_user = real_user.clone();

            // scopes
            let scopes = if session.ty == SessionType::User {
                Scopes(vec![Scope::Auth])
            } else if let Some(app_id) = session.app_id {
                let mut txn = globals.begin_read().await.cast_internal()?;
                txn.connection_get(real_user.id, app_id)
                    .await
                    .map(|c| c.scopes)
                    .unwrap_or_default()
            } else {
                Scopes::default()
            };

            if acting_user.id != real_user.id {
                return Ok(Identity::Puppet {
                    puppet: acting_user,
                    puppeteer: real_user,
                    session: (*session).to_owned(),
                    scopes,
                });
            } else {
                return Ok(Identity::User {
                    user: real_user,
                    session: (*session).to_owned(),
                    scopes,
                });
            }
        };
    }

    // FIXME: federation auth
    // // federation auth
    // if let Some(FederationIdentity(origin)) =
    //     parts.extensions.get::<FederationIdentity>().cloned()
    // {
    //     let puppet = if let Some(puppet_id) = headers.puppet_id {
    //         let user = srv.users.get(puppet_id, None).await?;
    //         if user.remote.as_ref().is_some_and(|r| r.hostname == origin) {
    //             Some(user)
    //         } else {
    //             return Err(Error::ApiError(ApiError::from_code(
    //                 ErrorCode::CannotManageRemoteUser,
    //             )));
    //         }
    //     } else {
    //         None
    //     };

    //     return Ok(Auth4 {
    //         identity: Identity::Server {
    //             hostname: origin,
    //             puppet,
    //         },
    //         reason,
    //         audit_txn_slot,
    //         room_id: None,
    //         ty: None,
    //     });
    // }

    // public endpoints
    Ok(Identity::Public)
}
