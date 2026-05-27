use crate::services::voice::voice_state::VoiceStateHandle;
use crate::services::voice::{ServiceVoice, SfuCommand, SfuStats};
use crate::Result;
use axum::extract::ws::WebSocket;
use common::v1::types::error::{ApiError, ErrorCode};
use common::v1::types::voice::messages::{SfuEvent, SignallingEvent};
use common::v1::types::{ChannelId, MessageSync, SfuId, UserId};
use lamprey_backend_core::Error;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, error, info, warn};

pub struct SfuHandleInner {
    pub id: SfuId,
    pub stats: RwLock<SfuStats>,
    pub tx: mpsc::UnboundedSender<SfuCommand>,
}

pub type SfuHandle = Arc<SfuHandleInner>;

impl SfuHandleInner {
    pub fn new(id: SfuId, tx: mpsc::UnboundedSender<SfuCommand>) -> Self {
        SfuHandleInner {
            id,
            stats: RwLock::new(SfuStats::default()),
            tx,
        }
    }

    pub fn id(&self) -> SfuId {
        self.id
    }

    pub fn send(&self, message: SfuCommand) {
        let _ = self.tx.send(message);
    }

    pub async fn update_stats(&self, stats: SfuStats) {
        let mut w = self.stats.write().await;
        *w = stats;
    }

    // TODO: send() -> tell(), add request()

    pub fn has_capacity(&self) -> bool {
        // TODO: use stats to calculate capacity
        // let stats = self.stats.blocking_read();
        // if stats.peer_count >= 500 {
        //     return false;
        // }

        // if stats.peer_count > 0 && stats.bandwidth_max > 0 {
        //     let bandwidth_per_peer = stats.bandwidth_usage / stats.peer_count;
        //     // check if adding another peer would exceed 80% of maximum bandwidth
        //     return (stats.bandwidth_usage + bandwidth_per_peer) < (stats.bandwidth_max * 8 / 10);
        // }

        true
    }
}

pub enum Allocation {
    /// join an existing sfu
    JoinExisting(SfuId),

    /// create a new worker on this sfu
    CascadeToNew {
        existing_sfu_id: SfuId,
        new_sfu_id: SfuId,
    },
}

pub enum RebalanceAction {
    /// migrate these users to this target sfu
    MigrateUsers {
        users: Vec<UserId>,
        target_sfu: SfuId,
    },

    /// shutdown the call worker on this sfu
    Shutdown { target_sfu: SfuId },
}

/// the approximate location of a user
///
/// used to allocate sfus close to the user
pub struct UserLocation {
    // /// the ip address information of the user
    // pub ip_info: IpInfo,
}

// impl UserLocation {
//     pub fn approx_latency_to(&self, other: ()) -> u64 {
//         todo!()
//     }
// }

impl ServiceVoice {
    pub async fn sfu_handle_connect(&self, mut socket: WebSocket) -> Result<SfuHandle> {
        let (tx, mut rx) = mpsc::unbounded_channel::<SfuCommand>();
        let sfu_id = SfuId::new();
        let handle = Arc::new(SfuHandleInner::new(sfu_id, tx));
        handle.send(SfuCommand::Init { sfu_id });

        self.sfus.insert(sfu_id, Arc::clone(&handle));

        let state = Arc::clone(&self.state);
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(cmd) = rx.recv() => {
                        if let Ok(json) = serde_json::to_string(&cmd) {
                            if socket.send(axum::extract::ws::Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    msg = socket.recv() => {
                        match msg {
                            Some(Ok(axum::extract::ws::Message::Text(text))) => {
                                // handle incoming event
                                if let Ok(event) = serde_json::from_str::<SfuEvent>(&text) {
                                    if let Err(e) = state.services().voice.sfu_handle_event(sfu_id, event).await {
                                        error!("Error handling SFU event: {:?}", e);
                                    }
                                }
                            }
                            _ => break,
                        }
                    }
                }
            }
        });

        Ok(handle)
    }

    pub(crate) async fn sfu_handle_event(&self, sfu_id: SfuId, event: SfuEvent) -> Result<()> {
        let srv = self.state.services();

        match event {
            SfuEvent::VoiceDispatch {
                user_id,
                channel_id,
                payload,
            } => {
                self.state.broadcast(MessageSync::VoiceDispatch {
                    user_id,
                    channel_id,
                    payload: *payload,
                })?;
            }
            SfuEvent::VoiceState {
                user_id,
                channel_id,
                update,
            } => {
                let old_state = srv.voice.state_get(channel_id, user_id);
                let new_channel_id = update.channel_id;
                srv.voice.state_update(user_id, update)?;
                let state = srv.voice.state_get(new_channel_id, user_id).unwrap();

                self.state.broadcast(MessageSync::VoiceState {
                    user_id,
                    state: Some(state.inner().clone()),
                    old_state: old_state.map(|h| h.inner().clone()),
                })?;
            }
            SfuEvent::PeerDisconnect {
                user_id,
                channel_id,
            } => {
                srv.voice.state_destroy(channel_id, user_id)?;

                self.state.broadcast(MessageSync::VoiceDispatch {
                    user_id,
                    channel_id,
                    payload: SignallingEvent::Disconnected,
                })?;
            }
            SfuEvent::Latency { target_sfu, rtt } => {
                let mut router = self.router.write().await;
                router.update_latency(sfu_id, target_sfu, rtt);
            }
            SfuEvent::Stats { stats } => {
                if let Some(sfu) = self.sfu_get(sfu_id) {
                    sfu.update_stats(stats).await;
                    debug!(%sfu_id, "SFU stats updated");
                }
            }
            SfuEvent::PeerCreated {
                user_id,
                channel_id,
            } => {
                info!(%user_id, %channel_id, "Peer created on SFU");
            }
            SfuEvent::CascadePrepared {
                sfu_id,
                token,
                addr,
            } => {
                info!(%addr, "Cascade prepared");
                // finish creating cascade (send command to sfu at sfu_id)
                if let Some(sfu) = self.sfu_get(sfu_id) {
                    sfu.send(SfuCommand::CreateCascade {
                        sfu_id,
                        token,
                        addr,
                    });
                } else {
                    error!(%sfu_id, "SFU not found for cascade preparation");
                }
            }
        }

        Ok(())
    }

    pub async fn sfu_alloc(&self, channel_id: ChannelId, user_id: UserId) -> Result<SfuHandle> {
        let sfu =
            match self.sfu_alloc_user(channel_id, user_id).await? {
                Allocation::JoinExisting(sfu_id) => self
                    .sfus
                    .get(&sfu_id)
                    .map(|s| Arc::clone(s.value()))
                    .ok_or_else(|| Error::ApiError(ApiError::from_code(ErrorCode::UnknownSfu)))?,
                Allocation::CascadeToNew {
                    existing_sfu_id,
                    new_sfu_id,
                } => {
                    let existing_sfu = self.sfus.get(&existing_sfu_id).ok_or_else(|| {
                        Error::ApiError(ApiError::from_code(ErrorCode::UnknownSfu))
                    })?;
                    let new_sfu = self.sfus.get(&new_sfu_id).ok_or_else(|| {
                        Error::ApiError(ApiError::from_code(ErrorCode::UnknownSfu))
                    })?;

                    existing_sfu.send(SfuCommand::PrepareCascade { sfu_id: new_sfu_id });
                    Arc::clone(&new_sfu)
                }
            };

        // ensure this sfu is registered in the call's active sfus set
        if let Some(call) = self.calls.get(&channel_id) {
            call.sfus.insert(sfu.id());
        }

        Ok(sfu)
    }

    // TODO: explicitly/manually shut down a sfu
    // pub fn sfu_destroy(&self, sfu_id: SfuId) -> Result<()> {
    //     let Some((_, sfu)) = self.sfus.remove(&sfu_id) else {
    //         return Ok(());
    //     };

    //     // TODO: send shutdown
    //     // sfu.send(SfuCommand::Shutdown);

    //     // NOTE: mabe use this?
    //     // sfu.send(SfuCommand::MigratePeers { peers: (), target_sfu: () });

    //     for state in self.state_list_by_sfu(sfu_id) {
    //         let _ = self.state.broadcast(MessageSync::VoiceDispatch {
    //             user_id: todo!(),
    //             channel_id: todo!(),
    //             payload: SignallingEvent::Migrate {
    //                 new_sfu_id: todo!(),
    //             },
    //         });
    //     }

    //     Ok(())
    // }

    pub fn sfu_get(&self, sfu_id: SfuId) -> Option<SfuHandle> {
        self.sfus.get(&sfu_id).map(|s| s.value().clone())
    }

    pub fn sfu_by_channel(&self, channel_id: ChannelId) -> Option<SfuHandle> {
        self.state_list_by_channel(channel_id)
            .first()
            .and_then(|handle| self.sfu_get(handle.sfu_id()))
    }

    pub fn sfu_broadcast(&self, command: SfuCommand) {
        for entry in self.sfus.iter() {
            entry.value().send(command.clone());
        }
    }

    /// find the closest sfu to this context
    fn sfu_find_closest(&self, loc: &UserLocation) -> Result<SfuHandle> {
        self.sfus
            .iter()
            .next()
            .filter(|s| s.has_capacity())
            .map(|s| Arc::clone(&*s))
            .ok_or(Error::BadStatic("no available sfus"))
    }

    /// figure out where to connect a user to
    pub async fn sfu_alloc_user(
        &self,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Result<Allocation> {
        let router = self.router.read().await;
        let acceptable_latency_ms = router.config.maximum_latency;

        // 1. check if the channel is already active on a node close to the user
        let mut best_existing = None;
        let mut min_latency = u32::MAX;

        let Some(call) = self.calls.get(&channel_id) else {
            return Err(Error::ApiError(ApiError::from_code(ErrorCode::UnknownCall)));
        };

        for sfu_id in call.sfus.iter() {
            let sfu_id = *sfu_id;
            if let Some(sfu) = self.sfus.get(&sfu_id) {
                if sfu.has_capacity() {
                    // let latency = graph.get_latency(user_region, node.region);
                    // let latency = self.router.latencies.get((a, b));
                    // TODO: more accurate latency estimation
                    let estimated_latency = 0;
                    if estimated_latency < min_latency {
                        min_latency = estimated_latency;
                        best_existing = Some(sfu_id);
                    }
                } else {
                    // TODO: handle full sfu case (maybe trigger a rebalance/)
                }
            } else {
                warn!(%sfu_id, "couldn't find sfu in use")
            }
        }

        // if so, join the existing sfu
        if let Some(sfu_id) = &best_existing {
            if min_latency <= acceptable_latency_ms {
                return Ok(Allocation::JoinExisting(*sfu_id));
            }
        }

        // 2. otherwise, the existing nodes are too far away or full
        // cascade by allocating a new node closer to the user
        let loc = UserLocation {}; // TODO: populate
        if let Ok(new_sfu) = self.sfu_find_closest(&loc) {
            let new_sfu_id = new_sfu.id();

            if let Some(existing_sfu_id) = best_existing {
                return Ok(Allocation::CascadeToNew {
                    existing_sfu_id,
                    new_sfu_id,
                });
            } else {
                // if there is no existing sfu for this call (ie. first user),
                // connect them directly to this new sfu
                return Ok(Allocation::JoinExisting(new_sfu_id));
            }
        }

        // 3. if no closer nodes exist, dump them on the best existing one for now
        if let Some(sfu_id) = &best_existing {
            return Ok(Allocation::JoinExisting(*sfu_id));
        }

        Err(Error::BadStatic("no available capacity anywhere!"))
    }

    /// recalculate the topology of a channel
    pub async fn sfu_rebalance(&self, channel_id: ChannelId) -> Vec<RebalanceAction> {
        // TODO: implement this

        let mut actions = Vec::new();
        let router = self.router.read().await;
        let merge_threshold = router.config.merge_threshold;

        // 0. partition voice states by sfu
        let mut voice_states_by_sfu: HashMap<SfuId, Vec<VoiceStateHandle>> = HashMap::new();
        for s in self.state_list_by_channel(channel_id) {
            voice_states_by_sfu
                .entry(s.sfu_id)
                .or_default()
                .push(Arc::clone(&s));
        }

        // let mut sfus_to_remove = HashSet::new();

        // 1. evaluate merging (cleanup under-utilized shards)
        for (&sfu_id, local_users) in &voice_states_by_sfu {
            // if local_users.len() <= merge_threshold && channel.active_nodes.len() > 1 {
            //     // find the biggest "center of gravity" node to absorb these users
            //     if let Some((&absorb_node, _)) = voice_states_by_sfu
            //         .iter()
            //         .filter(|(&id, _)| id != sfu_id && !sfus_to_remove.contains(&id))
            //         .max_by_key(|(_, users)| users.len())
            //     {
            //         let user_ids: Vec<UserId> = local_users.iter().map(|u| u.user_id).collect();
            //         actions.push(RebalanceAction::MigrateUsers {
            //             users: user_ids,
            //             target_sfu: absorb_node,
            //         });
            //         actions.push(RebalanceAction::Shutdown(sfu_id));
            //         sfus_to_remove.insert(sfu_id);
            //     }
            // }
        }

        // 2. evaluate splitting / migration (fix bad placements)
        for (&sfu_id, local_users) in &voice_states_by_sfu {
            // if sfus_to_remove.contains(&sfu_id) {
            //     continue;
            // }

            // NOTE: this pseudocode won't work for me, as regions don't exist in lamprey. i'll need to maybe create something like VoiceRouter but fuzzier for user locations.
            // // Group local users by their physical geographical region
            // let mut users_by_region: HashMap<RegionId, Vec<UserId>> = HashMap::new();
            // for user in local_users {
            //     users_by_region
            //         .entry(user.region)
            //         .or_default()
            //         .push(user.user_id);
            // }

            // let node_region = graph.nodes[&node_id].region;

            // for (user_region, users_in_region) in users_by_region {
            //     let latency = graph.get_latency(node_region, user_region);

            //     // If there's a large cluster of users experiencing high latency
            //     if latency > 100 && users_in_region.len() > 5 {
            //         // Find a better node closer to them
            //         if let Some(better_node) = graph.find_closest_available_node(user_region) {
            //             let new_latency =
            //                 graph.get_latency(user_region, graph.nodes[&better_node].region);

            //             // If the new node is significantly better, migrate them!
            //             if new_latency < latency - 40 {
            //                 actions.push(RebalanceAction::MigrateUsers {
            //                     users: users_in_region,
            //                     target_sfu: better_node,
            //                 });
            //             }
            //         }
            //     }
            // }
        }

        actions
    }
}
