CREATE TABLE IF NOT EXISTS messages (
    id TEXT NOT NULL,
    thread_id TEXT NOT NULL,
    version_id TEXT PRIMARY KEY,
    ordering INT NOT NULL,
    content TEXT NOT NULL,
    metadata JSONB NOT NULL,
    reply_id TEXT,
    author_id TEXT NOT NULL,
    FOREIGN KEY (thread_id) REFERENCES threads(id),
    FOREIGN KEY (author_id) REFERENCES users(id)
    -- FOREIGN KEY (reply) REFERENCES messages(id)
);
