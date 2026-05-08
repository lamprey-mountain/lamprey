SELECT
    version_id, script_id, channel_id, creator_id, created_at, deleted_at, data,
    cached_inputs, status
FROM script_version
WHERE script_id = $1 AND version_id = $2 AND deleted_at IS NULL
