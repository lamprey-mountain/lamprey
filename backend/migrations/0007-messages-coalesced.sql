CREATE OR REPLACE VIEW messages_coalesced AS
    SELECT *
    FROM (SELECT *, ROW_NUMBER() OVER(PARTITION BY id ORDER BY version_id DESC) AS row_num
        FROM messages)
    WHERE row_num = 1;
