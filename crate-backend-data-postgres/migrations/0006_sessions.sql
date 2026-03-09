CREATE TYPE session_status AS ENUM ('Unauthorized', 'Authorized', 'Sudo');

CREATE TABLE session (
    id UUID PRIMARY KEY,
    user_id UUID NOT NULL,
    token TEXT NOT NULL,
    name TEXT,
    status session_status NOT NULL,
    FOREIGN KEY (user_id) REFERENCES usr(id)
);

CREATE INDEX session_token ON session (token);
