// TODO: *maybe* extract out connection logic?
// i'm not sure how immediately useful this would be, so i'll punt this for later, if i do it at all...
// api below somewhat inspired by str0m

use lamprey::{
    v1::types::{MessageClient, MessageSync, Session},
    v2::types::ConnectionId,
};
use tokio::time::Instant;

use crate::{queue::ConnectionQueue, util::ConnectionClose};

pub struct Connection {
    id: ConnectionId,
    session: Session,
    queue: ConnectionQueue,
    // subscriptions: Box<ConnectionSubscriptions>,
    // transport: Option<ConnectionTransport>,
    // globals: Globals,
    // rx: mpsc::Receiver<Command>,
}

pub enum Output {
    Timeout(Instant),
    Send(MessageSync),
    Close(ConnectionClose),
}

pub enum Input {
    Timeout(Instant),
    Recv(MessageClient),
    Close(ConnectionClose),
}

impl Connection {
    pub fn new(id: ConnectionId, session: Session) -> Self {
        let mut conn = Self {
            id,
            session,
            queue: ConnectionQueue::new(crate::util::MAX_QUEUE_LEN),
        };

        // TODO: somehow merge pending_outputs into queue
        // pending_outputs: VecDeque::new(),

        // // Immediately request ready state boot data
        // conn.pending_outputs.push_back(Output::FetchReadyState);
        // conn
        todo!()
    }

    pub fn handle_input(&mut self, input: Input) {
        todo!()
    }

    pub fn poll_output(&mut self) -> Output {
        todo!()
    }
}

// ConnectionBuilder
// .with_id(ConnectionId)
// .with_max_queue_len(usize)
// .buuild(Session)
