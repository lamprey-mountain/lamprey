SELECT * FROM script_run
WHERE script_id = $1
  AND id > $2 AND id < $3
ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC
LIMIT $5
