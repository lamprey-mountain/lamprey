with
message_coalesced as (
    select *
    from (select *, row_number() over(partition by id order by version_id desc) as row_num
        from message)
    where row_num = 1
),
embeds as (
    select version_id, array_agg(row_to_json(u)) as embeds
    from url_embed_message u
    group by version_id
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
    coalesce(att_json.attachments, '{}') as "attachments!",
    coalesce(embeds.embeds, '{}') as "embeds!"
FROM message_coalesced AS msg
JOIN usr ON usr.id = msg.author_id
left join embeds on embeds.version_id = msg.version_id
left JOIN att_json ON att_json.version_id = msg.version_id
     WHERE thread_id = $1 AND msg.id = $2 AND msg.deleted_at IS NULL
