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
from message as m
join message_version mv on m.latest_version_id = mv.version_id
join channel_viewer on m.channel_id = channel_viewer.id
where m.deleted_at is null
  and ($2::text is null or $2 = '' or mv.content @@ websearch_to_tsquery('english', $2))
  and (cardinality($3::uuid[]) = 0 or channel_viewer.room_id = any($3))
  and (cardinality($4::uuid[]) = 0 or m.channel_id = any($4))
  and (cardinality($5::uuid[]) = 0 or m.author_id = any($5))
  -- has_attachment: $6
  and ($6::boolean is null or (exists (select 1 from message_attachment where version_id = mv.version_id)) = $6)
  -- has_image: $7
  and ($7::boolean is null or (exists (select 1 from message_attachment ma join media m2 on ma.media_id = m2.id, jsonb_array_elements(m2.data->'tracks') as track where ma.version_id = mv.version_id and track->>'mime' like 'image/%')) = $7)
  -- has_audio: $8
  and ($8::boolean is null or (exists (select 1 from message_attachment ma join media m2 on ma.media_id = m2.id, jsonb_array_elements(m2.data->'tracks') as track where ma.version_id = mv.version_id and track->>'mime' like 'audio/%')) = $8)
  -- has_video: $9
  and ($9::boolean is null or (exists (select 1 from message_attachment ma join media m2 on ma.media_id = m2.id, jsonb_array_elements(m2.data->'tracks') as track where ma.version_id = mv.version_id and track->>'mime' like 'video/%')) = $9)
  -- has_link: $10
  and ($10::boolean is null or ((mv.content ~ 'https?://[^\s<>"]+|www\.[^\s<>"]+') = $10))
  -- has_embed: $11
  and ($11::boolean is null or ((jsonb_array_length(coalesce(mv.embeds, '[]'::jsonb)) > 0) = $11))
  -- pinned: $12
  and ($12::boolean is null or ((m.pinned is not null) = $12))
  -- link_hostnames: $13
  and (cardinality($13::text[]) = 0 or embed_hosts(mv.embeds) && $13 or content_hosts(mv.content) && $13)
  -- mentions_users: $14
  and (cardinality($14::uuid[]) = 0 or (mv.mentions->'users')::jsonb ?| array(select jsonb_array_elements_text(to_jsonb($14::uuid[]))))
  -- mentions_roles: $15
  and (cardinality($15::uuid[]) = 0 or (mv.mentions->'roles')::jsonb ?| array(select jsonb_array_elements_text(to_jsonb($15::uuid[]))))
  -- mentions_everyone: $16
  and ($16::boolean is null or (mv.mentions->>'everyone')::boolean = $16)
