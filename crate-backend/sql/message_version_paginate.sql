with
    att_json as (
        select version_id, json_agg(row_to_json(media) order by ord) as attachments
        from message, unnest(message.attachments) with ordinality as att(id, ord)
        join media on att.id = media.id
        group by message.version_id
    )
select
    msg.type as "message_type: DbMessageType",
    msg.id,
    msg.thread_id, 
    msg.version_id,
    msg.ordering,
    msg.content,
    msg.metadata,
    msg.reply_id,
    msg.override_name,
    row_to_json(usr) as "author!: serde_json::Value",
    coalesce(att_json.attachments, '[]'::json) as "attachments!: serde_json::Value",
    false as "is_pinned!"
from message as msg
join usr on usr.id = msg.author_id
left join att_json on att_json.version_id = msg.version_id
where thread_id = $1 and msg.id = $2 and msg.deleted_at is null
  and msg.id > $3 AND msg.id < $4
order by (CASE WHEN $5 = 'f' THEN msg.version_id END), msg.version_id DESC LIMIT $6

