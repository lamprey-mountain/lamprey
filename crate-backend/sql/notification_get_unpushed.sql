SELECT
    id,
    user_id as "user_id!",
    room_id,
    channel_id as "channel_id!",
    message_id as "message_id!",
    type as "ty!: DbNotificationType",
    added_at as "added_at!",
    read_at
FROM inbox
WHERE pushed_at IS NULL
    AND read_at IS NULL
ORDER BY added_at ASC
LIMIT $1;
