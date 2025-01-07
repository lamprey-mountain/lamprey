CREATE TABLE IF NOT EXISTS session (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    token TEXT NOT NULL,
    name TEXT,
    status INT NOT NULL,
    FOREIGN KEY (user_id) REFERENCES usr(id)
);

CREATE INDEX IF NOT EXISTS session_token ON session (token);
