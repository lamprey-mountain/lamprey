WITH last_messages AS (
    SELECT DISTINCT ON (m.channel_id)
        m.channel_id,
        m.id AS message_id
    FROM message m
    WHERE m.channel_id IN (
        SELECT id FROM channel WHERE room_id = $1 AND deleted_at IS NULL AND archived_at IS NULL
    ) AND m.deleted_at IS NULL
    ORDER BY m.channel_id, m.id DESC
)
SELECT channel_id, message_id FROM last_messages
