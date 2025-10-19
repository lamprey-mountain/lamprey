SELECT count(*)
FROM inbox
WHERE user_id = $1
  AND ($2 OR read_at IS NULL)
  AND (array_length($3::uuid[], 1) IS NULL OR room_id = ANY($3))
  AND (array_length($4::uuid[], 1) IS NULL OR channel_id = ANY($4))
