use crate::{prelude::*, services::notifications::ServiceNotifications};
use common::v1::types::notifications::{
    NotificationPagination, NotificationQuery, NotificationType,
};
use common::v1::types::{NotificationId, PaginationQuery, UserId};
use common::v2::types::{ChannelId, MessageId, RoomId};
use futures::stream::FuturesUnordered;
use futures::{FutureExt, TryFutureExt, TryStreamExt, try_join};
use std::collections::{HashMap, HashSet, hash_map};

impl ServiceNotifications {
    // PERF: run as many queries in parallel as possible
    pub async fn inbox_query(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<NotificationId>,
        query: NotificationQuery,
    ) -> Result<NotificationPagination> {
        let srv = self.globals.services();

        let mut txn = self.globals.begin_read().await?;
        let raw = txn.notification_list(user_id, pagination, query).await?;

        let notifications = raw.items;

        let mut room_ids = HashSet::new();
        let mut channel_ids = HashSet::new();
        let mut message_ids: HashMap<ChannelId, HashSet<MessageId>> = HashMap::new();
        let mut user_ids = HashSet::new();
        let mut user_ids_by_room: HashMap<RoomId, HashSet<UserId>> = HashMap::new();
        let mut user_ids_by_channel: HashMap<ChannelId, HashSet<UserId>> = HashMap::new();

        for notif in &notifications {
            // TODO: move this match into common?
            let user_id = match &notif.ty {
                NotificationType::Message { user_id, .. }
                | NotificationType::Reaction { user_id, .. }
                | NotificationType::Thread { user_id, .. }
                | NotificationType::FriendRequestSent { user_id }
                | NotificationType::FriendRequestReceived { user_id }
                | NotificationType::FriendRequestAccepted { user_id } => *user_id,
            };

            if let Some(room_id) = notif.room_id() {
                room_ids.insert(room_id);
                user_ids_by_room.entry(room_id).or_default().insert(user_id);
            }

            if let Some(channel_id) = notif.channel_id() {
                channel_ids.insert(channel_id);

                user_ids_by_channel
                    .entry(channel_id)
                    .or_default()
                    .insert(user_id);

                if let Some(message_id) = notif.message_id() {
                    message_ids
                        .entry(channel_id)
                        .or_default()
                        .insert(message_id);
                }
            }

            user_ids.insert(*user_id);
        }

        // verify channel access before returning channel data
        let mut channels = Vec::new();
        if !channel_ids.is_empty() {
            let mut visibilities = HashMap::new();
            for channel_id in &channel_ids {
                if let hash_map::Entry::Vacant(entry) = visibilities.entry(*channel_id) {
                    let perm = srv.perms.for_channel3(Some(user_id), *channel_id).await?;
                    entry.insert(perm.visible);
                }
            }

            let channel_ids_vec: Vec<_> = channel_ids.iter().cloned().collect();
            let threads = srv
                .channels
                .get_many(&channel_ids_vec, Some(user_id))
                .await?;
            for thread in threads {
                if visibilities.get(&thread.id).copied().unwrap_or(false) {
                    channels.push(thread);
                }
            }
        }

        // fetch messages
        let mut message_futs = FuturesUnordered::new();
        for (channel_id, message_ids) in &message_ids {
            let message_ids_vec: Vec<_> = message_ids.iter().cloned().collect();
            let srv2 = self.globals.services();
            message_futs.push(
                async move {
                    srv2.messages
                        .get_many(*channel_id, Some(user_id), &message_ids_vec)
                        .map_ok(|msgs| (*channel_id, msgs))
                        .await
                }
                .boxed(),
            );
        }

        let mut messages = Vec::new();
        let mut reply_ids: HashMap<ChannelId, HashSet<MessageId>> = HashMap::new();
        while let Some(res) = message_futs.next().await {
            if let Ok((channel_id, msgs)) = res {
                let reply_ids_for_channel = reply_ids.entry(channel_id).or_default();
                for msg in &msgs {
                    user_ids.insert(*msg.author_id);
                    user_ids_by_channel
                        .entry(channel_id)
                        .or_default()
                        .insert(msg.author_id);
                    if let Some(room_id) = msg.room_id {
                        user_ids_by_room
                            .entry(room_id)
                            .or_default()
                            .insert(msg.author_id);
                    }
                }
                messages.extend(msgs);
            }
        }

        // fetch replies
        for (channel_id, msgs) in &message_ids {
            if let Some(a) = reply_ids.get_mut(channel_id) {
                for b in msgs {
                    a.remove(b);
                }
            }
        }

        let mut reply_futs = FuturesUnordered::new();
        for (channel_id, reply_ids) in &reply_ids {
            let message_ids_vec: Vec<_> = reply_ids.iter().cloned().collect();
            let srv2 = self.globals.services();
            reply_futs.push(
                async move {
                    srv2.messages
                        .get_many(*channel_id, Some(user_id), &message_ids_vec)
                        .map_ok(|msgs| (*channel_id, msgs))
                        .await
                }
                .boxed(),
            );
        }

        while let Some(res) = reply_futs.next().await {
            if let Ok((channel_id, msgs)) = res {
                for msg in &msgs {
                    user_ids.insert(*msg.author_id);
                    user_ids_by_channel
                        .entry(channel_id)
                        .or_default()
                        .insert(msg.author_id);
                    if let Some(room_id) = msg.room_id {
                        user_ids_by_room
                            .entry(room_id)
                            .or_default()
                            .insert(msg.author_id);
                    }
                }
                messages.extend(msgs);
            }
        }

        // only return archived threads and dms
        let threads: Vec<_> = channels
            .into_iter()
            .filter(|c| (c.is_thread() && c.is_archived()) || c.is_dm())
            .collect();

        // fetch user data for every referenced user
        let user_ids: Vec<UserId> = user_ids.into_iter().map(UserId::from).collect();
        let users = srv.users.get_many(&user_ids);

        let room_members = async {
            let mut room_member_futs = FuturesUnordered::new();
            for (room_id, user_ids) in &mut user_ids_by_room {
                // also fetch room member for the requesting user
                user_ids.insert(user_id);

                room_member_futs.push(async {
                    let handle = srv.rooms.load(*room_id);
                    let room = handle.ready(true).await?;
                    let mut members = vec![];

                    for uid in user_ids.iter() {
                        if let Some(m) = room.members.get(uid) {
                            members.push(m.member.clone())
                        }
                    }

                    Result::Ok(members)
                });
            }

            room_member_futs
                .try_fold(Vec::new(), |mut acc, members| async move {
                    acc.extend(members);
                    Ok(acc)
                })
                .await
        };

        let thread_members = async {
            let thread_member_futs = FuturesUnordered::new();
            for (thread_id, user_ids) in &mut user_ids_by_channel {
                // also fetch room member for the requesting user
                user_ids.insert(user_id);

                thread_member_futs.push(async {
                    let user_ids: Vec<_> = user_ids.iter().cloned().collect();
                    let mut txn = self.globals.begin_read().await?;
                    let members = txn.thread_member_get_many(*thread_id, &user_ids).await?;
                    Result::Ok(members)
                });
            }

            thread_member_futs
                .try_fold(Vec::new(), |mut acc, members| async move {
                    acc.extend(members);
                    Ok(acc)
                })
                .await
        };

        let (room_members, thread_members, users) = try_join!(room_members, thread_members, users)?;

        Ok(NotificationPagination {
            notifications,
            total: raw.total,
            has_more: raw.has_more,
            cursor: raw.cursor,
            messages,
            threads,
            room_members,
            thread_members,
            users,
        })
    }
}
