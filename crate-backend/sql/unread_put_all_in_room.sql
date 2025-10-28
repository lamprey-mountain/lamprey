WITH last_messages AS (
    SELECT DISTINCT ON (channel_id)
        channel_id,
        id AS message_id,
        version_id
    FROM message
    WHERE channel_id IN (
        SELECT id FROM channel WHERE room_id = $2 AND deleted_at IS NULL AND archived_at IS NULL
    ) AND deleted_at IS NULL
    ORDER BY channel_id, id DESC
),
updated_unreads AS (
    INSERT INTO unread (user_id, channel_id, message_id, version_id, mention_count)
    SELECT
        $1,
        lm.channel_id,
        lm.message_id,
        lm.version_id,
        0
    FROM last_messages lm
    ON CONFLICT (user_id, channel_id) DO UPDATE SET
        message_id = excluded.message_id,
        version_id = excluded.version_id,
        mention_count = 0
    RETURNING channel_id, message_id, version_id
)
SELECT channel_id, message_id, version_id FROM updated_unreads
