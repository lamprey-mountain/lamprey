with
    message_reaction as (
        -- select message_id, key, count(*), bool_or(user_id = $123) as self
        select
            message_id,
            json_agg((select row_to_json(j) from (select key, count(*) as count) j)) as json
        from reaction
        group by message_id
        order by min(position)
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
    msg.author_id,
    msg.created_at,
    msg.edited_at,
    msg.deleted_at,
    coalesce(att_json.attachments, '{}') as "attachments!",
    msg.embeds as "embeds",
    r.json as "reactions"
from message as msg
left join att_json on att_json.version_id = msg.version_id
left join message_reaction r on r.message_id = msg.id
where thread_id = $1 and msg.id = $2 and msg.deleted_at is null
  and msg.id > $3 and msg.id < $4
order by (case when $5 = 'f' then msg.version_id end), msg.version_id desc limit $6
