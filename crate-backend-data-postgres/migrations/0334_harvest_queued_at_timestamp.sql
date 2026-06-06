ALTER TABLE harvest ALTER COLUMN queued_at TYPE TIMESTAMP WITHOUT TIME ZONE USING to_timestamp(queued_at)::timestamp;
