create view att_json as (
    select
        m.version_id,
        array_agg(row_to_json(media_json) order by a.ordering) as attachments
    from message m
    join message_attachment a on a.version_id = m.version_id
    join media_json on a.media_id = media_json.id
    group by m.version_id
);
