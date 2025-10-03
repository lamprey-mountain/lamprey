with
    thread_viewer as (
        select thread.id, thread.room_id from thread
        join room_member on thread.room_id = room_member.room_id
        where room_member.user_id = $1
        union
        select thread.id, thread.room_id from thread
        join thread_member on thread.id = thread_member.thread_id
        where thread.room_id is null and thread_member.user_id = $1 and thread_member.membership = 'Join'
    )
select count(*)
from message as msg
join thread_viewer on msg.thread_id = thread_viewer.id
where is_latest and msg.deleted_at is null
  and content @@ websearch_to_tsquery($2)
  and (array_length($3::uuid[], 1) is null or thread_viewer.room_id = any($3))
  and (array_length($4::uuid[], 1) is null or msg.thread_id = any($4))
  and (array_length($5::uuid[], 1) is null or msg.author_id = any($5))