with
    message_coalesced as (
        select *
        from (select *, row_number() over(partition by id order by version_id desc) as row_num
            from message)
        where row_num = 1
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
    coalesce(att_json.attachments, '{}') as "attachments!",
    false as "is_pinned!",
    coalesce(u.embeds, '{}') as "embeds!"
from message_coalesced as msg
left join att_json on att_json.version_id = msg.version_id
left join url_embed_json u on u.version_id = msg.version_id
where thread_id = $1 and msg.deleted_at is null
  and msg.id > $2 AND msg.id < $3
order by (CASE WHEN $4 = 'f' THEN msg.id END), msg.id DESC LIMIT $5
