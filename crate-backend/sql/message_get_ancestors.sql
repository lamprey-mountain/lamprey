WITH RECURSIVE parents AS (
    SELECT m.id, mv.reply_id, 0 as level
    FROM message m
    JOIN message_version mv ON m.latest_version_id = mv.version_id
    WHERE m.id = $1
    UNION ALL
    SELECT m.id, mv.reply_id, p.level + 1
    FROM message m
    JOIN message_version mv ON m.latest_version_id = mv.version_id
    JOIN parents p ON m.id = p.reply_id
    WHERE p.level < $2
)
SELECT
    mv.type as "message_type: DbMessageType",
    m.id,
    m.channel_id,
    m.author_id,
    m.created_at,
    m.deleted_at,
    m.removed_at,
    m.pinned,
    mv.version_id,
    mv.author_id as version_author_id,
    mv.content,
    mv.metadata,
    mv.reply_id,
    mv.override_name,
    mv.embeds as "embeds",
    mv.created_at as version_created_at,
    mv.deleted_at as version_deleted_at,
    coalesce(att_json.attachments, '{}') as "attachments!"
FROM message AS m
JOIN parents p ON m.id = p.id
JOIN message_version AS mv ON m.latest_version_id = mv.version_id
LEFT JOIN att_json ON att_json.version_id = mv.version_id
WHERE m.deleted_at IS NULL
  AND m.id != $1
ORDER BY p.level ASC