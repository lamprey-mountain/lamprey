ALTER TABLE channel ADD COLUMN last_message_id UUID;
ALTER TABLE channel ADD COLUMN last_version_id UUID;

UPDATE channel t
SET
    last_message_id = m.id,
    last_version_id = m.latest_version_id
FROM (
    SELECT id, latest_version_id, channel_id,
           ROW_NUMBER() OVER (PARTITION BY channel_id ORDER BY id DESC) as rn
    FROM message
) m
WHERE m.channel_id = t.id AND m.rn = 1;
