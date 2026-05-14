SELECT
    version_id, script_id, channel_id, creator_id, created_at, deleted_at, data,
    cached_inputs, status
FROM redex_version
WHERE script_id = $1 AND channel_id = $2 AND version_id = $3 AND deleted_at IS NULL
