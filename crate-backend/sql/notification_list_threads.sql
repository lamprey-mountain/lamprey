with last_id as (
    select channel_id, max(version_id) as last_version_id
    from message
    where deleted_at is null
    group by channel_id
),
message_count as (
    select channel_id, count(*) as count
    from message
    where is_latest
    group by channel_id
),
member_count as (
    select channel_id, count(*) as count
    from thread_member
    where membership = 'Join'
    group by channel_id
),
permission_overwrites as (
    select target_id, json_agg(jsonb_build_object('id', actor_id, 'type', type, 'allow', allow, 'deny', deny)) as overwrites
    from permission_overwrite
    group by target_id
),
unread_channels as (
    select distinct channel_id from inbox
    where user_id = $1
    and ($2 or read_at is null)
    and (array_length($3::uuid[], 1) is null or room_id = any($3))
    and (array_length($4::uuid[], 1) is null or channel_id = any($4))
)
select
    channel.id,
    channel.type as "ty: DbChannelType",
    channel.room_id,
    channel.creator_id,
    channel.name,
    channel.version_id,
    channel.description,
    channel.nsfw,
    channel.locked,
    channel.archived_at,
    channel.deleted_at,
    channel.parent_id,
    channel.position,
    channel.bitrate,
    channel.user_limit,
    channel.owner_id,
    channel.icon,
    coalesce(message_count.count, 0) as "message_count!",
    coalesce(member_count.count, 0) as "member_count!",
    last_id.last_version_id as "last_version_id",
    coalesce(permission_overwrites.overwrites, '[]') as "permission_overwrites!"
from channel
join unread_channels on channel.id = unread_channels.channel_id
left join message_count on message_count.channel_id = channel.id
left join member_count on member_count.channel_id = channel.id
left join last_id on last_id.channel_id = channel.id
left join permission_overwrites on permission_overwrites.target_id = channel.id
where channel.id > $5 and channel.id < $6
order by (CASE WHEN $7 = 'f' THEN channel.id END), channel.id DESC
limit $8
