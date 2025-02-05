with
att_json as (
    select version_id, json_agg(row_to_json(media) order by ord) as attachments
    from message, unnest(message.attachments) with ordinality as att(id, ord)
    join media on att.id = media.id
    group by message.version_id
)
SELECT
    msg.type as "message_type: DbMessageType",
    msg.id,
    msg.thread_id, 
    msg.version_id,
    msg.ordering,
    msg.content,
    msg.metadata,
    msg.reply_id,
    msg.override_name,
    false as "is_pinned!",
    row_to_json(usr) as "author!",
    coalesce(att_json.attachments, '[]'::json) as "attachments!"
FROM message AS msg
JOIN usr ON usr.id = msg.author_id
left JOIN att_json ON att_json.version_id = msg.version_id
     WHERE thread_id = $1 AND msg.version_id = $2 AND msg.deleted_at IS NULL
