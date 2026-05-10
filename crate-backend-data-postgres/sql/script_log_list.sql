SELECT * FROM script_log
WHERE run_id = $1
  AND line_id > (CASE WHEN $2 = 0 AND $4 = 'f' THEN -1 ELSE $2 END)
  AND line_id < $3
ORDER BY (CASE WHEN $4 = 'f' THEN line_id END), line_id DESC
LIMIT $5
