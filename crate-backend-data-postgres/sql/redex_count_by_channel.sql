SELECT count(*)
FROM redex r
JOIN LATERAL (
    SELECT 1
    FROM redex_version
    WHERE script_id = r.id AND deleted_at IS NULL
    LIMIT 1
) sv ON true
WHERE r.channel_id = $1 AND r.deleted_at IS NULL
