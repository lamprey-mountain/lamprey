UPDATE media
SET deleted_at = now()
WHERE
    NOT EXISTS (
        SELECT
            1
        FROM
            media_link
        WHERE
            media_link.media_id = media.id
    )
    AND extract_timestamp_from_uuid_v7(id) < now() - interval '7 day';
