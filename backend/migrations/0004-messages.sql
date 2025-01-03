CREATE TABLE IF NOT EXISTS messages (
    id UUID NOT NULL,
    thread_id UUID NOT NULL,
    version_id UUID PRIMARY KEY,
    -- order_id UUID NOT NULL,
    ordering INT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB NOT NULL,
    reply_id UUID,
    author_id UUID NOT NULL,
    FOREIGN KEY (thread_id) REFERENCES threads(id),
    FOREIGN KEY (author_id) REFERENCES users(id)
    -- FOREIGN KEY (reply) REFERENCES messages(id)
);
