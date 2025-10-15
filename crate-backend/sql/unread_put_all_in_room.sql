WITH last_messages AS (
    SELECT DISTINCT ON (thread_id)
        thread_id,
        id AS message_id,
        version_id
    FROM message
    WHERE thread_id IN (
        SELECT id FROM thread WHERE room_id = $2 AND deleted_at IS NULL AND archived_at IS NULL
    ) AND deleted_at IS NULL
    ORDER BY thread_id, id DESC
),
updated_unreads AS (
    INSERT INTO unread (user_id, thread_id, message_id, version_id)
    SELECT
        $1,
        lm.thread_id,
        lm.message_id,
        lm.version_id
    FROM last_messages lm
    ON CONFLICT (user_id, thread_id) DO UPDATE SET
        message_id = excluded.message_id,
        version_id = excluded.version_id
    RETURNING thread_id, message_id, version_id
)
SELECT thread_id, message_id, version_id FROM updated_unreads
