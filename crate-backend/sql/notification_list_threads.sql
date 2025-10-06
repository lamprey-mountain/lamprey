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
),
unread_threads as (
    select distinct thread_id from inbox
    where user_id = $1
    and ($2 or read_at is null)
    and (array_length($3::uuid[], 1) is null or room_id = any($3))
    and (array_length($4::uuid[], 1) is null or thread_id = any($4))
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
    thread.owner_id,
    coalesce(message_count.count, 0) as "message_count!",
    coalesce(member_count.count, 0) as "member_count!",
    last_id.last_version_id as "last_version_id",
    coalesce(permission_overwrites.overwrites, '[]') as "permission_overwrites!"
from thread
join unread_threads on thread.id = unread_threads.thread_id
left join message_count on message_count.thread_id = thread.id
left join member_count on member_count.thread_id = thread.id
left join last_id on last_id.thread_id = thread.id
left join permission_overwrites on permission_overwrites.target_id = thread.id
where thread.id > $5 and thread.id < $6
order by (CASE WHEN $7 = 'f' THEN thread.id END), thread.id DESC
limit $8
