use async_trait::async_trait;
use common::v1::types::{MessagePayload, MessageSync, Session, User};
use std::future::{ready, Future};

#[allow(unused_variables)]
pub trait EventHandler: Send {
    type Error: Send;

    fn ready(
        &mut self,
        user: Option<User>,
        session: Session,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn error(&mut self, err: String) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }

    fn sync(&mut self, msg: MessageSync) -> impl Future<Output = Result<(), Self::Error>> + Send {
        ready(Ok(()))
    }
}

pub struct EmptyHandler;

impl EventHandler for EmptyHandler {
    type Error = ();
}

#[async_trait]
pub trait ErasedHandler: Send {
    async fn handle(&mut self, payload: MessagePayload);
}

#[async_trait]
impl<T, E> ErasedHandler for T
where
    T: EventHandler<Error = E>,
{
    async fn handle(&mut self, payload: MessagePayload) {
        let _ = match payload {
            MessagePayload::Sync { data, .. } => self.sync(data).await,
            MessagePayload::Error { error } => self.error(error).await,
            MessagePayload::Ready { user, session, .. } => self.ready(user, session).await,
            _ => return,
        };
    }
}
