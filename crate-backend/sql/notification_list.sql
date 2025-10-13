SELECT id, thread_id as "thread_id!", message_id as "message_id!", reason as "reason!", added_at, read_at
FROM inbox
WHERE user_id = $1
  AND ($2 OR read_at IS NULL)
  AND (array_length($3::uuid[], 1) IS NULL OR room_id = ANY($3))
  AND (array_length($4::uuid[], 1) IS NULL OR thread_id = ANY($4))
  AND id > $5 AND id < $6
ORDER BY (CASE WHEN $7 = 'f' THEN id END), id DESC
LIMIT $8
