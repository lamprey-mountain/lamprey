DROP VIEW url_embed_json;

ALTER TABLE url_embed DROP COLUMN media_is_thumbnail;

CREATE VIEW url_embed_json AS
    WITH emb AS (
        SELECT
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
        FROM url_embed u
        LEFT JOIN media_json m ON m.id = u.media
        LEFT JOIN media_json t ON t.id = u.thumbnail
        LEFT JOIN media_json a ON a.id = u.author_avatar
        LEFT JOIN media_json s ON s.id = u.site_avatar)
    SELECT version_id, array_agg(row_to_json(emb) ORDER BY ordering) as embeds
    FROM url_embed_message u
    JOIN emb ON emb.id = u.embed_id
    GROUP BY version_id;