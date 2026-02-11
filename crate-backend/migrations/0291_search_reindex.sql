CREATE TABLE search_reindex_queue (
    channel_id UUID PRIMARY KEY REFERENCES channel(id) ON DELETE CASCADE,
    last_message_id UUID,
    updated_at TIMESTAMP NOT NULL DEFAULT NOW()
);
