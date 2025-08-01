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
permission_overwrites as (
    select target_id, json_agg(jsonb_build_object('id', actor_id, 'type', type, 'allow', allow, 'deny', deny)) as overwrites
    from permission_overwrite
    where target_id = $1
    group by target_id
)
select
    thread.id,
    thread.type as "ty: DbThreadType",
    thread.room_id,
    thread.creator_id,
    thread.version_id,
    thread.name,
    thread.description,
    coalesce(count, 0) as "message_count!",
    last_version_id as "last_version_id",
    coalesce(permission_overwrites.overwrites, '[]') as "permission_overwrites!"
from thread
left join message_count on message_count.thread_id = thread.id
left join last_id on last_id.thread_id = thread.id
left join permission_overwrites on permission_overwrites.target_id = thread.id
where thread.id = $1
