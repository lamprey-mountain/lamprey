with last_id as (
    select m.channel_id, max(mv.version_id) as last_version_id
    from message m
    join message_version mv on m.latest_version_id = mv.version_id
    where m.deleted_at is null
    group by m.channel_id
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
