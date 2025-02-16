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
    coalesce(att_json.attachments, '{}') as "attachments!",
    false as "is_pinned!"
from message as msg
join usr on usr.id = msg.author_id
left join att_json on att_json.version_id = msg.version_id
where thread_id = $1 and msg.id = $2 and msg.deleted_at is null
  and msg.id > $3 and msg.id < $4
order by (case when $5 = 'f' then msg.version_id end), msg.version_id desc limit $6

