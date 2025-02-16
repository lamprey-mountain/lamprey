drop view att_json;

create view media_json as (
    select
        m.id, m.filename, m.alt,
        json_agg(row_to_json(t)) as tracks
    from media m
    join media_track t on t.media_id = m.id
    group by m.id
);

create view att_json as (
    select
        version_id,
        array_agg(row_to_json(media_json) order by ord) as attachments
    from
        message,
        unnest(message.attachments) with ordinality as att(id, ord)
    join media_json on att.id = media_json.id
    group by message.version_id
);
