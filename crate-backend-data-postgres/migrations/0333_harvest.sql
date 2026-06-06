CREATE TABLE harvest (
    id TEXT PRIMARY KEY,
    target_id UUID NOT NULL,
    queued_at BIGINT NOT NULL,
    data JSONB NOT NULL
);

CREATE INDEX harvest_target_id ON harvest (target_id);
