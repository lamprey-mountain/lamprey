with
    message_coalesced as (
        select *
        from (select *, row_number() over(partition by id order by version_id desc) as row_num
            from message)
        where row_num = 1
    ),
    thread_viewer as (
        select thread.id from thread
        join room_member on thread.room_id = room_member.room_id
        where room_member.user_id = $1
    )
select
    count(*) -- unsure about the performance?
from message_coalesced as msg
where msg.deleted_at is null
  and content @@ websearch_to_tsquery($2)
