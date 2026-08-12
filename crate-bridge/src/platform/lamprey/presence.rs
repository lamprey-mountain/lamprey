use std::collections::HashMap;
use std::time::Duration;

use common::v1::types::misc::UserIdReq;
use common::v1::types::presence::{Presence, Status};
use common::v2::types::UserId;
use futures::StreamExt;
use sdk::http::Http;
use tokio::sync::mpsc;
use tokio_util::time::{DelayQueue, delay_queue};

pub enum PresenceEvent {
    Update(UserId, Presence),
}

pub struct PresenceRefreshActor {
    http: Http,
    queue: DelayQueue<UserId>,
    presence_data: HashMap<UserId, Presence>,
    presence_keys: HashMap<UserId, delay_queue::Key>,
}

impl PresenceRefreshActor {
    pub fn spawn(http: Http) -> mpsc::UnboundedSender<PresenceEvent> {
        let (tx, rx) = mpsc::unbounded_channel();
        let actor = Self {
            http,
            queue: DelayQueue::new(),
            presence_data: HashMap::new(),
            presence_keys: HashMap::new(),
        };
        tokio::spawn(actor.run(rx));
        tx
    }

    fn send_presence(&self, user_id: UserId, presence: Presence) {
        let http = self
            .http
            .for_puppet(user_id)
            .expect("Failed to get puppet http client");

        // TODO: supervise this task, log on request failure
        tokio::spawn(async move {
            let _ = http
                .user_set_presence(UserIdReq::UserId(user_id), &presence)
                .await;
        });
    }

    async fn run(mut self, mut cmd_rx: mpsc::UnboundedReceiver<PresenceEvent>) {
        loop {
            tokio::select! {
                event = cmd_rx.recv() => {
                    match event {
                        Some(PresenceEvent::Update(user_id, presence)) => {
                            self.send_presence(user_id, presence.clone());

                            if presence.status == Status::Offline {
                                if let Some(key) = self.presence_keys.remove(&user_id) {
                                    self.queue.remove(&key);
                                }
                                self.presence_data.remove(&user_id);
                            } else {
                                if let Some(key) = self.presence_keys.remove(&user_id) {
                                    self.queue.remove(&key);
                                }
                                let key = self.queue.insert(user_id, Duration::from_secs(180));
                                self.presence_keys.insert(user_id, key);
                                self.presence_data.insert(user_id, presence);
                            }
                        }
                        None => break,
                    }
                }
                Some(expired) = self.queue.next() => {
                    let user_id = expired.into_inner();
                    if let Some(presence) = self.presence_data.get(&user_id).cloned() {
                        self.send_presence(user_id, presence.clone());
                        let key = self.queue.insert(user_id, Duration::from_secs(180));
                        self.presence_keys.insert(user_id, key);
                    }
                }
            }
        }
    }
}
