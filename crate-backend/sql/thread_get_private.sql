with last_id as (
    select thread_id, max(version_id) as last_version_id
    from message
    where deleted_at is null
    group by thread_id
)
select
    thread.id,
    thread.type as "ty: DbThreadType",
    u.message_id as "last_read_id?",
    coalesce(u.version_id < last_version_id, true) as "is_unread!"
from thread
left join last_id on last_id.thread_id = thread.id
left join unread u on u.thread_id = thread.id and u.user_id = $2
where thread.id = $1
