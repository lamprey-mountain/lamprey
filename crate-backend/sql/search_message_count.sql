with
    channel_viewer as (
        select channel.id, channel.room_id from channel
        where channel.id = any($17)
        union
        select channel.id, channel.room_id from channel
        where channel.parent_id = any($18)
        union
        select channel.id, channel.room_id from channel
        join thread_member on channel.id = thread_member.channel_id
        where channel.room_id is null and thread_member.user_id = $1 and thread_member.membership = 'Join'
    )
select count(*)
from message as msg
join channel_viewer on msg.channel_id = channel_viewer.id
where is_latest and msg.deleted_at is null
  and ($2::text is null or $2 = '' or content @@ websearch_to_tsquery('english', $2))
  and (cardinality($3::uuid[]) = 0 or channel_viewer.room_id = any($3))
  and (cardinality($4::uuid[]) = 0 or msg.channel_id = any($4))
  and (cardinality($5::uuid[]) = 0 or msg.author_id = any($5))
  -- has_attachment: $6
  and ($6::boolean is null or (exists (select 1 from message_attachment where version_id = msg.version_id)) = $6)
  -- has_image: $7
  and ($7::boolean is null or (exists (select 1 from message_attachment ma join media m on ma.media_id = m.id, jsonb_array_elements(m.data->'tracks') as track where ma.version_id = msg.version_id and track->>'mime' like 'image/%')) = $7)
  -- has_audio: $8
  and ($8::boolean is null or (exists (select 1 from message_attachment ma join media m on ma.media_id = m.id, jsonb_array_elements(m.data->'tracks') as track where ma.version_id = msg.version_id and track->>'mime' like 'audio/%')) = $8)
  -- has_video: $9
  and ($9::boolean is null or (exists (select 1 from message_attachment ma join media m on ma.media_id = m.id, jsonb_array_elements(m.data->'tracks') as track where ma.version_id = msg.version_id and track->>'mime' like 'video/%')) = $9)
  -- has_link: $10
  and ($10::boolean is null or ((content ~ 'https?://[^\s<>"]+|www\.[^\s<>"]+') = $10))
  -- has_embed: $11
  and ($11::boolean is null or ((jsonb_array_length(coalesce(msg.embeds, '[]'::jsonb)) > 0) = $11))
  -- pinned: $12
  and ($12::boolean is null or ((msg.pinned is not null) = $12))
  -- link_hostnames: $13
  and (cardinality($13::text[]) = 0 or embed_hosts(msg.embeds) && $13 or content_hosts(msg.content) && $13)
  -- mentions_users: $14
  and (cardinality($14::uuid[]) = 0 or (msg.mentions->'users')::jsonb ?| array(select jsonb_array_elements_text(to_jsonb($14::uuid[]))))
  -- mentions_roles: $15
  and (cardinality($15::uuid[]) = 0 or (msg.mentions->'roles')::jsonb ?| array(select jsonb_array_elements_text(to_jsonb($15::uuid[]))))
  -- mentions_everyone: $16
  and ($16::boolean is null or (msg.mentions->>'everyone')::boolean = $16)
