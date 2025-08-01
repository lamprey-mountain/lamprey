with last_id as (
    select thread_id, max(version_id) as last_version_id
    from message
    where deleted_at is null
    group by thread_id
)
select
    thread.id,
    thread.type as "ty: DbThreadType",
    unread.message_id as "last_read_id?",
    coalesce(unread.version_id < last_version_id, true) as "is_unread!"
from thread
left join last_id on last_id.thread_id = thread.id
full outer join usr on true
left join unread on usr.id = unread.user_id and thread.id = unread.thread_id
where thread.id = $1 and usr.id = $2
