with last_id as (
    select thread_id, max(version_id) as last_version_id from message group by thread_id
)
select
    thread.*,
    usr.id as user_id,
    count::int as message_count,
    last_version_id,
    unread.version_id as last_read_id,
    coalesce(last_version_id != unread.version_id, true) as is_unread
from thread
join message_count on message_count.thread_id = thread.id
join last_id on last_id.thread_id = thread.id
full outer join usr on true
left join unread on usr.id = unread.user_id and thread.id = unread.thread_id
where usr.id = $1 and thread.id = $2
