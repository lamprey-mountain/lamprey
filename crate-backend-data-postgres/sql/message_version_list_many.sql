SELECT
    mv.version_id,
    mv.author_id,
    mv.type as "message_type: DbMessageType",
    mv.content,
    mv.metadata,
    mv.reply_id,
    mv.override_name,
    mv.embeds as "embeds",
    mv.components as "components",
    mv.created_at,
    mv.deleted_at,
    mv.created_seq,
    coalesce(att_json.attachments, '{}') as "attachments!"
FROM message_version AS mv
JOIN message AS m ON m.id = mv.message_id
LEFT JOIN att_json ON att_json.version_id = mv.version_id
WHERE m.channel_id = $1
  AND mv.message_id = ANY($2)
  AND ($3::uuid IS NULL OR mv.version_id > $3)
ORDER BY mv.version_id
LIMIT $4
