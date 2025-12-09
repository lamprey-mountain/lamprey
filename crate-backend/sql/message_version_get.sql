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
where channel_id = $1 and msg.version_id = $2 and msg.deleted_at is null
