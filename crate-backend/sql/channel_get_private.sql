select
    channel.id,
    channel.type as "ty: DbChannelType",
    u.message_id as "last_read_id?",
    coalesce(u.version_id < channel.last_message_id, true) as "is_unread!",
    coalesce(u.mention_count, 0) as "mention_count!"
from channel
left join unread u on u.channel_id = channel.id and u.user_id = $2
where channel.id = $1
