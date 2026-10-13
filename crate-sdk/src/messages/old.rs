// NOTE: maybe allow multiple in flight requests as long as they're deduplicated?
impl Messages {
    /// fetch a single message
    pub async fn fetch(&self, id: MessageId) -> Result<Arc<Message>> {
        {
            let read = self.inner.read().await;
            if let Some(m) = read.messages.get(&id) {
                return Ok(m.clone());
            }
        }

        let message = Arc::new(
            self.http
                .message_get(self.channel_id, id)
                .await
                .map_err(|e| anyhow::anyhow!("failed to fetch message: {}", e))?,
        );

        {
            let mut write = self.inner.write().await;
            write.insert_message(message.clone());
            write.insert_range(Range {
                start: id,
                end: id,
                stale: false,
            });
        }

        Ok(message)
    }

    /// fetch a range of messages before this message
    ///
    /// the returned `MessageRange` may contain more or less messages than `limit`
    pub async fn fetch_before(&self, id: MessageId, limit: u16) -> Result<MessageSlice> {
        {
            let read = self.inner.read().await;
            if let Some(range) = read.find_range(id) {
                let have: Vec<_> = read
                    .messages
                    .range(range.start..=id)
                    .take(limit as usize)
                    .map(|(_, m)| m.clone())
                    .collect();
                if have.len() == limit as usize || Some(range.start) == read.start {
                    let has_backwards = Some(range.start) != read.start;
                    return Ok(MessageSlice {
                        // NOTE: should be false if `id` is in live timeline
                        has_forward: true,
                        has_backwards,
                        stale: range.stale,
                        messages: have,
                    });
                }
            }
        }

        let res = self
            .http
            .message_list(
                self.channel_id,
                &PaginationQuery {
                    from: Some(id),
                    to: None,
                    dir: Some(PaginationDirection::B),
                    limit: Some(limit),
                },
            )
            .await?;
        let msgs: Vec<Arc<Message>> = res.items.into_iter().map(Arc::new).collect();
        let new_range = Range {
            start: msgs.first().map(|m| m.id).unwrap_or(id),
            end: id,
            stale: false,
        };

        {
            let mut write = self.inner.write().await;
            for m in msgs.iter().cloned() {
                write.insert_message(m);
            }
            write.insert_range(new_range);
        }

        Ok(MessageSlice {
            messages: msgs,
            stale: false,
            has_forward: true,
            has_backwards: res.has_more,
        })
    }

    pub async fn fetch_after(&self, id: MessageId, limit: u16) -> Result<MessageSlice> {
        {
            let read = self.inner.read().await;
            if let Some(range) = read.find_range(id) {
                // messages in range after id
                let have: Vec<_> = read
                    .messages
                    .range(id..=range.end)
                    .take(limit as usize)
                    .map(|(_, m)| m.clone())
                    .collect();
                if have.len() == limit as usize || Some(range.end) == read.end {
                    return Ok(MessageSlice {
                        messages: have,
                        has_forward: todo!(),
                        has_backwards: todo!(),
                        stale: todo!(),
                    });
                }
            }
        }

        let res = self
            .http
            .message_list(
                self.channel_id,
                &PaginationQuery {
                    from: Some(id),
                    to: None,
                    dir: Some(PaginationDirection::F),
                    limit: Some(limit),
                },
            )
            .await?;
        let msgs: Vec<Arc<Message>> = res.items.into_iter().map(Arc::new).collect();
        let new_range = Range {
            start: id,
            end: msgs.last().map(|m| m.id).unwrap_or(id),
            stale: false,
        };

        let (has_backwards, has_forward) = {
            let mut write = self.inner.write().await;
            for m in msgs.iter().cloned() {
                write.insert_message(m);
            }
            write.insert_range(new_range);
            (
                Some(new_range.start) != write.start,
                Some(new_range.end) != write.end,
            )
        };

        Ok(MessageSlice {
            messages: msgs,
            has_forward,
            has_backwards,
            stale: false,
        })
    }

    pub async fn fetch_context(&self, id: MessageId, limit: u16) -> Result<MessageSlice> {
        // TODO: return existing messge slice if it exists

        let res = self
            .http
            .message_context(
                self.channel_id,
                id,
                ContextQuery {
                    to_start: None,
                    to_end: None,
                    limit: Some(limit),
                },
            )
            .await?;

        let msgs: Vec<Arc<Message>> = res.items.into_iter().map(Arc::new).collect();

        if !msgs.is_empty() {
            let new_range = Range {
                start: msgs.first().unwrap().id,
                end: msgs.last().unwrap().id,
                stale: false,
            };

            let mut write = self.inner.write().await;
            for m in msgs.iter().cloned() {
                write.insert_message(m);
            }
            write.insert_range(new_range);
        }

        Ok(MessageSlice {
            messages: msgs,
            has_forward: res.has_after,
            has_backwards: res.has_before,
            stale: false,
        })
    }

    /// subscribe to the live timeline
    ///
    /// returns a stream of message slices containing the last `limit` messages in this channel
    pub fn subscribe_live(&self, limit: u16) -> impl Stream<Item = MessageSlice> {
        futures_util::stream::empty().boxed(
    }

    // // ???
    // // maybe have subscribe_foo variants of fetch_foo?
    // pub fn subscribe_range(&self, limit: u16) -> impl Stream<Item = MessageSlice<'_>> {
    //     futures_util::stream::empty().boxed()
    // }
}

impl MessagesInner {
    fn insert_message(&mut self, message: Arc<Message>) {
        self.messages.insert(message.id, message);
    }

    /// insert a new range, merging with overlapping/adjacent ranges
    fn insert_range(&mut self, new_range: Range) {
        let idx = self.ranges.partition_point(|r| r.end < new_range.start);

        let mut merged = new_range;
        let mut remove_start = idx;
        let mut remove_end = idx;

        // merge backwards
        while remove_start > 0 && self.ranges[remove_start - 1].touches(merged) {
            merged = merged.merge(self.ranges[remove_start - 1]);
            remove_start -= 1;
        }

        // merge forwards
        while remove_end < self.ranges.len() && self.ranges[remove_end].touches(merged) {
            merged = merged.merge(self.ranges[remove_end]);
            remove_end += 1;
        }

        self.ranges.splice(remove_start..remove_end, [merged]);
    }

    /// get which range this message id is in
    fn find_range(&self, id: MessageId) -> Option<Range> {
        let idx = self.ranges.partition_point(|r| r.end < id);
        self.ranges.get(idx).filter(|r| r.contains(id)).copied()
    }
}

// TODO: remove this, impl sync event handling logic in cache
async fn spawn_sync_task(syncer: SyncerHandle, inner: Arc<RwLock<MessagesInner>>) {
    let mut s = syncer.subscribe();
    while let Some(e) = s.next().await {
        match &*e {
            SyncerEvent::Message(m) => match &m.payload {
                MessagePayload::Sync { data, nonce, .. } => match &**data {
                    MessageSync::MessageCreate { message } => {
                        // NOTE: if nonce == idempotency_key we sent this message
                        // let a = inner.write().await;
                        // a.insert_message(message);
                        // a.insert_range(new_range);
                        // Range::single(message.id)
                        todo!()
                    }
                    MessageSync::MessageUpdate { message } => todo!(),
                    MessageSync::MessageDelete {
                        channel_id,
                        message_id,
                        room_id,
                    } => todo!(),
                    MessageSync::MessageDeleteBulk {
                        channel_id,
                        message_ids,
                        room_id,
                    } => todo!(),
                    MessageSync::MessageVersionDelete {
                        channel_id,
                        message_id,
                        version_id,
                        room_id,
                    } => todo!(),
                    MessageSync::MessageRemove {
                        channel_id,
                        message_ids,
                        room_id,
                    } => todo!(),
                    MessageSync::MessageRestore {
                        channel_id,
                        message_ids,
                        room_id,
                    } => todo!(),
                    _ => {}
                },
                _ => {}
            },
            SyncerEvent::StateChanged => {
                todo!("mark as stale if disconnected and failed to resume")
            }
            _ => {}
        }
    }
}

