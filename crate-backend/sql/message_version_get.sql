with
    message_reaction as (
        -- select message_id, key as emoji, count(*), bool_or(user_id = $123) as self
        select
            message_id,
            json_agg((select row_to_json(j) from (select key as emoji, count(*) as count) j)) as json
        from reaction
        group by message_id, key
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
    false as "is_pinned!",
    coalesce(att_json.attachments, '{}') as "attachments!",
    coalesce(u.embeds, '{}') as "embeds!",
    r.json as "reactions"
from message as msg
left join att_json on att_json.version_id = msg.version_id
left join url_embed_json u on u.version_id = msg.version_id
left join message_reaction r on r.message_id = msg.id
where thread_id = $1 and msg.version_id = $2 and msg.deleted_at is null
