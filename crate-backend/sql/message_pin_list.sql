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
    coalesce(att_json.attachments, '{}') as "attachments!",
    msg.embeds as "embeds"
from message as msg
left join att_json on att_json.version_id = msg.version_id
left join hydrated_mentions hm on hm.message_id = msg.id
where is_latest and channel_id = $1 and msg.deleted_at is null and msg.pinned is not null
  and msg.id > $2 AND msg.id < $3
order by (CASE WHEN $4 = 'f' THEN (msg.pinned->>'position')::int END), (msg.pinned->>'position')::int DESC, (CASE WHEN $4 = 'f' THEN msg.id END), msg.id DESC
LIMIT $5
