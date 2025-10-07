with
    thread_viewer as (
        select thread.id from thread
        join room_member on thread.room_id = room_member.room_id
        where room_member.user_id = $1
        union
        select thread.id from thread
        join thread_member on thread.id = thread_member.thread_id
        where thread.room_id is null and thread_member.user_id = $1 and thread_member.membership = 'Join'
    )
select count(*)
from thread
join thread_viewer on thread.id = thread_viewer.id
where thread.deleted_at is null and thread.archived_at is null
  and (
    $2::text is null or $2 = '' or
    thread.name @@ websearch_to_tsquery('english', $2) or
    coalesce(thread.description, '') @@ websearch_to_tsquery('english', $2)
  )
  and (cardinality($3::uuid[]) = 0 or thread.room_id = any($3))
  and (cardinality($4::uuid[]) = 0 or thread.parent_id = any($4))