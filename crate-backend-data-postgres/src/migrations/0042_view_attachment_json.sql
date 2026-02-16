create view att_json as (
    select version_id, json_agg(row_to_json(media) order by ord) as attachments
    from message, unnest(message.attachments) with ordinality as att(id, ord)
    join media on att.id = media.id
    group by message.version_id
);
