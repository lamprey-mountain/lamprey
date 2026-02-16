drop view url_embed_json;

create view url_embed_json as
    with emb as (
        select
            u.id,
            u.url,
            u.canonical_url,
            u.title,
            u.description,
            u.color,
            m.data as media,
            t.data as thumbnail,
            u.author_url,
            u.author_name,
            a.data as author_avatar,
            u.site_name,
            s.data as site_avatar
        from url_embed u
        left join media m on m.id = u.media
        left join media t on m.id = u.thumbnail
        left join media a on a.id = u.author_avatar
        left join media s on s.id = u.site_avatar)
    select version_id, array_agg(row_to_json(emb) order by ordering) as embeds
    from url_embed_message u
    join emb on emb.id = u.embed_id
    group by version_id;
