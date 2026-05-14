SELECT count(*)
FROM redex_version
WHERE script_id = $1 AND channel_id = $2 AND deleted_at IS NULL
