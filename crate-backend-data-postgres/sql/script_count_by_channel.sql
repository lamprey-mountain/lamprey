SELECT count(*)
FROM script s
JOIN LATERAL (
    SELECT 1
    FROM script_version
    WHERE script_id = s.id AND deleted_at IS NULL
    LIMIT 1
) sv ON true
WHERE s.channel_id = $1 AND s.deleted_at IS NULL
