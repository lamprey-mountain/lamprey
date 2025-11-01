with
    channel_viewer as (
        select channel.id from channel
        where channel.id = any($11)
        union
        select channel.id from channel
        where channel.parent_id = any($12)
        union
        select channel.id from channel
        join thread_member on channel.id = thread_member.channel_id
        where channel.room_id is null and thread_member.user_id = $1 and thread_member.membership = 'Join'
    ),
    last_id as (
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
    channel.invitable,
    channel.auto_archive_duration,
    channel.default_auto_archive_duration,
    channel.last_activity_at,
    coalesce(message_count.count, 0) as "message_count!",
    coalesce(member_count.count, 0) as "member_count!",
    last_version_id as "last_version_id",
    coalesce(permission_overwrites.overwrites, '[]') as "permission_overwrites!",
    (SELECT json_agg(tag_id) FROM channel_tag WHERE channel_id = channel.id) as tags,
    (SELECT json_agg(tag.*) FROM tag WHERE channel_id = channel.id) as tags_available
from channel
join channel_viewer on channel.id = channel_viewer.id
left join message_count on message_count.channel_id = channel.id
left join member_count on member_count.channel_id = channel.id
left join last_id on last_id.channel_id = channel.id
left join permission_overwrites on permission_overwrites.target_id = channel.id
where ($9::boolean is null or (channel.archived_at is not null) = $9)
  and ($10::boolean is null or (channel.deleted_at is not null) = $10)
  and channel.id > $2 AND channel.id < $3
  and (
    $6::text is null or $6 = '' or
    channel.name @@ websearch_to_tsquery('english', $6) or
    coalesce(channel.description, '') @@ websearch_to_tsquery('english', $6)
  )
  and (cardinality($7::uuid[]) = 0 or channel.room_id = any($7))
  and (cardinality($8::uuid[]) = 0 or channel.parent_id = any($8))
order by (CASE WHEN $4 = 'f' THEN channel.id END), channel.id DESC LIMIT $5
