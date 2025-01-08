-- ugly ahh sql
create or replace view message_json as (
    with
    att_unnest as (select id as message_id, unnest(attachments) as media_id from message),
    att_json as (
        select message_id, json_agg(row_to_json(media)) as attachments
        from att_unnest
        join media on att_unnest.media_id = media.id
        group by att_unnest.message_id
    )
    SELECT msg.type, msg.id, msg.thread_id, msg.version_id, msg.ordering, msg.content, msg.metadata, msg.reply_id, msg.override_name,
        row_to_json(usr) as author, coalesce(att_json.attachments, '[]'::json) as attachments FROM message_coalesced AS msg
    JOIN usr ON usr.id = msg.author_id
    left JOIN att_json ON att_json.message_id = msg.id
);
