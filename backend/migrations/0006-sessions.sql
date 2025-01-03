CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    token TEXT NOT NULL,
    name TEXT,
    status INT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
