SELECT
    mv.version_id,
    mv.author_id,
    mv.type as "message_type: DbMessageType",
    mv.content,
    mv.metadata,
    mv.reply_id,
    mv.override_name,
    mv.embeds as "embeds",
    mv.created_at,
    mv.deleted_at,
    coalesce(att_json.attachments, '{}') as "attachments!"
FROM message_version AS mv
JOIN message AS m ON m.id = mv.message_id
LEFT JOIN att_json ON att_json.version_id = mv.version_id
WHERE m.channel_id = $1 AND mv.version_id = $2 AND m.deleted_at IS NULL
