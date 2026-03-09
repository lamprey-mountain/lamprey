CREATE VIEW message_count AS (
    WITH counts AS (
        SELECT thread_id, count(*) as count
        FROM message_coalesced
        GROUP BY thread_id
    )
    SELECT thread.id AS thread_id, coalesce(count, 0) AS count
    FROM thread LEFT JOIN counts ON thread_id = thread.id
);
