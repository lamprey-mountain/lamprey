with last_id as (
    select thread_id, max(version_id) as last_version_id
    from message
    where deleted_at is null
    group by thread_id
),
other as (
    select user_id as recipient_id from thread_member
    where thread_id = $1 and user_id <> $2
)
select
    thread.id,
    thread.type as "ty: DbThreadType",
    unread.message_id as "last_read_id?",
    coalesce(unread.version_id < last_version_id, true) as "is_unread!",
    other.recipient_id
from thread
left join last_id on last_id.thread_id = thread.id
full outer join usr on true
left join unread on usr.id = unread.user_id and thread.id = unread.thread_id
left join other on true
where thread.id = $1 and usr.id = $2
