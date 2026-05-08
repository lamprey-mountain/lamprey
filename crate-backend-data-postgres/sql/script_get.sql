SELECT
    s.id, s.channel_id, s.creator_id, s.created_at, s.deleted_at, s.data,
    sv.version_id, sv.creator_id AS version_creator_id, sv.created_at AS version_created_at,
    sv.deleted_at AS version_deleted_at, sv.data AS version_data, sv.cached_inputs,
    sv.status AS version_status
FROM script s
JOIN LATERAL (
    SELECT version_id, creator_id, created_at, deleted_at, data, cached_inputs, status
    FROM script_version
    WHERE script_id = s.id AND deleted_at IS NULL
    ORDER BY created_at DESC
    LIMIT 1
) sv ON true
WHERE s.id = $1 AND s.deleted_at IS NULL
