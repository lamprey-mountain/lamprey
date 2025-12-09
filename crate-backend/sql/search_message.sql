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
    msg.type as "message_type: DbMessageType",
    msg.id,
    msg.channel_id,
    msg.version_id,
    msg.ordering,
    msg.content,
    msg.metadata,
    msg.reply_id,
    msg.override_name,
    msg.author_id,
    msg.created_at,
    msg.edited_at,
    msg.deleted_at,
    msg.removed_at,
    msg.pinned,
    hm.mentions,
    coalesce(att_json.attachments, '[]'::json) as "attachments!",
    msg.embeds as "embeds"
from message as msg
join channel_viewer on msg.channel_id = channel_viewer.id
left join att_json on att_json.version_id = msg.version_id
left join hydrated_mentions hm on hm.message_id = msg.id
where is_latest and msg.deleted_at is null
  and msg.id > $2 AND msg.id < $3
  and ($6::text is null or $6 = '' or content @@ websearch_to_tsquery('english', $6))
  and (cardinality($7::uuid[]) = 0 or channel_viewer.room_id = any($7))
  and (cardinality($8::uuid[]) = 0 or msg.channel_id = any($8))
  and (cardinality($9::uuid[]) = 0 or msg.author_id = any($9))
  -- has_attachment: $10
  and ($10::boolean is null or (exists (select 1 from message_attachment where version_id = msg.version_id)) = $10)
  -- has_image: $11
  and ($11::boolean is null or (exists (select 1 from message_attachment ma join media m on ma.media_id = m.id, jsonb_array_elements(m.data->'tracks') as track where ma.version_id = msg.version_id and track->>'mime' like 'image/%')) = $11)
  -- has_audio: $12
  and ($12::boolean is null or (exists (select 1 from message_attachment ma join media m on ma.media_id = m.id, jsonb_array_elements(m.data->'tracks') as track where ma.version_id = msg.version_id and track->>'mime' like 'audio/%')) = $12)
  -- has_video: $13
  and ($13::boolean is null or (exists (select 1 from message_attachment ma join media m on ma.media_id = m.id, jsonb_array_elements(m.data->'tracks') as track where ma.version_id = msg.version_id and track->>'mime' like 'video/%')) = $13)
  -- has_link: $14
  and ($14::boolean is null or ((content ~ 'https?://[^\s<>"]+|www\.[^\s<>"]+') = $14))
  -- has_embed: $15
  and ($15::boolean is null or ((jsonb_array_length(coalesce(msg.embeds, '[]'::jsonb)) > 0) = $15))
  -- pinned: $16
  and ($16::boolean is null or ((msg.pinned is not null) = $16))
  -- link_hostnames: $17
  and (cardinality($17::text[]) = 0 or embed_hosts(msg.embeds) && $17 or content_hosts(msg.content) && $17)
  -- mentions_users: $18
  and (cardinality($18::uuid[]) = 0 or (msg.mentions->'users')::jsonb ?| array(select jsonb_array_elements_text(to_jsonb($18::uuid[]))))
  -- mentions_roles: $19
  and (cardinality($19::uuid[]) = 0 or (msg.mentions->'roles')::jsonb ?| array(select jsonb_array_elements_text(to_jsonb($19::uuid[]))))
  -- mentions_everyone_room: $20
  and ($20::boolean is null or (msg.mentions->>'everyone')::boolean = $20)
order by (CASE WHEN $4 = 'f' THEN msg.id END), msg.id DESC LIMIT $5
