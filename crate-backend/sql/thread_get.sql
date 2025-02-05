-- FIXME: doesn't work if usr.id = null
with last_id as (
    select thread_id, max(version_id) as last_version_id from message group by thread_id
), message_coalesced AS (
    select *
    from (select *, row_number() over(partition by id order by version_id desc) as row_num
        from message)
    where row_num = 1
),
message_count as (
    select thread_id, count(*) as count
    from message_coalesced
    group by thread_id
)
select
    thread.id,
    thread.room_id,
    thread.creator_id,
    thread.version_id,
    thread.name,
    thread.description,
    thread.state as "state: DbThreadState",
    coalesce(count, 0) as "message_count!",
    last_version_id as "last_version_id!",
    unread.version_id as "last_read_id?",
    coalesce(last_version_id != unread.version_id, true) as "is_unread!"
from thread
join message_count on message_count.thread_id = thread.id
join last_id on last_id.thread_id = thread.id
full outer join usr on true
left join unread on usr.id = unread.user_id and thread.id = unread.thread_id
where thread.id = $1 and usr.id = $2
