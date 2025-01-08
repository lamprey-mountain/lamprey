CREATE TABLE IF NOT EXISTS message (
    type INT NOT NULL,
    id UUID NOT NULL,
    thread_id UUID NOT NULL,
    version_id UUID PRIMARY KEY,
    -- order_id UUID NOT NULL, -- instead of int..? or switch version_id to int?
    ordering INT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB,
    reply_id UUID,
    attachments UUID[] NOT NULL,
    -- embeds JSONB,
    author_id UUID NOT NULL,
    override_name TEXT,
    deleted_at INT,
    FOREIGN KEY (thread_id) REFERENCES thread(id),
    FOREIGN KEY (author_id) REFERENCES usr(id)
    -- FOREIGN KEY (reply) REFERENCES messages(id)
);
