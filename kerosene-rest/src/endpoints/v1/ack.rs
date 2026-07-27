use std::collections::HashSet;

use common::v1::types::{MessageSync, ack::AckState, oauth::Scope};
use tracing::warn;

use crate::prelude::*;

// FIXME: don't use internal errors everywhere
// .map_err(|e| ServerError::Internal(Box::new(e)))?;

#[handler(routes::ack_bulk)]
async fn bulk(req: Req<routes::ack_bulk::Endpoint>) -> Result<routes::ack_bulk::Response> {
    let user = req.identity().ensure_user()?;
    req.identity().ensure_scopes(&[Scope::Full])?;

    let srv = req.services();

    let mut channel_ids = HashSet::new();
    let mut unknown_auth = false;

    for ack in &req.inner().body.acks {
        if let Some(channel_id) = ack.ty.channel_id() {
            channel_ids.insert(channel_id);
        } else {
            unknown_auth = true;
        }
    }

    if unknown_auth {
        warn!("unknown auth check for this ack type, allowing");
    }

    for &channel_id in &channel_ids {
        srv.perms
            .for_channel3(Some(user.id), channel_id)
            .await
            .map_err(|e| ServerError::Internal(Box::new(e)))?
            .ensure_view()
            .map_err(|e| ServerError::Internal(Box::new(e)))?
            .check()
            .map_err(|e| ServerError::Internal(Box::new(e)))?;
    }

    if !req.inner().body.acks.is_empty() {
        let mut txn = req
            .globals()
            .begin()
            .await
            .map_err(|e| ServerError::Internal(Box::new(e)))?;
        txn.unread_ack_bulk(user.id, &req.inner().body.acks)
            .await
            .map_err(|e| ServerError::Internal(Box::new(e)))?;
        txn.commit()
            .await
            .map_err(|e| ServerError::Internal(Box::new(e)))?;

        for &channel_id in &channel_ids {
            srv.channels.invalidate_user(channel_id, user.id).await;
        }

        req.globals()
            .messaging()
            .broadcast_user(
                user.id,
                MessageSync::PassiveAck {
                    user_id: user.id,
                    ack_states: req
                        .inner()
                        .body
                        .acks
                        .iter()
                        .map(|a| AckState {
                            ty: a.ty.clone(),
                            unread: false,
                        })
                        .collect(),
                },
            )
            .await
            .map_err(|e| ServerError::Internal(Box::new(e)))?;
    }

    Ok(routes::ack_bulk::Response {})
}
