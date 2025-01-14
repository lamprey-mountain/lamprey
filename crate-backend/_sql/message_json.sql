with
att_unnest as (select version_id, unnest(attachments) as media_id from message),
att_json as (
    select version_id, json_agg(row_to_json(media)) as attachments
    from att_unnest
    join media on att_unnest.media_id = media.id
    group by att_unnest.version_id
),
message_coalesced as (
    select *
    from (select *, row_number() over(partition by id order by version_id desc) as row_num
        from message)
    where row_num = 1;
)
SELECT msg.type, msg.id, msg.thread_id, msg.version_id, msg.ordering, msg.content, msg.metadata, msg.reply_id, msg.override_name,
    msg.deleted_at,
    row_to_json(usr) as author, coalesce(att_json.attachments, '[]'::json) as attachments FROM message_coalesced AS msg
JOIN usr ON usr.id = msg.author_id
left JOIN att_json ON att_json.version_id = msg.version_id
