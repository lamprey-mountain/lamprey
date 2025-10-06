with last_id as (
    select thread_id, max(version_id) as last_version_id
    from message
    where deleted_at is null
    group by thread_id
),
message_count as (
    select thread_id, count(*) as count
    from message
    where is_latest
    group by thread_id
),
member_count as (
    select thread_id, count(*) as count
    from thread_member
    where membership = 'Join'
    group by thread_id
),
permission_overwrites as (
    select target_id, json_agg(jsonb_build_object('id', actor_id, 'type', type, 'allow', allow, 'deny', deny)) as overwrites
    from permission_overwrite
    group by target_id
)
select
    thread.id,
    thread.type as "ty: DbThreadType",
    thread.room_id,
    thread.creator_id,
    thread.name,
    thread.version_id,
    thread.description,
    thread.nsfw,
    thread.locked,
    thread.archived_at,
    thread.deleted_at,
    thread.parent_id,
    thread.position,
    thread.bitrate,
    thread.user_limit,
    coalesce(message_count.count, 0) as "message_count!",
    coalesce(member_count.count, 0) as "member_count!",
    last_version_id as "last_version_id",
    coalesce(permission_overwrites.overwrites, '[]') as "permission_overwrites!"
from thread
left join message_count on message_count.thread_id = thread.id
left join member_count on member_count.thread_id = thread.id
left join last_id on last_id.thread_id = thread.id
left join permission_overwrites on permission_overwrites.target_id = thread.id
where room_id = $1 AND thread.id > $2 AND thread.id < $3 and thread.deleted_at is null and thread.archived_at is not null and ($6::uuid is null or parent_id = $6)
order by (CASE WHEN $4 = 'f' THEN thread.id END), thread.id DESC LIMIT $5
