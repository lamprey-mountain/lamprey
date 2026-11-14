select
    channel.id,
    channel.type as "ty: DbChannelType",
    u.message_id as "last_read_id?",
    coalesce(u.message_id < channel.last_message_id, true) as "is_unread!",
    coalesce(u.mention_count, 0) as "mention_count!",
    (
        exists (select 1 from dm where dm.channel_id = channel.id)
        and not exists (
            select 1 from dm_visible v
            where v.channel_id = channel.id and v.user_id = $2
        )
    ) as "dm_hidden!"
from channel
left join unread u on u.channel_id = channel.id and u.user_id = $2
where channel.id = $1
