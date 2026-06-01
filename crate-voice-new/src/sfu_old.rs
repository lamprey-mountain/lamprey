// TODO: allow explicitly subscribing/unsubscribing to audio streams
impl SfuShard {
    fn handle_command(&mut self, cmd: ShardCommand) {
        match cmd {
            ShardCommand::Signalling { user_id, inner } => {
                if let Some(&pid) = self.user_map.get(&user_id) {
                    let mut tracks_to_have = Vec::new();

                    // update known tracks
                    if let SignallingCommand::Offer { tracks, .. } = &inner {
                        let peer = &mut self.peers[pid];
                        for track in tracks {
                            let track_id = self.tracks.insert(TrackSfu {
                                publisher: pid,
                                subscribers: smallvec::SmallVec::new(),
                                kind: track.kind.into(),
                                key: track.key.clone(),
                                state: TrackState::Negotiating(track.mid.into()),
                            });
                            peer.inbound.insert(track.mid.into(), track_id);

                            tracks_to_have.push((
                                track_id,
                                TrackMetadata {
                                    kind: track.kind.into(),
                                    key: track.key.clone(),
                                    mid: track.mid.into(),
                                    layers: vec![],
                                },
                            ));
                        }
                    }

                    if !tracks_to_have.is_empty() {
                        let peer_keys: Vec<PeerId> = self.peers.keys().collect();
                        for target_pid in peer_keys {
                            if target_pid == pid {
                                continue;
                            }
                            let target_peer = self.peers.get_mut(target_pid).unwrap();
                            let mut have_tracks = Vec::new();
                            for (track_id, track_meta) in &tracks_to_have {
                                target_peer.pending_tracks.push(*track_id);
                                have_tracks.push(track_meta.clone());
                            }

                            let _ = self.backend.send(SfuEvent::VoiceDispatch {
                                user_id: target_peer.user_id,
                                channel_id: self.channel_id,
                                payload: Box::new(SignallingEvent::Have {
                                    user_id, // publisher's user_id
                                    tracks: have_tracks,
                                }),
                            });
                        }
                    }

                } else {
                    // TODO: warn
                }
            }
        }
    }
}
