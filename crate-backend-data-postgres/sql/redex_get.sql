SELECT
    r.id, r.channel_id, r.creator_id, r.created_at, r.deleted_at, r.data,
    rv.version_id, rv.creator_id AS version_creator_id, rv.created_at AS version_created_at,
    rv.deleted_at AS version_deleted_at, rv.data AS version_data, rv.cached_inputs,
    rv.status AS version_status
FROM redex r
JOIN LATERAL (
    SELECT version_id, creator_id, created_at, deleted_at, data, cached_inputs, status
    FROM redex_version
    WHERE script_id = r.id AND deleted_at IS NULL
    ORDER BY created_at DESC
    LIMIT 1
) rv ON true
WHERE r.id = $1 AND r.deleted_at IS NULL
