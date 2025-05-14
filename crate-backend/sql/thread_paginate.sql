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
)
select
    thread.id,
    thread.room_id,
    thread.creator_id,
    thread.name,
    thread.version_id,
    thread.description,
    thread.state as "state: DbThreadState",
    coalesce(count, 0) as "message_count!",
    last_version_id as "last_version_id!",
    unread.message_id as "last_read_id?",
    coalesce(unread.version_id < last_version_id, true) as "is_unread!"
from thread
join message_count on message_count.thread_id = thread.id
join last_id on last_id.thread_id = thread.id
full outer join usr on true
left join unread on usr.id = unread.user_id and thread.id = unread.thread_id
where room_id = $1 AND usr.id = $2 AND thread.id > $3 AND thread.id < $4
order by (CASE WHEN $5 = 'f' THEN thread.id END), thread.id DESC LIMIT $6
