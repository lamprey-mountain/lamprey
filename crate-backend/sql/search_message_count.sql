with
    thread_viewer as (
        select thread.id from thread
        join room_member on thread.room_id = room_member.room_id
        where room_member.user_id = $1
    )
select count(*) -- unsure about the performance?
from message as msg
where is_latest and msg.deleted_at is null
  and content @@ websearch_to_tsquery($2)
