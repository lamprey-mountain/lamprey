create view att_json as
select
    ma.version_id,
    json_agg(m.data) as attachments
from message_attachment ma
join media m on ma.media_id = m.id
group by ma.version_id;

create view url_embed_json as
    with emb as (
        select
            u.id,
            u.url,
            u.canonical_url,
            u.title,
            u.description,
            u.color,
            row_to_json(m) as media,
            row_to_json(t) as thumbnail,
            u.author_url,
            u.author_name,
            row_to_json(a) as author_avatar,
            u.site_name,
            row_to_json(s) as site_avatar
        from url_embed u
        left join media m on m.id = u.media
        left join media t on m.id = u.thumbnail
        left join media a on a.id = u.author_avatar
        left join media s on s.id = u.site_avatar)
    select version_id, array_agg(row_to_json(emb) order by ordering) as embeds
    from url_embed_message u
    join emb on emb.id = u.embed_id
    group by version_id;
