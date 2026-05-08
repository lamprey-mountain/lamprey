SELECT
    version_id, script_id, channel_id, creator_id, created_at, deleted_at, data,
    cached_inputs, status
FROM script_version
WHERE script_id = $1 AND channel_id = $2 AND deleted_at IS NULL
  AND version_id > $3 AND version_id < $4
ORDER BY (CASE WHEN $5 = 'f' THEN version_id END), version_id DESC LIMIT $6
