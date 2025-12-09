with
    channel_viewer as (
        select channel.id from channel
        where channel.id = any($7)
        union
        select channel.id from channel
        where channel.parent_id = any($8)
        union
        select channel.id from channel
        join thread_member on channel.id = thread_member.channel_id
        where channel.room_id is null and thread_member.user_id = $1 and thread_member.membership = 'Join'
    )
select count(*)
from channel
join channel_viewer on channel.id = channel_viewer.id
where ($5::boolean is null or (channel.archived_at is not null) = $5)
  and ($6::boolean is null or (channel.deleted_at is not null) = $6)
  and (
    $2::text is null or $2 = '' or
    channel.name @@ websearch_to_tsquery('english', $2) or
    coalesce(channel.description, '') @@ websearch_to_tsquery('english', $2)
  )
  and (cardinality($3::uuid[]) = 0 or channel.room_id = any($3))
  and (cardinality($4::uuid[]) = 0 or channel.parent_id = any($4))
  and (cardinality($9::text[]) = 0 or channel.type::text = any($9))
  and (cardinality($10::uuid[]) = 0 OR EXISTS (SELECT 1 FROM channel_tag tt WHERE tt.channel_id = channel.id AND tt.tag_id = ANY($10)))
