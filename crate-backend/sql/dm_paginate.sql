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
    thread.owner_id,
    coalesce(message_count.count, 0) as "message_count!",
    coalesce(member_count.count, 0) as "member_count!",
    last_id.last_version_id as "last_version_id",
    coalesce(permission_overwrites.overwrites, '[]') as "permission_overwrites!"
from thread
left join dm on dm.thread_id = thread.id
left join message_count on message_count.thread_id = thread.id
left join member_count on member_count.thread_id = thread.id
join last_id on last_id.thread_id = thread.id
left join permission_overwrites on permission_overwrites.target_id = thread.id
where (dm.user_a_id = $1 or dm.user_b_id = $1 or dm.user_a_id is null)
  and last_id.last_version_id > $2 and last_id.last_version_id < $3
  and (thread.type = 'Dm' or thread.type = 'Gdm')
order by (CASE WHEN $4 = 'f' THEN last_id.last_version_id END), last_id.last_version_id DESC
LIMIT $5
