with last_id as (
    select m.channel_id, max(mv.version_id) as last_version_id
    from message m
    join message_version mv on m.latest_version_id = mv.version_id
    where m.deleted_at is null
    group by m.channel_id
),
message_count as (
    select m.channel_id, count(*) as count
    from message m
    where m.deleted_at is null
    group by m.channel_id
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
)
select
    channel.id,
    channel.type as "ty: DbChannelType",
    channel.room_id,
    channel.creator_id,
    channel.name,
    channel.version_id,
    channel.description,
    channel.url,
    channel.nsfw,
    channel.locked,
    channel.locked_until,
    channel.locked_roles,
    channel.archived_at,
    channel.deleted_at,
    channel.parent_id,
    channel.position,
    channel.bitrate,
    channel.user_limit,
    channel.owner_id,
    channel.icon,
    channel.invitable,
    channel.auto_archive_duration,
    channel.default_auto_archive_duration,
    channel.slowmode_thread,
    channel.slowmode_message,
    channel.default_slowmode_message,
    channel.last_activity_at,
    coalesce(message_count.count, 0) as "message_count!",
    coalesce(member_count.count, 0) as "member_count!",
    last_id.last_version_id as "last_version_id",
    coalesce(permission_overwrites.overwrites, '[]') as "permission_overwrites!",
    (SELECT json_agg(tag_id) FROM channel_tag WHERE channel_id = channel.id) as tags,
    (SELECT json_agg(tag.*) FROM tag WHERE channel_id = channel.id) as tags_available
from channel
left join dm on dm.channel_id = channel.id
left join message_count on message_count.channel_id = channel.id
left join member_count on member_count.channel_id = channel.id
join last_id on last_id.channel_id = channel.id
left join permission_overwrites on permission_overwrites.target_id = channel.id
where (dm.user_a_id = $1 or dm.user_b_id = $1 or dm.user_a_id is null)
  and last_id.last_version_id > $2 and last_id.last_version_id < $3
  and (channel.type = 'Dm' or channel.type = 'Gdm')
order by (CASE WHEN $4 = 'f' THEN last_id.last_version_id END), last_id.last_version_id DESC
LIMIT $5
