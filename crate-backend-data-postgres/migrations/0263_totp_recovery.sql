CREATE TABLE totp_recovery (
    user_id UUID NOT NULL REFERENCES usr(id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    used_at TIMESTAMP,
    PRIMARY KEY (user_id, code)
);
