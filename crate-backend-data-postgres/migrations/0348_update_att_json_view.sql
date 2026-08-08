CREATE OR REPLACE VIEW att_json AS
SELECT
    ma.version_id,
    json_agg(jsonb_build_object(
        'media', m.data,
        'spoiler', ma.spoiler
    )) AS attachments
FROM message_attachment ma
JOIN media m ON ma.media_id = m.id
GROUP BY ma.version_id;
