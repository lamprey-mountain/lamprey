with last_id as (
    select channel_id, max(version_id) as last_version_id
    from message
    where deleted_at is null
    group by channel_id
)
select
    channel.id,
    channel.type as "ty: DbChannelType",
    u.message_id as "last_read_id?",
    coalesce(u.version_id < last_version_id, true) as "is_unread!",
    coalesce(u.mention_count, 0) as "mention_count!"
from channel
left join last_id on last_id.channel_id = channel.id
left join unread u on u.channel_id = channel.id and u.user_id = $2
where channel.id = $1
