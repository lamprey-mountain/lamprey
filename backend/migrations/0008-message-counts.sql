CREATE OR REPLACE VIEW messages_counts AS
    SELECT thread_id, count(*)
    FROM messages_coalesced
    GROUP BY thread_id;
