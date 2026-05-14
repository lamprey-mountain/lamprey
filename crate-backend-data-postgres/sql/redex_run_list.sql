SELECT id, script_id, redex_version_id, created_at, stopped_at, status, input FROM redex_eval
WHERE script_id = $1
  AND id > $2 AND id < $3
ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC
LIMIT $5
