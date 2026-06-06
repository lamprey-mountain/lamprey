SELECT
    id,
    user_id as "user_id!",
    info,
    channel_id,
    room_id,
    added_at as "added_at!",
    read_at
FROM inbox
WHERE pushed_at IS NULL
    AND read_at IS NULL
ORDER BY added_at ASC
LIMIT $1;
