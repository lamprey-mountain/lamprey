with
    thread_viewer as (
        select thread.id from thread
        join room_member on thread.room_id = room_member.room_id
        where room_member.user_id = $1
        union
        select thread.id from thread
        join thread_member on thread.id = thread_member.thread_id
        where thread.room_id is null and thread_member.user_id = $1 and thread_member.membership = 'Join'
    ),
    last_id as (
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
    last_version_id as "last_version_id",
    coalesce(permission_overwrites.overwrites, '[]') as "permission_overwrites!"
from thread
join thread_viewer on thread.id = thread_viewer.id
left join message_count on message_count.thread_id = thread.id
left join member_count on member_count.thread_id = thread.id
left join last_id on last_id.thread_id = thread.id
left join permission_overwrites on permission_overwrites.target_id = thread.id
where ($9::boolean is null or (thread.archived_at is not null) = $9)
  and ($10::boolean is null or (thread.deleted_at is not null) = $10)
  and thread.id > $2 AND thread.id < $3
  and (
    $6::text is null or $6 = '' or
    thread.name @@ websearch_to_tsquery('english', $6) or
    coalesce(thread.description, '') @@ websearch_to_tsquery('english', $6)
  )
  and (cardinality($7::uuid[]) = 0 or thread.room_id = any($7))
  and (cardinality($8::uuid[]) = 0 or thread.parent_id = any($8))
order by (CASE WHEN $4 = 'f' THEN thread.id END), thread.id DESC LIMIT $5
