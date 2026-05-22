use crate::services::voice::{ServiceVoice, SfuCommand, SfuStats};
use crate::Result;
use axum::extract::ws::WebSocket;
use common::v1::types::voice::messages::{SfuEvent, SignallingEvent};
use common::v1::types::{ChannelId, MessageSync, PeerId, SfuId, UserId};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

pub struct SfuHandleInner {
    pub id: SfuId,
    pub stats: SfuStats,
    pub tx: mpsc::UnboundedSender<SfuCommand>,
}

pub type SfuHandle = Arc<SfuHandleInner>;

impl SfuHandleInner {
    pub fn new(id: SfuId, tx: mpsc::UnboundedSender<SfuCommand>) -> Self {
        SfuHandleInner {
            id,
            stats: SfuStats::default(),
            tx,
        }
    }

    pub fn id(&self) -> SfuId {
        self.id
    }

    pub fn send(&self, message: SfuCommand) {
        let _ = self.tx.send(message);
    }

    // TODO: send() -> tell(), add request()
}

pub enum Allocation {
    /// join an existing sfu
    JoinExisting(SfuId),

    /// create a new worker on this sfu
    CascadeToNew(SfuId),
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

impl ServiceVoice {
    pub async fn sfu_handle_connect(
        &self,
        sfu_id: SfuId,
        mut socket: WebSocket,
    ) -> Result<SfuHandle> {
        let (tx, mut rx) = mpsc::unbounded_channel::<SfuCommand>();

        let handle = Arc::new(SfuHandleInner::new(sfu_id, tx));

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
                                    if let Err(e) = state.services().voice.sfu_handle_event(event).await {
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

    pub(crate) async fn sfu_handle_event(&self, event: SfuEvent) -> Result<()> {
        let srv = self.state.services();

        match event {
            SfuEvent::VoiceDispatch { peer_id, payload } => {
                let user_id = self.state_get(peer_id).unwrap().inner.user_id;
                self.state.broadcast(MessageSync::VoiceDispatch {
                    user_id,
                    peer_id,
                    payload: *payload,
                })?;
            }
            SfuEvent::VoiceState { peer_id, state } => {
                let user_id = self.state_get(peer_id).unwrap().inner.user_id;
                let old_state = srv.voice.state_get(peer_id);
                if let Some(state) = &state {
                    srv.voice.state_replace(state.clone())?;
                } else {
                    srv.voice.state_destroy(peer_id)?;
                }

                self.state.broadcast(MessageSync::VoiceState {
                    user_id,
                    peer_id,
                    state,
                    old_state: old_state.map(|h| h.inner().clone()),
                })?;
            }
            SfuEvent::Ready { sfu_id } => {
                if let Some(sfu) = self.sfu_get(sfu_id) {
                    info!(%sfu_id, "SFU is ready");
                }
            }
            SfuEvent::Latency { target_sfu, rtt } => {
                if let Some(sfu) = self.sfu_get(target_sfu) {
                    let mut sfu_inner = sfu;
                    // Note: SfuStats is in an Arc, so we might need interior mutability if it were intended to be updated
                    // Assuming for now we just log it or that stats field is a placeholder
                    debug!(%target_sfu, %rtt, "SFU latency update");
                }
            }
            SfuEvent::Stats { stats } => {
                // Update stats if we had a way to mutate SfuHandleInner
                debug!(?stats, "SFU stats updated");
            }
            SfuEvent::PeerCreated { peer_id } => {
                info!(%peer_id, "Peer created on SFU");
            }
            SfuEvent::CascadePrepared { token, addr } => {
                info!(%addr, "Cascade prepared");
            }
        }

        Ok(())
    }

    pub fn sfu_alloc(&self, channel_id: ChannelId, peer_id: PeerId) -> Result<SfuHandle> {
        match self.sfu_alloc_user(channel_id, peer_id)? {
            Allocation::JoinExisting(id) => todo!(),
            Allocation::CascadeToNew(id) => todo!(),
        }
    }

    pub fn sfu_destroy(&self, sfu_id: SfuId) -> Result<()> {
        let Some((_, sfu)) = self.sfus.remove(&sfu_id) else {
            return Ok(());
        };

        // TODO: send shutdown
        // sfu.send(SfuCommand::Shutdown);

        // NOTE: mabe use this?
        // sfu.send(SfuCommand::MigratePeers { peers: (), target_sfu: () });

        for state in self.state_list_by_sfu(sfu_id) {
            let _ = self.state.broadcast(MessageSync::VoiceDispatch {
                user_id: todo!(),
                peer_id: todo!(),
                payload: SignallingEvent::Migrate {
                    new_sfu_id: todo!(),
                },
            });
        }

        Ok(())
    }

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

    // fn sfu_find_closest(&self, _addr: IpAddr, _limit: usize) -> Vec<SfuHandle> {
    //     todo!()
    // }

    /// figure out where to connect a user to
    pub fn sfu_alloc_user(&self, channel_id: ChannelId, peer_id: PeerId) -> Result<Allocation> {
        todo!()
        // let acceptable_latency_ms = 80;

        // // 1. check if the channel is already active on a node close to the user
        // let mut best_existing = None;
        // let mut min_latency = u32::MAX;

        // for &sfu_id in &self.calls.get(&channel_id).sfus { }

        // for &node_id in &channel.active_nodes {
        //     if let Some(node) = graph.nodes.get(&node_id) {
        //         if node.current_users < node.max_capacity {
        //             let latency = graph.get_latency(user_region, node.region);
        //             if latency < min_latency {
        //                 min_latency = latency;
        //                 best_existing = Some(node_id);
        //             }
        //         }
        //     }
        // }

        // if let Some(node_id) = best_existing {
        //     if min_latency <= acceptable_latency_ms {
        //         return Allocation::JoinExisting(node_id);
        //     }
        // }

        // // 2. otherwise, the existing nodes are too far away or full
        // // cascade by allocating a new node closer to the user
        // if let Some(new_node_id) = graph.find_closest_available_node(user_region) {
        //     return Allocation::CascadeToNew(new_node_id);
        // }

        // // 3. if no closer nodes exist, dump them on the best existing one for now
        // if let Some(node_id) = best_existing {
        //     return Allocation::JoinExisting(node_id);
        // }

        // Error("No available capacity worldwide".to_string())
    }

    /// recalculate the topology of a channel
    pub fn sfu_rebalance(&self, channel_id: ChannelId) -> Vec<RebalanceAction> {
        todo!()
        // let mut actions = Vec::new();
        // let merge_threshold = self.router.config.merge_threshold;

        // let mut voice_states_by_sfu: HashMap<SfuId, Vec<VoiceStateHandle>> = HashMap::new();
        // for s in self.state_list_by_channel(channel_id) {
        //     voice_states_by_sfu
        //         .entry(s.current_node)
        //         .or_default()
        //         .push(s);
        // }

        // let mut sfus_to_remove = HashSet::new();

        // // 1. evaluate merging (cleanup under-utilized shards)
        // for (&sfu_id, local_users) in &voice_states_by_sfu {
        //     if local_users.len() <= merge_threshold && channel.active_nodes.len() > 1 {
        //         // find the biggest "center of gravity" node to absorb these users
        //         if let Some((&absorb_node, _)) = voice_states_by_sfu
        //             .iter()
        //             .filter(|(&id, _)| id != sfu_id && !sfus_to_remove.contains(&id))
        //             .max_by_key(|(_, users)| users.len())
        //         {
        //             let user_ids: Vec<UserId> = local_users.iter().map(|u| u.user_id).collect();
        //             actions.push(RebalanceAction::MigrateUsers {
        //                 users: user_ids,
        //                 target_sfu: absorb_node,
        //             });
        //             actions.push(RebalanceAction::Shutdown(sfu_id));
        //             sfus_to_remove.insert(sfu_id);
        //         }
        //     }
        // }

        // // 2. evaluate splitting / migration (fix bad placements)
        // for (&node_id, local_users) in &voice_states_by_sfu {
        //     if sfus_to_remove.contains(&node_id) {
        //         continue;
        //     }

        //     // NOTE: this pseudocode won't work for me, as regions don't exist in lamprey. i'll need to maybe create something like VoiceRouter but fuzzier for user locations.
        //     // // Group local users by their physical geographical region
        //     // let mut users_by_region: HashMap<RegionId, Vec<UserId>> = HashMap::new();
        //     // for user in local_users {
        //     //     users_by_region
        //     //         .entry(user.region)
        //     //         .or_default()
        //     //         .push(user.user_id);
        //     // }

        //     // let node_region = graph.nodes[&node_id].region;

        //     // for (user_region, users_in_region) in users_by_region {
        //     //     let latency = graph.get_latency(node_region, user_region);

        //     //     // If there's a large cluster of users experiencing high latency
        //     //     if latency > 100 && users_in_region.len() > 5 {
        //     //         // Find a better node closer to them
        //     //         if let Some(better_node) = graph.find_closest_available_node(user_region) {
        //     //             let new_latency =
        //     //                 graph.get_latency(user_region, graph.nodes[&better_node].region);

        //     //             // If the new node is significantly better, migrate them!
        //     //             if new_latency < latency - 40 {
        //     //                 actions.push(RebalanceAction::MigrateUsers {
        //     //                     users: users_in_region,
        //     //                     target_sfu: better_node,
        //     //                 });
        //     //             }
        //     //         }
        //     //     }
        //     // }
        // }

        // actions
    }
}
