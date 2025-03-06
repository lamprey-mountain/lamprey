use std::sync::Arc;

use moka::future::Cache;
use types::{Session, SessionId, SessionToken};

use crate::{Result, ServerStateInner};

pub struct ServiceSessions {
    state: Arc<ServerStateInner>,
    cache_sessions: Cache<SessionId, Session>,
    // is it worth duplicating Sessions here? maybe
    cache_tokens: Cache<SessionToken, Session>,
}

impl ServiceSessions {
    pub fn new(state: Arc<ServerStateInner>) -> Self {
        Self {
            state,
            cache_sessions: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
            cache_tokens: Cache::builder()
                .max_capacity(100_000)
                .support_invalidation_closures()
                .build(),
        }
    }

    pub async fn get(&self, session_id: SessionId) -> Result<Session> {
        self.cache_sessions
            .try_get_with(session_id, self.state.data().session_get(session_id))
            .await
            .map_err(|err| err.fake_clone())
    }

    pub async fn get_by_token(&self, token: SessionToken) -> Result<Session> {
        let s = self
            .cache_tokens
            .try_get_with(token.clone(), self.state.data().session_get_by_token(token))
            .await
            .map_err(|err| err.fake_clone())?;
        // self.cache_sessions.insert(s.id, s.clone()).await;
        Ok(s)
    }

    pub async fn invalidate(&self, session_id: SessionId) {
        self.cache_sessions.invalidate(&session_id).await;
        let _ = self
            .cache_tokens
            .invalidate_entries_if(move |_, s| s.id == session_id);
    }
}
