WITH RECURSIVE parents AS (
    SELECT id, reply_id, 0 as level
    FROM message
    WHERE id = $1 AND is_latest
    UNION ALL
    SELECT m.id, m.reply_id, p.level + 1
    FROM message m
    JOIN parents p ON m.id = p.reply_id
    WHERE m.is_latest AND p.level < $2
)
SELECT
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
FROM message as msg
JOIN parents p ON msg.id = p.id
LEFT JOIN att_json ON att_json.version_id = msg.version_id
LEFT JOIN hydrated_mentions hm ON hm.message_id = msg.id
WHERE msg.is_latest
  AND msg.deleted_at IS NULL
  AND msg.id != $1
ORDER BY p.level ASC