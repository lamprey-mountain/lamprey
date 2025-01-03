CREATE TABLE sessions IF NOT EXISTS (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token TEXT NOT NULL,
    name TEXT,
    status INT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
