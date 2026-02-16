with
    channel_viewer as (
        select channel.id, channel.room_id from channel
        where channel.id = any($21)
        union
        select channel.id, channel.room_id from channel
        where channel.parent_id = any($22)
        union
        select channel.id, channel.room_id from channel
        join thread_member on channel.id = thread_member.channel_id
        where channel.room_id is null and thread_member.user_id = $1 and thread_member.membership = 'Join'
    )
select
    mv.type as "message_type: DbMessageType",
    m.id,
    m.channel_id,
    mv.version_id,
    mv.content,
    mv.metadata,
    mv.reply_id,
    mv.override_name,
    m.author_id,
    m.created_at,
    m.deleted_at,
    m.removed_at,
    m.pinned,
    coalesce(att_json.attachments, '[]'::json) as "attachments!",
    mv.embeds as "embeds",
    mv.author_id as version_author_id,
    mv.created_at as version_created_at,
    mv.deleted_at as version_deleted_at
from message as m
join message_version mv on m.latest_version_id = mv.version_id
join channel_viewer on m.channel_id = channel_viewer.id
left join att_json on att_json.version_id = mv.version_id
where m.deleted_at is null
  and m.id > $2 AND m.id < $3
  and ($6::text is null or $6 = '' or mv.content @@ websearch_to_tsquery('english', $6))
  and (cardinality($7::uuid[]) = 0 or channel_viewer.room_id = any($7))
  and (cardinality($8::uuid[]) = 0 or m.channel_id = any($8))
  and (cardinality($9::uuid[]) = 0 or m.author_id = any($9))
  -- has_attachment: $10
  and ($10::boolean is null or (exists (select 1 from message_attachment where version_id = mv.version_id)) = $10)
  -- has_image: $11
  and ($11::boolean is null or (exists (select 1 from message_attachment ma join media m2 on ma.media_id = m2.id, jsonb_array_elements(m2.data->'tracks') as track where ma.version_id = mv.version_id and track->>'mime' like 'image/%')) = $11)
  -- has_audio: $12
  and ($12::boolean is null or (exists (select 1 from message_attachment ma join media m2 on ma.media_id = m2.id, jsonb_array_elements(m2.data->'tracks') as track where ma.version_id = mv.version_id and track->>'mime' like 'audio/%')) = $12)
  -- has_video: $13
  and ($13::boolean is null or (exists (select 1 from message_attachment ma join media m2 on ma.media_id = m2.id, jsonb_array_elements(m2.data->'tracks') as track where ma.version_id = mv.version_id and track->>'mime' like 'video/%')) = $13)
  -- has_link: $14
  and ($14::boolean is null or ((mv.content ~ 'https?://[^\s<>"]+|www\.[^\s<>"]+') = $14))
  -- has_embed: $15
  and ($15::boolean is null or ((jsonb_array_length(coalesce(mv.embeds, '[]'::jsonb)) > 0) = $15))
  -- pinned: $16
  and ($16::boolean is null or ((m.pinned is not null) = $16))
  -- link_hostnames: $17
  and (cardinality($17::text[]) = 0 or embed_hosts(mv.embeds) && $17 or content_hosts(mv.content) && $17)
  -- mentions_users: $18
  and (cardinality($18::uuid[]) = 0 or (mv.mentions->'users')::jsonb ?| array(select jsonb_array_elements_text(to_jsonb($18::uuid[]))))
  -- mentions_roles: $19
  and (cardinality($19::uuid[]) = 0 or (mv.mentions->'roles')::jsonb ?| array(select jsonb_array_elements_text(to_jsonb($19::uuid[]))))
  -- mentions_everyone_room: $20
  and ($20::boolean is null or (mv.mentions->>'everyone')::boolean = $20)
order by (CASE WHEN $4 = 'f' THEN m.id END), m.id DESC LIMIT $5
