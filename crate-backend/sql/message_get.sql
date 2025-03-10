with
message_coalesced as (
    select *
    from (select *, row_number() over(partition by id order by version_id desc) as row_num
        from message)
    where row_num = 1
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
    msg.author_id,
    false as "is_pinned!",
    coalesce(att_json.attachments, '{}') as "attachments!",
    coalesce(u.embeds, '{}') as "embeds!"
FROM message_coalesced AS msg
left join url_embed_json u on u.version_id = msg.version_id
left JOIN att_json ON att_json.version_id = msg.version_id
     WHERE thread_id = $1 AND msg.id = $2 AND msg.deleted_at IS NULL
