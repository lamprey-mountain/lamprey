CREATE VIEW message_coalesced AS
    SELECT *
    FROM (SELECT *, ROW_NUMBER() OVER(PARTITION BY id ORDER BY version_id DESC) AS row_num
        FROM message)
    WHERE row_num = 1;
